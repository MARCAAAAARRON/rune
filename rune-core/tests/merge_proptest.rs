use proptest::prelude::*;
use proptest::collection::vec;
use rune_core::store::ObjectStore;
use rune_core::object::{Blob, Commit, FileMode, Object, ObjectId, Tree, TreeEntry};
use rune_core::diff::diff_trees;
use tempfile::TempDir;
use std::collections::HashMap;

fn create_tree(store: &ObjectStore, entries: Vec<(&str, ObjectId)>) -> ObjectId {
    let mut tree_entries = Vec::new();
    for (name, hash) in entries {
        tree_entries.push(TreeEntry {
            mode: FileMode::Regular,
            name: name.to_string(),
            hash,
        });
    }
    store.write(&Object::Tree(Tree { entries: tree_entries })).unwrap()
}

fn store_blob(store: &ObjectStore, content: &[u8]) -> ObjectId {
    store.write(&Object::Blob(Blob { content: content.to_vec() })).unwrap()
}

proptest! {
    #[test]
    fn test_diff_identical_trees(
        file_contents in vec(any::<u8>(), 0..100)
    ) {
        let dir = TempDir::new().unwrap();
        let store = ObjectStore::new(dir.path());
        
        let blob_id = store_blob(&store, &file_contents);
        let tree_id = create_tree(&store, vec![("file.txt", blob_id)]);
        
        let diff = diff_trees(&store, Some(&tree_id), &tree_id).unwrap();
        prop_assert!(diff.changes.is_empty(), "Identical trees should have no diff");
    }
}

#[test]
fn test_merge_base() {
    let dir = TempDir::new().unwrap();
    let store = ObjectStore::new(dir.path());
    
    // A -> B -> C (head)
    //  \-> D (other)
    let empty_tree = create_tree(&store, vec![]);
    
    let commit_a = store.write(&Object::Commit(Commit {
        tree_hash: empty_tree, parent_hashes: vec![], author: "".into(), committer: "".into(), timestamp: 0, message: "".into()
    })).unwrap();
    
    let commit_b = store.write(&Object::Commit(Commit {
        tree_hash: empty_tree, parent_hashes: vec![commit_a], author: "".into(), committer: "".into(), timestamp: 0, message: "".into()
    })).unwrap();
    
    let commit_c = store.write(&Object::Commit(Commit {
        tree_hash: empty_tree, parent_hashes: vec![commit_b], author: "".into(), committer: "".into(), timestamp: 0, message: "".into()
    })).unwrap();
    
    let commit_d = store.write(&Object::Commit(Commit {
        tree_hash: empty_tree, parent_hashes: vec![commit_a], author: "".into(), committer: "".into(), timestamp: 0, message: "different".into()
    })).unwrap();
    
    let base = rune_core::merge::find_merge_base(&store, &commit_c, &commit_d).unwrap();
    assert_eq!(base, commit_a);
}

#[test]
fn test_merge_commits_conflict() {
    let dir = TempDir::new().unwrap();
    let store = ObjectStore::new(dir.path());
    
    let base_blob = store_blob(&store, b"base");
    let base_tree = create_tree(&store, vec![("file.txt", base_blob)]);
    let base_commit = store.write(&Object::Commit(Commit {
        tree_hash: base_tree, parent_hashes: vec![], author: "".into(), committer: "".into(), timestamp: 0, message: "".into()
    })).unwrap();
    
    let head_blob = store_blob(&store, b"head");
    let head_tree = create_tree(&store, vec![("file.txt", head_blob)]);
    let head_commit = store.write(&Object::Commit(Commit {
        tree_hash: head_tree, parent_hashes: vec![base_commit], author: "".into(), committer: "".into(), timestamp: 0, message: "".into()
    })).unwrap();
    
    let other_blob = store_blob(&store, b"other");
    let other_tree = create_tree(&store, vec![("file.txt", other_blob)]);
    let other_commit = store.write(&Object::Commit(Commit {
        tree_hash: other_tree, parent_hashes: vec![base_commit], author: "".into(), committer: "".into(), timestamp: 0, message: "".into()
    })).unwrap();
    
    let result = rune_core::merge::merge_commits(&store, &base_commit, &head_commit, &other_commit).unwrap();
    
    assert!(result.add_or_modify.is_empty());
    assert!(result.delete.is_empty());
    assert_eq!(result.conflicts.len(), 1);
    
    let conflict = result.conflicts.get("file.txt").unwrap();
    let conflict_str = String::from_utf8_lossy(conflict);
    
    assert!(conflict_str.contains("<<<<<<< HEAD"));
    assert!(conflict_str.contains("head"));
    assert!(conflict_str.contains("======="));
    assert!(conflict_str.contains("other"));
    assert!(conflict_str.contains(">>>>>>> OTHER"));
}
