use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new rune repository
    Init,
    /// Add file contents to the index
    Add {
        /// Files to add
        path: String,
    },
    /// List, create, or delete branches
    Branch {
        /// Name of the branch
        name: Option<String>,
        /// Delete the branch
        #[arg(short, long)]
        delete: bool,
    },
    /// Record changes to the repository
    Commit {
        /// Commit message
        #[arg(short, long)]
        message: String,
        
        /// Author name
        #[arg(short, long, default_value = "Rune User <user@example.com>")]
        author: String,
    },
    /// Show commit logs
    Log,
    /// Checkout a branch or commit
    Checkout {
        #[arg(help = "Branch name or commit hash to checkout")]
        target: String,
    },
    #[command(about = "Show changes between commits, commit and working tree, etc")]
    Diff {
        #[arg(help = "Old commit hash")]
        old_commit: String,
        #[arg(help = "New commit hash")]
        new_commit: String,
    },
    #[command(about = "Merge two or more development histories together")]
    Merge {
        #[arg(help = "Branch to merge into current HEAD")]
        branch: String,
    },
    #[command(about = "Update remote refs along with associated objects")]
    Push {
        #[arg(help = "Path to the local remote directory")]
        remote: String,
        #[arg(help = "Branch to push")]
        branch: String,
    },
    #[command(about = "Download objects and refs from another repository")]
    Fetch {
        #[arg(help = "Path to the local remote directory")]
        remote: String,
        #[arg(help = "Branch to fetch")]
        branch: String,
    },
    #[command(about = "Fetch from and integrate with another repository or a local branch")]
    Pull {
        #[arg(help = "Path to the local remote directory")]
        remote: String,
        #[arg(help = "Branch to pull")]
        branch: String,
    },
    #[command(about = "Manage set of tracked repositories")]
    Remote {
        #[arg(help = "Action: add, list")]
        action: String,
        name: Option<String>,
        url: Option<String>,
    },
    #[command(about = "Start the Runestone local web UI")]
    Web {
        #[arg(short, long, default_value_t = 8080)]
        port: u16,
    },
}

async fn get_remote(name_or_url: &str) -> Result<Box<dyn rune_core::remote::Remote>> {
    let cwd = std::env::current_dir()?;
    let config = rune_core::config::Config::load(&cwd)?;
    
    // Resolve from config if it's a name like "origin", else treat as url
    let url = config.get_remote(name_or_url).map(|s| s.as_str()).unwrap_or(name_or_url);
    
    if url.starts_with("gcs://") {
        let bucket = url.strip_prefix("gcs://").unwrap().to_string();
        let client = rune_core::remote::gcs::GcsRemote::new(&bucket)?;
        Ok(Box::new(client))
    } else {
        Ok(Box::new(rune_core::remote::local::LocalRemote::new(url)?))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init => {
            let cwd = std::env::current_dir().unwrap();
            rune_core::init(&cwd)?;
            println!("Initialized empty Rune repository in {:?}", cwd);
        }
        Commands::Add { path } => {
            let cwd = std::env::current_dir().unwrap();
            rune_core::add(&cwd, &std::path::PathBuf::from(path))?;
            println!("Added {} to index", path);
        }
        Commands::Commit { message, author } => {
            let cwd = std::env::current_dir().unwrap();
            let hash = rune_core::commit(&cwd, message, author)?;
            println!("Committed {}", hash);
        }
        Commands::Log => {
            let cwd = std::env::current_dir().unwrap();
            let commits = rune_core::log(&cwd)?;
            for (hash, commit) in commits {
                println!("commit {}", hash);
                println!("Author: {}", commit.author);
                
                // Just format as secs for now
                println!("Date:   {}", commit.timestamp);
                println!("\n    {}\n", commit.message);
            }
        }
        Commands::Checkout { target } => {
            let cwd = std::env::current_dir().unwrap();
            rune_core::checkout(&cwd, target)?;
            println!("Checked out {}", target);
        }
        Commands::Branch { name, delete } => {
            let cwd = std::env::current_dir().unwrap();
            if *delete {
                if let Some(n) = name {
                    rune_core::delete_branch(&cwd, n)?;
                    println!("Deleted branch {}", n);
                } else {
                    anyhow::bail!("Branch name required for deletion");
                }
            } else if let Some(n) = name {
                rune_core::create_branch(&cwd, n)?;
                println!("Created branch {}", n);
            } else {
                let branches = rune_core::list_branches(&cwd)?;
                for b in branches {
                    println!("{}", b);
                }
            }
        }
        Commands::Diff { old_commit, new_commit } => {
            let cwd = std::env::current_dir()?;
            match rune_core::diff(&cwd, &old_commit, &new_commit) {
                Ok(tree_diff) => {
                    if tree_diff.changes.is_empty() {
                        println!("No changes between {} and {}.", old_commit, new_commit);
                    } else {
                        let store = rune_core::store::ObjectStore::new(&cwd);
                        for (path, status) in tree_diff.changes {
                            match status {
                                rune_core::diff::DiffStatus::Added(hash) => {
                                    println!("+ {} (added)", path);
                                    if let Ok(text_diff) = rune_core::diff::diff_blobs(&store, &rune_core::object::ObjectId([0; 32]), &hash) {
                                        println!("{}", text_diff);
                                    }
                                }
                                rune_core::diff::DiffStatus::Deleted(hash) => {
                                    println!("- {} (deleted)", path);
                                    if let Ok(text_diff) = rune_core::diff::diff_blobs(&store, &hash, &rune_core::object::ObjectId([0; 32])) {
                                        println!("{}", text_diff);
                                    }
                                }
                                rune_core::diff::DiffStatus::Modified { old, new } => {
                                    println!("M {} (modified)", path);
                                    if let Ok(text_diff) = rune_core::diff::diff_blobs(&store, &old, &new) {
                                        println!("{}", text_diff);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => eprintln!("Error diffing: {}", e),
            }
        }
        Commands::Merge { branch } => {
            let cwd = std::env::current_dir()?;
            rune_core::merge(&cwd, &branch)?;
        }
        Commands::Remote { action, name, url } => {
            let cwd = std::env::current_dir()?;
            let mut config = rune_core::config::Config::load(&cwd)?;
            
            if action == "add" {
                if let (Some(n), Some(u)) = (name, url) {
                    config.set_remote(n, u);
                    config.save(&cwd)?;
                    println!("Added remote {} -> {}", n, u);
                } else {
                    eprintln!("Usage: rune remote add <name> <url>");
                }
            } else if action == "list" {
                for (n, u) in &config.remotes {
                    println!("{}  {}", n, u);
                }
            } else {
                eprintln!("Unknown remote action: {}", action);
            }
        }
        Commands::Push { remote, branch } => {
            let cwd = std::env::current_dir()?;
            
            let target_remote = remote.clone();
            
            // Interactive prompt if "origin" is requested but not found
            if remote == "origin" {
                let mut config = rune_core::config::Config::load(&cwd)?;
                if config.get_remote("origin").is_none() {
                    println!("No 'origin' remote is configured.");
                    println!("Would you like to set up a Google Cloud Storage bucket? [y/N]");
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    if input.trim().eq_ignore_ascii_case("y") {
                        println!("Enter your GCS bucket name (e.g. my-rune-backup):");
                        let mut bucket = String::new();
                        std::io::stdin().read_line(&mut bucket)?;
                        let bucket = bucket.trim();
                        if !bucket.is_empty() {
                            let url = format!("gcs://{}", bucket);
                            config.set_remote("origin", &url);
                            config.save(&cwd)?;
                            println!("Saved remote 'origin' -> {}", url);
                        } else {
                            println!("Bucket name cannot be empty. Aborting.");
                            return Ok(());
                        }
                    } else {
                        println!("Aborting push.");
                        return Ok(());
                    }
                }
            }

            let remote_obj = get_remote(&target_remote).await?;
            println!("Pushing {} to {}...", branch, target_remote);
            if let Err(e) = rune_core::sync::push(&cwd, &*remote_obj, &branch).await {
                eprintln!("Push failed: {:?}", e);
                if e.to_string().contains("gcloud auth application-default login") {
                    println!("\nHint: You need to authenticate with Google Cloud to push to GCS.");
                    println!("Run the following command in your terminal:");
                    println!("  gcloud auth application-default login");
                }
            } else {
                println!("Push successful!");
            }
        }
        Commands::Fetch { remote, branch } => {
            let cwd = std::env::current_dir()?;
            let remote_obj = get_remote(&remote).await?;
            rune_core::sync::fetch(&cwd, &*remote_obj, &branch).await?;
        }
        Commands::Pull { remote, branch } => {
            let cwd = std::env::current_dir()?;
            let remote_obj = get_remote(&remote).await?;
            rune_core::sync::pull(&cwd, &*remote_obj, &branch).await?;
        }
        Commands::Web { port } => {
            let cwd = std::env::current_dir()?;
            rune_web::start_server(*port, cwd).await?;
        }
    }

    Ok(())
}
