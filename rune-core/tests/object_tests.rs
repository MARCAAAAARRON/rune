use rune_core::object::{Blob, Commit, FileMode, Object, ObjectId, Tree, TreeEntry};
use rune_core::codec;
use rune_core::store::ObjectStore;
use rune_core::init;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_blob_codec_roundtrip() {
    let blob = Object::Blob(Blob {
        content: b"Hello, world!".to_vec(),
    });

    let encoded = codec::encode(&blob);
    let decoded = codec::decode(&encoded).unwrap();

    assert_eq!(blob, decoded);
}

#[test]
fn test_tree_codec_roundtrip() {
    let tree = Object::Tree(Tree {
        entries: vec![
            TreeEntry {
                mode: FileMode::Regular,
                name: "file.txt".to_string(),
                hash: codec::hash_bytes(b"dummy1"),
            },
            TreeEntry {
                mode: FileMode::Directory,
                name: "src".to_string(),
                hash: codec::hash_bytes(b"dummy2"),
            },
        ],
    });

    let encoded = codec::encode(&tree);
    let decoded = codec::decode(&encoded).unwrap();

    assert_eq!(tree, decoded);
}

#[test]
fn test_commit_codec_roundtrip() {
    let commit = Object::Commit(Commit {
        tree_hash: codec::hash_bytes(b"dummy_tree"),
        parent_hashes: vec![codec::hash_bytes(b"dummy_parent")],
        author: "Alice <alice@example.com>".to_string(),
        committer: "Bob <bob@example.com>".to_string(),
        timestamp: 1672531200,
        message: "Initial commit".to_string(),
    });

    let encoded = codec::encode(&commit);
    let decoded = codec::decode(&encoded).unwrap();

    assert_eq!(commit, decoded);
}

#[test]
fn test_hash_stability() {
    let blob = Object::Blob(Blob {
        content: b"Stable content".to_vec(),
    });
    
    let encoded = codec::encode(&blob);
    let hash1 = codec::hash_bytes(&encoded);
    let hash2 = codec::hash_bytes(&encoded);
    
    assert_eq!(hash1, hash2);
    
    // Changing content slightly should change hash
    let blob2 = Object::Blob(Blob {
        content: b"Stable content!".to_vec(),
    });
    let hash3 = codec::hash_bytes(&codec::encode(&blob2));
    assert_ne!(hash1, hash3);
}

#[test]
fn test_store_dedup_and_read() {
    let dir = tempdir().unwrap();
    init(dir.path()).unwrap();
    
    let store = ObjectStore::new(dir.path());
    let blob = Object::Blob(Blob {
        content: b"Test data".to_vec(),
    });
    
    let hash1 = store.write(&blob).unwrap();
    let hash2 = store.write(&blob).unwrap(); // Write same object again
    
    assert_eq!(hash1, hash2); // Deduplication means same hash
    
    let read_obj = store.read(&hash1).unwrap();
    assert_eq!(blob, read_obj);
}

#[test]
fn test_init_command() {
    let dir = tempdir().unwrap();
    init(dir.path()).unwrap();
    
    let rune_dir = dir.path().join(".rune");
    assert!(rune_dir.join("objects").exists());
    assert!(rune_dir.join("refs").join("heads").exists());
    
    let head = fs::read_to_string(rune_dir.join("HEAD")).unwrap();
    assert_eq!(head, "ref: refs/heads/main\n");
    
    assert!(rune_dir.join("config").exists());
    
    // Double init should fail
    assert!(init(dir.path()).is_err());
}
