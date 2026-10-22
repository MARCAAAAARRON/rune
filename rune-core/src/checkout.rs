use crate::error::StoreError;
use crate::object::{FileMode, Object, ObjectId};
use crate::store::ObjectStore;
use crate::{index, refs};
use std::fs;
use std::io;
use std::path::Path;

#[derive(thiserror::Error, Debug)]
pub enum CheckoutError {
    #[error("I/O Error: {0}")]
    Io(#[from] io::Error),

    #[error("Store Error: {0}")]
    Store(#[from] StoreError),

    #[error("Refs Error: {0}")]
    Refs(#[from] crate::error::RefsError),

    #[error("Security violation: absolute or escaping path detected: {0}")]
    SecurityViolation(String),

    #[error("Object is not a commit")]
    NotACommit,

    #[error("Object is not a tree")]
    NotATree,

    #[error("Object is not a blob")]
    NotABlob,
}

pub fn checkout<P: AsRef<Path>>(
    repo_path: P,
    target: &str,
) -> Result<(), CheckoutError> {
    let repo_path = repo_path.as_ref();
    let store = ObjectStore::new(repo_path);

    // Resolve target to a commit hash
    let commit_hash = if let Some(hash) = refs::read_ref(repo_path, target)? {
        hash
    } else {
        // Try parsing as raw hash
        target
            .parse::<ObjectId>()
            .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "Target not found"))?
    };

    let obj = store.read(&commit_hash)?;
    let commit = match obj {
        Object::Commit(c) => c,
        _ => return Err(CheckoutError::NotACommit),
    };

    let mut new_index = index::Index::new();
    
    // First, we should clear the working directory or just overwrite.
    // For simplicity, we overwrite existing files and don't delete untracked ones.
    
    checkout_tree(
        repo_path,
        &store,
        &commit.tree_hash,
        Path::new(""),
        &mut new_index,
    )?;

    // Update index
    new_index.save(repo_path)?;

    // Update HEAD
    let new_head = if refs::read_ref(repo_path, target)?.is_some() {
        refs::HeadRef::Branch(target.to_string())
    } else {
        refs::HeadRef::Detached(commit_hash)
    };
    refs::write_head(repo_path, &new_head)?;

    Ok(())
}

fn checkout_tree(
    repo_path: &Path,
    store: &ObjectStore,
    tree_hash: &ObjectId,
    current_prefix: &Path,
    index: &mut index::Index,
) -> Result<(), CheckoutError> {
    let obj = store.read(tree_hash)?;
    let tree = match obj {
        Object::Tree(t) => t,
        _ => return Err(CheckoutError::NotATree),
    };

    for entry in tree.entries {
        // TRD §9.1: Validate entry name
        validate_path_component(&entry.name)?;

        let entry_rel_path = current_prefix.join(&entry.name);
        let entry_abs_path = repo_path.join(&entry_rel_path);

        match entry.mode {
            FileMode::Directory => {
                if let Ok(meta) = fs::symlink_metadata(&entry_abs_path) {
                    if meta.is_symlink() || !meta.is_dir() {
                        let _ = fs::remove_file(&entry_abs_path);
                    }
                }
                fs::create_dir_all(&entry_abs_path)?;
                checkout_tree(
                    repo_path,
                    store,
                    &entry.hash,
                    &entry_rel_path,
                    index,
                )?;
            }
            FileMode::Symlink => {
                let obj = store.read(&entry.hash)?;
                if let Object::Blob(blob) = obj {
                    let target_path = String::from_utf8_lossy(&blob.content).to_string();
                    
                    // TRD §9: Validate symlink target to prevent escaping the repo
                    if target_path.starts_with('/') || target_path.starts_with('\\') || target_path.contains("..") {
                        return Err(CheckoutError::SecurityViolation(format!("Malicious symlink target: {}", target_path)));
                    }
                    
                    #[cfg(unix)]
                    {
                        let _ = fs::remove_file(&entry_abs_path); // Remove if exists
                        std::os::unix::fs::symlink(&target_path, &entry_abs_path)?;
                    }
                    #[cfg(not(unix))]
                    {
                        // On Windows/non-unix, just write as text for simplicity or ignore
                        fs::write(&entry_abs_path, target_path)?;
                    }

                    add_to_index(index, &entry_rel_path, &entry.hash, FileMode::Symlink, blob.content.len() as u64, &entry_abs_path)?;
                } else {
                    return Err(CheckoutError::NotABlob);
                }
            }
            FileMode::Executable | FileMode::Regular => {
                let obj = store.read(&entry.hash)?;
                if let Object::Blob(blob) = obj {
                    if let Ok(meta) = fs::symlink_metadata(&entry_abs_path) {
                        if meta.is_symlink() {
                            let _ = fs::remove_file(&entry_abs_path);
                        }
                    }
                    if let Some(parent) = entry_abs_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&entry_abs_path, &blob.content)?;

                    #[cfg(unix)]
                    {
                        if entry.mode == FileMode::Executable {
                            use std::os::unix::fs::PermissionsExt;
                            let mut perms = fs::metadata(&entry_abs_path)?.permissions();
                            perms.set_mode(0o755);
                            fs::set_permissions(&entry_abs_path, perms)?;
                        }
                    }

                    add_to_index(index, &entry_rel_path, &entry.hash, entry.mode, blob.content.len() as u64, &entry_abs_path)?;
                } else {
                    return Err(CheckoutError::NotABlob);
                }
            }
        }
    }

    Ok(())
}

fn add_to_index(
    index: &mut index::Index,
    rel_path: &Path,
    hash: &ObjectId,
    mode: FileMode,
    size: u64,
    abs_path: &Path,
) -> Result<(), CheckoutError> {
    let metadata = fs::symlink_metadata(abs_path)?;
    let mtime = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let rel_path_str = rel_path.to_str().unwrap().replace("\\", "/");

    index.add_entry(index::IndexEntry {
        path: rel_path_str,
        hash: *hash,
        mode,
        size,
        mtime,
    });
    
    Ok(())
}

fn validate_path_component(name: &str) -> Result<(), CheckoutError> {
    if name.contains('\0') {
        return Err(CheckoutError::SecurityViolation("Null byte in path".into()));
    }
    if name.contains('/') || name.contains('\\') {
        return Err(CheckoutError::SecurityViolation("Slash in tree entry name".into()));
    }
    if name == ".." || name == "." {
        return Err(CheckoutError::SecurityViolation("Dot directory traversal".into()));
    }
    Ok(())
}
