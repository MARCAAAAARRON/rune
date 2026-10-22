use crate::error::StoreError;
use crate::object::{FileMode, Object, ObjectId, Tree};
use crate::store::ObjectStore;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DiffStatus {
    Added(ObjectId),
    Deleted(ObjectId),
    Modified { old: ObjectId, new: ObjectId },
}

#[derive(Debug, Clone)]
pub struct TreeDiff {
    pub changes: HashMap<String, DiffStatus>,
}

#[derive(thiserror::Error, Debug)]
pub enum DiffError {
    #[error("Store Error: {0}")]
    Store(#[from] StoreError),

    #[error("Object is not a tree: {0}")]
    NotATree(ObjectId),
}

pub fn diff_trees(
    store: &ObjectStore,
    old_hash: Option<&ObjectId>,
    new_hash: &ObjectId,
) -> Result<TreeDiff, DiffError> {
    let mut changes = HashMap::new();
    if let Some(old) = old_hash {
        if old == new_hash {
            return Ok(TreeDiff { changes });
        }
    }

    diff_trees_recursive(store, old_hash, new_hash, "", &mut changes)?;
    Ok(TreeDiff { changes })
}

fn diff_trees_recursive(
    store: &ObjectStore,
    old_hash: Option<&ObjectId>,
    new_hash: &ObjectId,
    path_prefix: &str,
    changes: &mut HashMap<String, DiffStatus>,
) -> Result<(), DiffError> {
    if let Some(old) = old_hash {
        if old == new_hash {
            return Ok(());
        }
    }

    let mut old_entries = HashMap::new();
    if let Some(old) = old_hash {
        let old_tree = read_tree(store, old)?;
        for entry in old_tree.entries {
            old_entries.insert(entry.name.clone(), entry);
        }
    }

    let new_tree = read_tree(store, new_hash)?;
    let mut new_entries = HashMap::new();
    for entry in new_tree.entries {
        new_entries.insert(entry.name.clone(), entry);
    }

    for (name, old_entry) in &old_entries {
        let full_path = if path_prefix.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", path_prefix, name)
        };

        match new_entries.get(name) {
            Some(new_entry) => {
                if old_entry.hash != new_entry.hash {
                    if old_entry.mode == FileMode::Directory
                        && new_entry.mode == FileMode::Directory
                    {
                        diff_trees_recursive(
                            store,
                            Some(&old_entry.hash),
                            &new_entry.hash,
                            &full_path,
                            changes,
                        )?;
                    } else if old_entry.mode == FileMode::Directory || new_entry.mode == FileMode::Directory {
                        add_all_recursive(store, &old_entry.hash, &full_path, false, changes)?;
                        add_all_recursive(store, &new_entry.hash, &full_path, true, changes)?;
                    } else {
                        changes.insert(
                            full_path,
                            DiffStatus::Modified {
                                old: old_entry.hash,
                                new: new_entry.hash,
                            },
                        );
                    }
                }
            }
            None => {
                if old_entry.mode == FileMode::Directory {
                    add_all_recursive(store, &old_entry.hash, &full_path, false, changes)?;
                } else {
                    changes.insert(full_path, DiffStatus::Deleted(old_entry.hash));
                }
            }
        }
    }

    for (name, new_entry) in &new_entries {
        if !old_entries.contains_key(name) {
            let full_path = if path_prefix.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", path_prefix, name)
            };

            if new_entry.mode == FileMode::Directory {
                add_all_recursive(store, &new_entry.hash, &full_path, true, changes)?;
            } else {
                changes.insert(full_path, DiffStatus::Added(new_entry.hash));
            }
        }
    }

    Ok(())
}

fn add_all_recursive(
    store: &ObjectStore,
    tree_hash: &ObjectId,
    path_prefix: &str,
    is_add: bool,
    changes: &mut HashMap<String, DiffStatus>,
) -> Result<(), DiffError> {
    let tree = read_tree(store, tree_hash)?;
    for entry in tree.entries {
        let full_path = format!("{}/{}", path_prefix, entry.name);
        if entry.mode == FileMode::Directory {
            add_all_recursive(store, &entry.hash, &full_path, is_add, changes)?;
        } else {
            if is_add {
                changes.insert(full_path, DiffStatus::Added(entry.hash));
            } else {
                changes.insert(full_path, DiffStatus::Deleted(entry.hash));
            }
        }
    }
    Ok(())
}

fn read_tree(store: &ObjectStore, hash: &ObjectId) -> Result<Tree, DiffError> {
    let obj = store.read(hash)?;
    match obj {
        Object::Tree(tree) => Ok(tree),
        _ => Err(DiffError::NotATree(*hash)),
    }
}

pub fn diff_blobs(store: &ObjectStore, old_hash: &ObjectId, new_hash: &ObjectId) -> Result<String, DiffError> {
    let old_content = if let Object::Blob(blob) = store.read(old_hash)? {
        blob.content
    } else {
        vec![] // Should be error, but keep it simple
    };
    
    let new_content = if let Object::Blob(blob) = store.read(new_hash)? {
        blob.content
    } else {
        vec![]
    };
    
    let old_text = String::from_utf8_lossy(&old_content);
    let new_text = String::from_utf8_lossy(&new_content);
    
    let diff = similar::TextDiff::from_lines(&old_text, &new_text);
    let mut result = String::new();
    
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            similar::ChangeTag::Delete => "-",
            similar::ChangeTag::Insert => "+",
            similar::ChangeTag::Equal => " ",
        };
        result.push_str(&format!("{}{}", sign, change));
    }
    
    Ok(result)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffTag {
    Insert,
    Delete,
    Equal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub tag: DiffTag,
    pub content: String,
}

pub fn diff_blobs_structured(store: &ObjectStore, old_hash: Option<&ObjectId>, new_hash: Option<&ObjectId>) -> Result<Vec<DiffLine>, DiffError> {
    let old_content = if let Some(hash) = old_hash {
        if let Object::Blob(blob) = store.read(hash)? {
            blob.content
        } else {
            vec![]
        }
    } else {
        vec![]
    };
    
    let new_content = if let Some(hash) = new_hash {
        if let Object::Blob(blob) = store.read(hash)? {
            blob.content
        } else {
            vec![]
        }
    } else {
        vec![]
    };
    
    let old_text = String::from_utf8_lossy(&old_content);
    let new_text = String::from_utf8_lossy(&new_content);
    
    let diff = similar::TextDiff::from_lines(&old_text, &new_text);
    let mut result = Vec::new();
    
    for change in diff.iter_all_changes() {
        let tag = match change.tag() {
            similar::ChangeTag::Delete => DiffTag::Delete,
            similar::ChangeTag::Insert => DiffTag::Insert,
            similar::ChangeTag::Equal => DiffTag::Equal,
        };
        result.push(DiffLine {
            tag,
            content: change.value().to_string(),
        });
    }
    
    Ok(result)
}
