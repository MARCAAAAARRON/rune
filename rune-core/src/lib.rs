//! # Rune Core Engine
//!
//! `rune-core` provides the fundamental domain logic, data models, and algorithms
//! for the Rune distributed Version Control System.
//!
//! ## Key Subsystems
//! - **Content-Addressed Storage (`codec`, `store`, `object`):** SHA-256 Merkle DAG with Zstandard compression.
//! - **Staging & Tree Building (`index`, `tree_walk`):** Working directory staging and hierarchical tree serialization.
//! - **Refs & Branch Management (`refs`):** Symbolic HEAD and branch ref pointers.
//! - **Diff & Merge (`diff`, `merge`):** Myers file diffing and 3-way LCA merge resolution.
//! - **Remote Synchronization (`remote`, `sync`):** Serverless push/pull backed by GCS and local directories.
//! - **Path Traversal Sandboxing (`checkout`):** Strict path validation against directory escapes and symlink attacks.

use std::path::Path;

pub mod codec;
pub mod error;
pub mod index;
pub mod object;
pub mod refs;
pub mod store;
pub mod diff;
pub mod merge;
pub mod remote;
pub mod sync;
pub mod checkout;
pub mod config;
pub mod tree_walk;

pub use checkout::{checkout, CheckoutError};

/// Initializes an empty Rune repository in the specified directory.
///
/// Creates the `.rune/` directory structure, including `objects/`, `refs/heads/`,
/// `refs/remotes/`, `HEAD` (defaulting to `refs/heads/main`), and initial `config`.
///
/// # Errors
/// Returns [`error::InitError`] if repository creation fails or if `.rune` already exists.
pub fn init<P: AsRef<Path>>(repo_path: P) -> Result<(), error::InitError> {
    store::init(repo_path)
}

/// Creates a new branch pointing to the current `HEAD` commit.
///
/// # Arguments
/// * `repo_path` - Path to the root of the repository.
/// * `name` - The name of the new branch (e.g. `"feature-auth"`).
///
/// # Errors
/// Returns [`error::RefsError`] if `HEAD` cannot be resolved or if writing the ref fails.
pub fn create_branch<P: AsRef<Path>>(repo_path: P, name: &str) -> Result<(), error::RefsError> {
    let repo_path = repo_path.as_ref();
    // Resolve HEAD to get the current commit
    let head = refs::read_head(repo_path)?;
    let commit_hash = match head {
        refs::HeadRef::Detached(hash) => hash,
        refs::HeadRef::Branch(branch_name) => {
            refs::read_ref(repo_path, &branch_name)?
                .ok_or_else(|| error::RefsError::NotFound("HEAD points to a non-existent branch".into()))?
        }
    };
    
    // Write new branch ref
    refs::write_ref(repo_path, name, &commit_hash)?;
    Ok(())
}

/// Deletes an existing branch reference.
///
/// # Arguments
/// * `repo_path` - Path to the root of the repository.
/// * `name` - The name of the branch to delete.
///
/// # Errors
/// Returns [`error::RefsError`] if the branch cannot be found or deleted.
pub fn delete_branch<P: AsRef<Path>>(repo_path: P, name: &str) -> Result<(), error::RefsError> {
    Ok(refs::delete_ref(repo_path, name)?)
}

/// Lists all local branch names in the repository.
///
/// # Arguments
/// * `repo_path` - Path to the root of the repository.
///
/// # Returns
/// A vector of branch name strings.
pub fn list_branches<P: AsRef<Path>>(repo_path: P) -> Result<Vec<String>, error::RefsError> {
    Ok(refs::list_refs(repo_path)?)
}

/// Stages a file or directory into the repository index.
///
/// When `target` is a directory, this walks the subtree recursively, respecting
/// `.runeignore` and `.gitignore` patterns while automatically ignoring `.rune` and `.git`.
///
/// # Errors
/// Returns [`error::AddError`] if file reading, hashing, or index saving fails.
pub fn add<P: AsRef<Path>>(repo_path: P, target: P) -> Result<(), error::AddError> {
    let repo_path = repo_path.as_ref();
    let target = target.as_ref();
    
    let mut index = index::Index::load(repo_path)?;
    
    if target.is_dir() {
        let walker = ignore::WalkBuilder::new(target)
            .add_custom_ignore_filename(".runeignore")
            .hidden(false) // Don't ignore hidden files by default (unless in .runeignore or .gitignore)
            .filter_entry(|entry| {
                // Manually ignore .rune and .git directories
                let name = entry.file_name();
                name != std::ffi::OsStr::new(".rune") && name != std::ffi::OsStr::new(".git")
            })
            .build();
            
        for result in walker {
            match result {
                Ok(entry) => {
                    let path = entry.path();
                    if path.is_file() || path.is_symlink() {
                        add_file(repo_path, path, &mut index)?;
                    }
                }
                Err(e) => {
                    // Log or ignore errors? Let's propagate for now, or just print warning.
                    eprintln!("Warning: skipping unreadable path: {}", e);
                }
            }
        }
    } else if target.is_file() {
        // If they manually add a file, we usually still respect ignore unless forced,
        // but for simplicity, we'll just add it.
        add_file(repo_path, target, &mut index)?;
    }
    
    index.save(repo_path)?;
    Ok(())
}

fn add_file(repo_path: &Path, file_path: &Path, index: &mut index::Index) -> Result<(), error::AddError> {
    use object::{Blob, Object};
    use std::fs;

    let content = fs::read(file_path)?;
    let size = content.len() as u64;
    
    let metadata = fs::symlink_metadata(file_path)?;
    let mtime = metadata.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mode = if metadata.is_symlink() {
        object::FileMode::Symlink
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o111 != 0 {
                object::FileMode::Executable
            } else {
                object::FileMode::Regular
            }
        }
        #[cfg(not(unix))]
        {
            object::FileMode::Regular
        }
    };

    let blob = Object::Blob(Blob { content });
    let store = store::ObjectStore::new(repo_path);
    let hash = store.write(&blob)?;

    // We must canonicalize paths to safely strip the repo prefix
    let abs_file_path = std::fs::canonicalize(file_path)?;
    let abs_repo_path = std::fs::canonicalize(repo_path)?;
    
    let rel_path = abs_file_path.strip_prefix(&abs_repo_path).unwrap_or(&abs_file_path);
    let rel_path_str = rel_path.to_str().unwrap().replace("\\", "/");

    let entry = index::IndexEntry {
        path: rel_path_str.clone(),
        hash,
        mode,
        size,
        mtime,
    };

    index.add_entry(entry);
    Ok(())
}

/// Creates a new commit snapshot from the current staging index.
///
/// Builds hierarchical Merkle trees recursively from staged index entries,
/// resolves the current parent commit from `HEAD`, creates the [`object::Commit`]
/// object with author metadata and timestamp, writes it to the object store,
/// and updates the active branch ref (or detached HEAD pointer).
///
/// # Arguments
/// * `repo_path` - Path to the root of the repository.
/// * `message` - The commit message.
/// * `author` - Author identity string (e.g. `"Name <email@example.com>"`).
///
/// # Returns
/// The computed [`object::ObjectId`] (SHA-256) of the newly created commit.
///
/// # Errors
/// Returns [`error::CommitError::NothingToCommit`] if the staging index has no entries,
/// or an error if tree building or object store writes fail.
pub fn commit<P: AsRef<Path>>(repo_path: P, message: &str, author: &str) -> Result<object::ObjectId, error::CommitError> {
    let repo_path = repo_path.as_ref();
    let index = index::Index::load(repo_path)?;
    
    if index.entries.is_empty() {
        return Err(error::CommitError::NothingToCommit);
    }
    
    let store = store::ObjectStore::new(repo_path);
    
    // Build tree
    let root_tree_hash = build_tree(&store, &index.entries, "")?;
    
    // Get parent
    let head = refs::read_head(repo_path)?;
    let mut parent_hashes = Vec::new();
    let mut current_branch = None;
    
    match head {
        refs::HeadRef::Detached(hash) => parent_hashes.push(hash),
        refs::HeadRef::Branch(branch_name) => {
            current_branch = Some(branch_name.clone());
            if let Some(hash) = refs::read_ref(repo_path, &branch_name)? {
                parent_hashes.push(hash);
            }
        }
    }
    
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
        
    let commit = object::Commit {
        tree_hash: root_tree_hash,
        parent_hashes,
        author: author.to_string(),
        committer: author.to_string(),
        timestamp,
        message: message.to_string(),
    };
    
    let commit_hash = store.write(&object::Object::Commit(commit))?;
    
    // Update ref
    if let Some(branch_name) = current_branch {
        refs::write_ref(repo_path, &branch_name, &commit_hash)?;
    } else {
        refs::write_head(repo_path, &refs::HeadRef::Detached(commit_hash))?;
    }
    
    Ok(commit_hash)
}

fn build_tree(
    store: &store::ObjectStore,
    entries: &std::collections::BTreeMap<String, index::IndexEntry>,
    prefix: &str,
) -> Result<object::ObjectId, error::CommitError> {
    use std::collections::BTreeMap;
    use object::{Tree, TreeEntry, Object};

    let mut direct_entries = Vec::new();
    let mut sub_trees: BTreeMap<String, BTreeMap<String, index::IndexEntry>> = BTreeMap::new();

    for (path, entry) in entries {
        if !path.starts_with(prefix) {
            continue;
        }
        
        let rel_path = if prefix.is_empty() {
            path.as_str()
        } else {
            &path[prefix.len()..]
        };
        
        // Remove leading slash if any (though we shouldn't have one if prefix ends in /)
        let rel_path = rel_path.trim_start_matches('/');
        
        if let Some(slash_idx) = rel_path.find('/') {
            // It's in a subdirectory
            let dir_name = &rel_path[..slash_idx];
            sub_trees.entry(dir_name.to_string()).or_default().insert(path.clone(), entry.clone());
        } else {
            // It's a direct child
            direct_entries.push(TreeEntry {
                mode: entry.mode,
                name: rel_path.to_string(),
                hash: entry.hash,
            });
        }
    }
    
    for (dir_name, sub_entries) in sub_trees {
        let new_prefix = if prefix.is_empty() {
            format!("{}/", dir_name)
        } else {
            format!("{}{}/", prefix, dir_name)
        };
        
        let sub_tree_hash = build_tree(store, &sub_entries, &new_prefix)?;
        
        direct_entries.push(TreeEntry {
            mode: object::FileMode::Directory,
            name: dir_name,
            hash: sub_tree_hash,
        });
    }
    
    let tree = Tree {
        entries: direct_entries,
    };
    
    let hash = store.write(&Object::Tree(tree))?;
    Ok(hash)
}

fn resolve_hash(repo_path: &Path, rev: &str) -> anyhow::Result<object::ObjectId> {
    use std::str::FromStr;
    if let Ok(hash) = object::ObjectId::from_str(rev) {
        return Ok(hash);
    }
    
    if let Some(hash) = refs::read_ref(repo_path, rev)? {
        return Ok(hash);
    }
    
    if rev == "HEAD" {
        let head = refs::read_head(repo_path)?;
        match head {
            refs::HeadRef::Detached(h) => return Ok(h),
            refs::HeadRef::Branch(b) => {
                if let Some(hash) = refs::read_ref(repo_path, &b)? {
                    return Ok(hash);
                }
            }
        }
    }
    
    anyhow::bail!("Cannot resolve revision: {}", rev)
}

/// Computes the tree-level differences between two commits.
///
/// Resolves both commit revisions (branch name, commit hash, or `"HEAD"`),
/// reads their respective root tree objects, and performs a recursive diff.
/// Unchanged subtrees short-circuit execution when their hashes match.
///
/// # Arguments
/// * `repo_path` - Path to the root of the repository.
/// * `old_commit_hash` - Revision identifier for the base/older commit.
/// * `new_commit_hash` - Revision identifier for the target/newer commit.
///
/// # Returns
/// A [`diff::TreeDiff`] containing added, deleted, and modified paths.
pub fn diff(repo_path: &Path, old_commit_hash: &str, new_commit_hash: &str) -> anyhow::Result<diff::TreeDiff> {
    use store::ObjectStore;
    use object::Object;

    let store = ObjectStore::new(repo_path);
    
    let old_obj_id = resolve_hash(repo_path, old_commit_hash)?;
    let new_obj_id = resolve_hash(repo_path, new_commit_hash)?;
    
    let old_commit = match store.read(&old_obj_id)? {
        Object::Commit(c) => c,
        _ => anyhow::bail!("{} is not a commit", old_commit_hash),
    };
    
    let new_commit = match store.read(&new_obj_id)? {
        Object::Commit(c) => c,
        _ => anyhow::bail!("{} is not a commit", new_commit_hash),
    };
    
    let tree_diff = diff::diff_trees(&store, Some(&old_commit.tree_hash), &new_commit.tree_hash)?;
    Ok(tree_diff)
}

/// Merges changes from `other_branch` into the current `HEAD` using a 3-way LCA merge.
///
/// 1. Finds the Lowest Common Ancestor (LCA) between `HEAD` and `other_branch`.
/// 2. Performs a 3-way merge on trees and blob hunks.
/// 3. Writes auto-resolved files directly to the working tree.
/// 4. If conflicting hunk edits occur, writes conflict markers (`<<<<<<< HEAD`, `=======`, `>>>>>>>`)
///    directly to the affected files in the working tree and returns an error notifying the user.
///
/// # Arguments
/// * `repo_path` - Path to the root of the repository.
/// * `other_branch` - Name or revision of the branch to merge in.
///
/// # Errors
/// Returns an error if no common ancestor exists, if commits cannot be resolved,
/// or if merge conflicts occur.
pub fn merge(repo_path: &Path, other_branch: &str) -> anyhow::Result<()> {
    use store::ObjectStore;
    use merge::MergeError;
    use std::fs;
    
    let store = ObjectStore::new(repo_path);
    
    let head_hash = resolve_hash(repo_path, "HEAD")?;
    let other_hash = resolve_hash(repo_path, other_branch)?;
    
    let base = match merge::find_merge_base(&store, &head_hash, &other_hash) {
        Ok(b) => b,
        Err(MergeError::NoCommonAncestor) => anyhow::bail!("Refusing to merge unrelated histories"),
        Err(e) => return Err(e.into()),
    };
    
    let merge_result = merge::merge_commits(&store, &base, &head_hash, &other_hash)?;
    
    let mut has_conflicts = false;
    
    for (path, content) in merge_result.add_or_modify {
        let full_path = repo_path.join(&path);
        if let Some(p) = full_path.parent() {
            fs::create_dir_all(p)?;
        }
        fs::write(&full_path, content)?;
        println!("Merged: {}", path);
    }
    
    for path in merge_result.delete {
        let full_path = repo_path.join(&path);
        if full_path.exists() {
            fs::remove_file(&full_path)?;
            println!("Deleted: {}", path);
        }
    }
    
    for (path, content) in merge_result.conflicts {
        let full_path = repo_path.join(&path);
        if let Some(p) = full_path.parent() {
            fs::create_dir_all(p)?;
        }
        fs::write(&full_path, content)?;
        println!("CONFLICT: {}", path);
        has_conflicts = true;
    }
    
    if has_conflicts {
        anyhow::bail!("Merge conflicts found. Resolve them before committing.");
    }
    
    println!("Merge successful. You must commit to finalize.");
    Ok(())
}

/// Retrieves the commit history starting from the current `HEAD` pointer.
///
/// Traverses the commit DAG backward following parent pointers using a visited set
/// to prevent cycles or redundant visits across merge commits, returning commits
/// sorted in reverse-chronological order.
///
/// # Arguments
/// * `repo_path` - Path to the root of the repository.
///
/// # Returns
/// A vector of `(ObjectId, Commit)` tuples sorted chronologically descending.
pub fn log<P: AsRef<Path>>(repo_path: P) -> Result<Vec<(object::ObjectId, object::Commit)>, error::CommitError> {
    let repo_path = repo_path.as_ref();
    let head = refs::read_head(repo_path)?;
    
    let start_hash = match head {
        refs::HeadRef::Detached(hash) => Some(hash),
        refs::HeadRef::Branch(branch_name) => refs::read_ref(repo_path, &branch_name)?,
    };
    
    let store = store::ObjectStore::new(repo_path);
    let mut commits = Vec::new();
    let mut visited = std::collections::HashSet::new();
    
    let mut queue = std::collections::VecDeque::new();
    if let Some(hash) = start_hash {
        queue.push_back(hash);
    }
    
    while let Some(hash) = queue.pop_front() {
        if !visited.insert(hash) {
            continue;
        }
        
        let obj = store.read(&hash)?;
        if let object::Object::Commit(commit) = obj {
            for parent in &commit.parent_hashes {
                queue.push_back(*parent);
            }
            commits.push((hash, commit));
        }
    }
    
    commits.sort_by(|a, b| b.1.timestamp.cmp(&a.1.timestamp));
    Ok(commits)
}

/// Retrieves the commit history starting from a specified reference or commit hash.
///
/// # Arguments
/// * `repo_path` - Path to the root of the repository.
/// * `ref_name` - Branch name or commit hash to begin traversal from.
///
/// # Returns
/// A vector of `(ObjectId, Commit)` tuples sorted chronologically descending.
pub fn log_from_ref<P: AsRef<Path>>(repo_path: P, ref_name: &str) -> Result<Vec<(object::ObjectId, object::Commit)>, error::CommitError> {
    let repo_path = repo_path.as_ref();
    let start_hash = resolve_hash(repo_path, ref_name).map_err(|_| error::CommitError::NothingToCommit)?;
    
    let store = store::ObjectStore::new(repo_path);
    let mut commits = Vec::new();
    let mut visited = std::collections::HashSet::new();
    
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(start_hash);
    
    while let Some(hash) = queue.pop_front() {
        if !visited.insert(hash) {
            continue;
        }
        
        let obj = store.read(&hash)?;
        if let object::Object::Commit(commit) = obj {
            for parent in &commit.parent_hashes {
                queue.push_back(*parent);
            }
            commits.push((hash, commit));
        }
    }
    
    commits.sort_by(|a, b| b.1.timestamp.cmp(&a.1.timestamp));
    Ok(commits)
}
