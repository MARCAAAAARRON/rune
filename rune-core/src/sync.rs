use std::collections::{HashSet, VecDeque};
use anyhow::{Context, Result};
use crate::object::Object;
use crate::store::ObjectStore;
use crate::remote::Remote;
use crate::refs;
use std::path::Path;

/// Push a local branch to a remote.
pub async fn push(repo_path: &Path, remote: &dyn Remote, branch: &str) -> Result<()> {
    let store = ObjectStore::new(repo_path);
    
    // Resolve local branch
    let local_hash = refs::read_ref(repo_path, branch)?
        .with_context(|| format!("Local branch {} does not exist", branch))?;
        
    let remote_refs = remote.list_refs().await?;
    let expected_old = remote_refs.get(branch).copied();
    
    // If the remote is already up to date, do nothing
    if Some(local_hash) == expected_old {
        println!("Everything up-to-date");
        return Ok(());
    }
    
    // Find all missing commits
    let mut commits_to_push = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    
    queue.push_back(local_hash);
    
    while let Some(hash) = queue.pop_front() {
        if Some(hash) == expected_old {
            continue; // Reached the remote's known state
        }
        
        if visited.insert(hash) {
            commits_to_push.push(hash);
            if let Ok(Object::Commit(c)) = store.read(&hash) {
                for parent in c.parent_hashes {
                    queue.push_back(parent);
                }
            }
        }
    }
    
    // Now collect trees and blobs for these commits
    let mut objects_to_push = HashSet::new(); // HashSet of ObjectIds
    
    for commit_hash in &commits_to_push {
        objects_to_push.insert(*commit_hash);
        
        if let Ok(Object::Commit(c)) = store.read(commit_hash) {
            objects_to_push.insert(c.tree_hash);
            
            // Walk tree recursively to find all blobs
            let mut tree_queue = VecDeque::new();
            tree_queue.push_back(c.tree_hash);
            
            while let Some(tree_hash) = tree_queue.pop_front() {
                if let Ok(Object::Tree(tree)) = store.read(&tree_hash) {
                    for entry in tree.entries {
                        objects_to_push.insert(entry.hash);
                        if let Ok(Object::Tree(_)) = store.read(&entry.hash) {
                            tree_queue.push_back(entry.hash);
                        }
                    }
                }
            }
        }
    }
    
    println!("Uploading {} objects...", objects_to_push.len());
    
    use futures::stream::{StreamExt, iter};

    let push_stream = iter(objects_to_push.into_iter()).map(|id| async move {
        if !remote.has_object(&id).await? {
            let hash_str = id.to_string();
            let path = repo_path.join(".rune").join("objects").join(&hash_str[0..2]).join(&hash_str[2..]);
            let data = std::fs::read(&path)?;
            remote.upload_object(&id, &data).await?;
        }
        anyhow::Result::<()>::Ok(())
    });

    let mut buffered_push = push_stream.buffer_unordered(10);
    while let Some(res) = buffered_push.next().await {
        res?; // Fail fast if any upload fails
    }
    
    remote.update_ref(branch, expected_old, local_hash).await?;
    println!("Pushed to remote branch {}", branch);
    
    Ok(())
}

/// Fetch a remote branch into the local repository.
pub async fn fetch(repo_path: &Path, remote: &dyn Remote, branch: &str) -> Result<()> {
    let _store = ObjectStore::new(repo_path);
    let remote_refs = remote.list_refs().await?;
    
    let remote_hash = match remote_refs.get(branch) {
        Some(h) => *h,
        None => anyhow::bail!("Remote branch {} does not exist", branch),
    };
    
    let local_remote_branch = format!("remotes/{}", branch);
    let current_local_remote_hash = refs::read_ref(repo_path, &local_remote_branch)?;
    
    if current_local_remote_hash == Some(remote_hash) {
        println!("Already up to date.");
        return Ok(());
    }
    
    let mut queue = VecDeque::new();
    queue.push_back(remote_hash);
    let mut fetched_count = 0;
    
    // Instead of buffer_unordered for fetch (which is trickier because we discover new objects to fetch recursively),
    // we stick to sequential for fetch since we need to decode objects to find dependencies. 
    // Concurrent fetch requires a more complex worker pool. We will keep fetch sequential for now.
    
    while let Some(hash) = queue.pop_front() {
        let hash_str = hash.to_string();
        let local_path = repo_path.join(".rune").join("objects").join(&hash_str[0..2]).join(&hash_str[2..]);
        
        if local_path.exists() {
            continue; // We already have this object
        }
        
        // Fetch it
        let data = remote.fetch_object(&hash).await?;
        
        let encoded = crate::codec::decompress(&data)
            .map_err(|_| anyhow::anyhow!("Decompression failed for fetched object {}", hash))?;
        
        let computed_hash = crate::codec::hash_bytes(&encoded);
        if computed_hash != hash {
            anyhow::bail!("Hash mismatch! Remote supplied malicious or corrupted data. Expected {}, got {}", hash, computed_hash);
        }
        
        // Ensure local parent dir exists
        if let Some(p) = local_path.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::fs::write(&local_path, &data)?;
        fetched_count += 1;
        
        // Now we need to parse it to find its children
        let obj = crate::codec::decode(&encoded)
            .map_err(|e| anyhow::anyhow!("Failed to decode fetched object {}: {}", hash, e))?;
            
        match obj {
            Object::Commit(c) => {
                queue.push_back(c.tree_hash);
                for p in c.parent_hashes {
                    queue.push_back(p);
                }
            }
            Object::Tree(t) => {
                for entry in t.entries {
                    queue.push_back(entry.hash);
                }
            }
            Object::Blob(_) => {} // Blobs have no children
        }
    }
    
    println!("Fetched {} objects.", fetched_count);
    
    // Update local remote-tracking branch
    refs::write_ref(repo_path, &local_remote_branch, &remote_hash)?;
    println!("Updated {} -> {}", branch, remote_hash);
    
    Ok(())
}

/// Pull a remote branch into the local repository (fetch + merge).
pub async fn pull(repo_path: &Path, remote: &dyn Remote, branch: &str) -> Result<()> {
    fetch(repo_path, remote, branch).await?;
    
    let local_remote_branch = format!("remotes/{}", branch);
    crate::merge(repo_path, &local_remote_branch)?;
    
    Ok(())
}
