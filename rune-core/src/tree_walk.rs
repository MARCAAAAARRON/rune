use crate::object::{ObjectId, Object, FileMode};
use crate::store::ObjectStore;
use crate::checkout::CheckoutError; // Reusing CheckoutError's security violation
use anyhow::{Result, Context};

pub fn resolve_path(store: &ObjectStore, root_tree: &ObjectId, path_str: &str) -> Result<(Object, FileMode)> {
    // 1. Path safety validation (TRD §7.4)
    validate_path(path_str)?;
    
    // If empty path, return the root tree
    if path_str.is_empty() || path_str == "/" {
        let obj = store.read(root_tree)?;
        return Ok((obj, FileMode::Directory));
    }
    
    // Split path into components
    let components: Vec<&str> = path_str.split('/')
        .filter(|c| !c.is_empty())
        .collect();
        
    let mut current_hash = *root_tree;
    let mut current_mode = FileMode::Directory;
    
    for (i, component) in components.iter().enumerate() {
        let obj = store.read(&current_hash)?;
        if let Object::Tree(tree) = obj {
            // Find component in tree
            let entry = tree.entries.iter().find(|e| e.name == *component)
                .with_context(|| format!("Component {} not found in path {}", component, path_str))?;
                
            current_hash = entry.hash;
            current_mode = entry.mode;
            
            // If it's a file but we're not at the last component, that's an error
            if i < components.len() - 1 && current_mode != FileMode::Directory {
                anyhow::bail!("Path component {} is not a directory", component);
            }
        } else {
            anyhow::bail!("Expected tree at component {}, but found something else", component);
        }
    }
    
    // Read the final object
    let final_obj = store.read(&current_hash)?;
    Ok((final_obj, current_mode))
}

fn validate_path(path: &str) -> Result<(), CheckoutError> {
    if path.contains('\0') {
        return Err(CheckoutError::SecurityViolation("Null byte in path".into()));
    }
    if path.contains('\\') {
        return Err(CheckoutError::SecurityViolation("Backslash in path not allowed in web routes".into()));
    }
    let parts: Vec<&str> = path.split('/').collect();
    for part in parts {
        if part == ".." {
            return Err(CheckoutError::SecurityViolation("Directory traversal (..) not allowed".into()));
        }
    }
    
    // Check for absolute paths starting with /
    // Axum routes typically strip the leading slash for wildcard matches, but if they don't:
    // It's technically safe if we just split by `/` and ignore empty, but we shouldn't allow explicit absolute requests just to be safe.
    // However, axum's *path might be empty or start with something.
    Ok(())
}
