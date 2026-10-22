use std::collections::{HashSet, VecDeque};
use crate::object::{ObjectId, Object};
use crate::store::ObjectStore;
use crate::error::StoreError;
use crate::diff::DiffError;

#[derive(thiserror::Error, Debug)]
pub enum MergeError {
    #[error("Store error: {0}")]
    Store(#[from] StoreError),
    #[error("Diff error: {0}")]
    Diff(#[from] DiffError),
    #[error("Merge conflict: {0}")]
    Conflict(String),
    #[error("Not a commit: {0}")]
    NotACommit(ObjectId),
    #[error("No common ancestor found")]
    NoCommonAncestor,
}

pub fn find_merge_base(
    store: &ObjectStore,
    commit_a: &ObjectId,
    commit_b: &ObjectId,
) -> Result<ObjectId, MergeError> {
    if commit_a == commit_b {
        return Ok(*commit_a);
    }

    let mut visited_a = HashSet::new();
    let mut visited_b = HashSet::new();
    
    let mut queue_a = VecDeque::new();
    queue_a.push_back(*commit_a);
    
    let mut queue_b = VecDeque::new();
    queue_b.push_back(*commit_b);
    
    while !queue_a.is_empty() || !queue_b.is_empty() {
        if let Some(a) = queue_a.pop_front() {
            if visited_b.contains(&a) {
                return Ok(a);
            }
            if visited_a.insert(a) {
                if let Ok(Object::Commit(c)) = store.read(&a) {
                    for parent in c.parent_hashes {
                        queue_a.push_back(parent);
                    }
                }
            }
        }
        
        if let Some(b) = queue_b.pop_front() {
            if visited_a.contains(&b) {
                return Ok(b);
            }
            if visited_b.insert(b) {
                if let Ok(Object::Commit(c)) = store.read(&b) {
                    for parent in c.parent_hashes {
                        queue_b.push_back(parent);
                    }
                }
            }
        }
    }
    
    Err(MergeError::NoCommonAncestor)
}

pub struct MergeResult {
    pub add_or_modify: std::collections::HashMap<String, Vec<u8>>,
    pub delete: Vec<String>,
    pub conflicts: std::collections::HashMap<String, Vec<u8>>,
}

pub fn merge_commits(
    store: &ObjectStore,
    base: &ObjectId,
    head: &ObjectId,
    other: &ObjectId,
) -> Result<MergeResult, MergeError> {
    let base_commit = get_commit(store, base)?;
    let head_commit = get_commit(store, head)?;
    let other_commit = get_commit(store, other)?;

    let head_diff = crate::diff::diff_trees(store, Some(&base_commit.tree_hash), &head_commit.tree_hash)?;
    let other_diff = crate::diff::diff_trees(store, Some(&base_commit.tree_hash), &other_commit.tree_hash)?;

    let mut result = MergeResult {
        add_or_modify: std::collections::HashMap::new(),
        delete: Vec::new(),
        conflicts: std::collections::HashMap::new(),
    };

    let mut all_paths: HashSet<String> = HashSet::new();
    for p in head_diff.changes.keys() { all_paths.insert(p.clone()); }
    for p in other_diff.changes.keys() { all_paths.insert(p.clone()); }

    for path in all_paths {
        let h_change = head_diff.changes.get(&path);
        let o_change = other_diff.changes.get(&path);

        match (h_change, o_change) {
            (Some(crate::diff::DiffStatus::Added(h_hash)), Some(crate::diff::DiffStatus::Added(o_hash))) => {
                if h_hash == o_hash {
                    result.add_or_modify.insert(path, get_blob_content(store, h_hash));
                } else {
                    result.conflicts.insert(path, format_conflict(store, h_hash, o_hash));
                }
            }
            (Some(crate::diff::DiffStatus::Modified{new: h_hash, ..}), Some(crate::diff::DiffStatus::Modified{new: o_hash, ..})) => {
                if h_hash == o_hash {
                    result.add_or_modify.insert(path, get_blob_content(store, h_hash));
                } else {
                    result.conflicts.insert(path, format_conflict(store, h_hash, o_hash));
                }
            }
            (Some(crate::diff::DiffStatus::Deleted(_)), Some(crate::diff::DiffStatus::Deleted(_))) => {
                result.delete.push(path);
            }
            (Some(crate::diff::DiffStatus::Modified{..}), Some(crate::diff::DiffStatus::Deleted(_))) |
            (Some(crate::diff::DiffStatus::Deleted(_)), Some(crate::diff::DiffStatus::Modified{..})) => {
                result.conflicts.insert(path, b"CONFLICT: file modified in one branch and deleted in another\n".to_vec());
            }
            (Some(crate::diff::DiffStatus::Added(hash)), None) |
            (Some(crate::diff::DiffStatus::Modified{new: hash, ..}), None) => {
                result.add_or_modify.insert(path, get_blob_content(store, hash));
            }
            (Some(crate::diff::DiffStatus::Deleted(_)), None) => {
                result.delete.push(path);
            }
            (None, Some(crate::diff::DiffStatus::Added(hash))) |
            (None, Some(crate::diff::DiffStatus::Modified{new: hash, ..})) => {
                result.add_or_modify.insert(path, get_blob_content(store, hash));
            }
            (None, Some(crate::diff::DiffStatus::Deleted(_))) => {
                result.delete.push(path);
            }
            (None, None) => {}
            _ => {}
        }
    }
    
    Ok(result)
}

fn get_commit(store: &ObjectStore, hash: &ObjectId) -> Result<crate::object::Commit, MergeError> {
    if let Object::Commit(c) = store.read(hash)? {
        Ok(c)
    } else {
        Err(MergeError::NotACommit(*hash))
    }
}

fn get_blob_content(store: &ObjectStore, hash: &ObjectId) -> Vec<u8> {
    if let Ok(Object::Blob(blob)) = store.read(hash) {
        blob.content
    } else {
        vec![]
    }
}

fn format_conflict(store: &ObjectStore, hash_a: &ObjectId, hash_b: &ObjectId) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"<<<<<<< HEAD\n");
    out.extend_from_slice(get_blob_content(store, hash_a).as_slice());
    out.extend_from_slice(b"=======\n");
    out.extend_from_slice(get_blob_content(store, hash_b).as_slice());
    out.extend_from_slice(b">>>>>>> OTHER\n");
    out
}
