use tempfile::TempDir;
use rune_core::store::ObjectStore;
use rune_core::object::{Object, Blob, Commit, FileMode, Tree, TreeEntry, ObjectId};
use rune_core::remote::Remote;
use rune_core::remote::local::LocalRemote;
use rune_core::sync;
use std::collections::HashMap;

fn store_blob(store: &ObjectStore, content: &[u8]) -> ObjectId {
    store.write(&Object::Blob(Blob { content: content.to_vec() })).unwrap()
}

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

#[tokio::test]
async fn test_local_remote_update_ref_concurrency() {
    let remote_dir = TempDir::new().unwrap();
    let remote = LocalRemote::new(remote_dir.path()).unwrap();
    
    let obj_a = ObjectId([1; 32]);
    let obj_b = ObjectId([2; 32]);
    let obj_c = ObjectId([3; 32]);
    
    // Initial push (no expected_old)
    assert!(remote.update_ref("main", None, obj_a).await.is_ok());
    
    let refs = remote.list_refs().await.unwrap();
    assert_eq!(refs.get("main"), Some(&obj_a));
    
    // Concurrent pushes: Alice expects obj_a -> obj_b
    // Bob expects obj_a -> obj_c
    
    // Alice wins
    assert!(remote.update_ref("main", Some(obj_a), obj_b).await.is_ok());
    
    // Bob loses (CAS failure)
    let bob_result = remote.update_ref("main", Some(obj_a), obj_c).await;
    assert!(bob_result.is_err());
    
    let refs = remote.list_refs().await.unwrap();
    assert_eq!(refs.get("main"), Some(&obj_b));
}

#[tokio::test]
async fn test_sync_push_and_fetch() {
    let remote_dir = TempDir::new().unwrap();
    let remote = LocalRemote::new(remote_dir.path()).unwrap();
    
    let repo_dir = TempDir::new().unwrap();
    std::fs::create_dir_all(repo_dir.path().join(".rune").join("objects")).unwrap();
    std::fs::create_dir_all(repo_dir.path().join(".rune").join("refs").join("heads")).unwrap();
    
    let store = ObjectStore::new(repo_dir.path());
    let blob_id = store_blob(&store, b"hello world");
    let tree_id = create_tree(&store, vec![("hello.txt", blob_id)]);
    let commit_id = store.write(&Object::Commit(Commit {
        tree_hash: tree_id,
        parent_hashes: vec![],
        author: "Alice".into(),
        committer: "Alice".into(),
        timestamp: 0,
        message: "Initial commit".into(),
    })).unwrap();
    
    rune_core::refs::write_ref(repo_dir.path(), "main", &commit_id).unwrap();
    
    // Push
    sync::push(repo_dir.path(), &remote, "main").await.unwrap();
    
    // Verify remote has objects
    assert!(remote.has_object(&blob_id).await.unwrap());
    assert!(remote.has_object(&tree_id).await.unwrap());
    assert!(remote.has_object(&commit_id).await.unwrap());
    
    // Fetch into a new clone
    let clone_dir = TempDir::new().unwrap();
    std::fs::create_dir_all(clone_dir.path().join(".rune").join("objects")).unwrap();
    std::fs::create_dir_all(clone_dir.path().join(".rune").join("refs").join("heads")).unwrap();
    
    sync::fetch(clone_dir.path(), &remote, "main").await.unwrap();
    
    let clone_store = ObjectStore::new(clone_dir.path());
    assert!(clone_store.read(&blob_id).is_ok());
    
    // Fetch sets the remote tracking branch
    let fetch_head = rune_core::refs::read_ref(clone_dir.path(), "remotes/main").unwrap().unwrap();
    assert_eq!(fetch_head, commit_id);
}
