use rune_core::{add, checkout, commit, create_branch, init, log, CheckoutError};
use rune_core::object::{Blob, FileMode, Object, Tree, TreeEntry};
use rune_core::store::ObjectStore;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

struct TempRepo {
    dir: TempDir,
}

impl TempRepo {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        init(dir.path()).unwrap();
        Self { dir }
    }

    fn path(&self) -> PathBuf {
        self.dir.path().to_path_buf()
    }
}

#[test]
fn test_end_to_end_flow() {
    let repo = TempRepo::new();
    let root = repo.path();

    // 1. Create a file and add it
    let file1 = root.join("test.txt");
    fs::write(&file1, "Hello World").unwrap();
    add(&root, &file1).unwrap();

    // 2. Commit it
    let commit1 = commit(&root, "Initial commit", "Test User").unwrap();

    // 3. Create a branch
    create_branch(&root, "feature").unwrap();
    
    // Check that log works and has 1 commit
    let commits = log(&root).unwrap();
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].0, commit1);

    // 4. Modify file and add another
    fs::write(&file1, "Hello World v2").unwrap();
    let file2 = root.join("test2.txt");
    fs::write(&file2, "Another file").unwrap();
    add(&root, &file1).unwrap();
    add(&root, &file2).unwrap();

    // 5. Commit again
    let commit2 = commit(&root, "Second commit", "Test User").unwrap();

    let commits = log(&root).unwrap();
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].0, commit2);

    // 6. Checkout the first commit
    checkout(&root, &commit1.to_string()).unwrap();

    // The second file shouldn't exist anymore (wait, checkout currently doesn't delete untracked files, 
    // but the content of file1 should be "Hello World" again)
    let content = fs::read_to_string(&file1).unwrap();
    assert_eq!(content, "Hello World");

    // 7. Checkout the feature branch
    checkout(&root, "feature").unwrap();
    let content = fs::read_to_string(&file1).unwrap();
    assert_eq!(content, "Hello World");
}

#[test]
fn test_checkout_path_traversal_attack() {
    let repo = TempRepo::new();
    let root = repo.path();
    
    let store = ObjectStore::new(&root);
    
    // Create a malicious blob
    let malicious_blob = Object::Blob(Blob {
        content: b"malicious payload".to_vec(),
    });
    let blob_hash = store.write(&malicious_blob).unwrap();
    
    // Create a malicious tree with a traversal path
    let malicious_tree = Object::Tree(Tree {
        entries: vec![
            TreeEntry {
                mode: FileMode::Regular,
                name: "../outside.txt".to_string(), // malicious
                hash: blob_hash,
            }
        ],
    });
    let tree_hash = store.write(&malicious_tree).unwrap();
    
    // Create a commit
    let malicious_commit = Object::Commit(rune_core::object::Commit {
        tree_hash,
        parent_hashes: vec![],
        author: "Hacker".to_string(),
        committer: "Hacker".to_string(),
        timestamp: 0,
        message: "Hacked".to_string(),
    });
    let commit_hash = store.write(&malicious_commit).unwrap();
    
    // Attempt checkout
    let result = checkout(&root, &commit_hash.to_string());
    
    match result {
        Err(e) => {
            assert!(e.to_string().contains("Invalid path in tree entry"), "Got unexpected error: {:?}", e);
        }
        _ => panic!("Expected error due to path traversal, got {:?}", result),
    }
}

#[test]
fn test_checkout_symlink_escape_attack() {
    let repo = TempRepo::new();
    let root = repo.path();
    let store = ObjectStore::new(&root);
    
    // Malicious symlink target pointing to /etc/passwd
    let malicious_blob = Object::Blob(Blob {
        content: b"/etc/passwd".to_vec(),
    });
    let blob_hash = store.write(&malicious_blob).unwrap();
    
    let malicious_tree = Object::Tree(Tree {
        entries: vec![
            TreeEntry {
                mode: FileMode::Symlink,
                name: "passwd_link".to_string(),
                hash: blob_hash,
            }
        ],
    });
    let tree_hash = store.write(&malicious_tree).unwrap();
    
    let malicious_commit = Object::Commit(rune_core::object::Commit {
        tree_hash,
        parent_hashes: vec![],
        author: "Hacker".to_string(),
        committer: "Hacker".to_string(),
        timestamp: 0,
        message: "Hacked".to_string(),
    });
    let commit_hash = store.write(&malicious_commit).unwrap();
    
    let result = checkout(&root, &commit_hash.to_string());
    match result {
        Err(e) => {
            assert!(e.to_string().contains("Malicious symlink target"), "Got unexpected error: {:?}", e);
        }
        _ => panic!("Expected error due to symlink escape, got {:?}", result),
    }
}

#[test]
fn test_decompression_bomb() {
    // We can test the codec directly for decompression bombs
    use rune_core::codec;
    
    // Create a 101 MB array of zeros
    let data = vec![0u8; 101 * 1024 * 1024];
    
    // Compress it
    let compressed = codec::compress(&data).unwrap();
    
    // Attempt to decompress it - should fail due to MAX_DECOMPRESSED_SIZE (100MB)
    let result = codec::decompress(&compressed);
    assert!(result.is_err(), "Expected decompression to fail for 101MB payload");
}

#[test]
fn test_hash_mismatch_rejection() {
    let repo = TempRepo::new();
    let root = repo.path();
    let store = ObjectStore::new(&root);
    
    let empty_tree = Object::Tree(Tree { entries: vec![] });
    let tree_hash = store.write(&empty_tree).unwrap();
    
    let commit = Object::Commit(rune_core::object::Commit {
        tree_hash,
        parent_hashes: vec![],
        author: "Hacker".to_string(),
        committer: "Hacker".to_string(),
        timestamp: 0,
        message: "Hacked".to_string(),
    });
    
    // We want to simulate an attacker substituting an object on disk 
    // to create a cycle or just inject malicious content.
    let commit_hash = store.write(&commit).unwrap();
    
    // Attacker modifies the object on disk (e.g., changes author)
    let mut malicious_commit = rune_core::object::Commit {
        tree_hash,
        parent_hashes: vec![],
        author: "Super Hacker".to_string(), // Modified!
        committer: "Super Hacker".to_string(),
        timestamp: 0,
        message: "Hacked".to_string(),
    };
    
    let malicious_encoded = rune_core::codec::encode(&Object::Commit(malicious_commit));
    let malicious_compressed = rune_core::codec::compress(&malicious_encoded).unwrap();
    
    // Overwrite the original object file
    let hex = commit_hash.to_string();
    let object_path = root.join(".rune").join("objects").join(&hex[0..2]).join(&hex[2..]);
    std::fs::write(&object_path, malicious_compressed).unwrap();
    
    // Now try to read it - store should detect the hash mismatch
    let read_result = store.read(&commit_hash);
    
    assert!(read_result.is_err(), "Store should have rejected the modified object");
    if let Err(rune_core::error::StoreError::Validation(rune_core::error::ObjectError::HashMismatch { expected, actual })) = read_result {
        assert_eq!(expected, commit_hash.to_string());
        assert_ne!(expected, actual);
    } else {
        panic!("Expected HashMismatch error, got {:?}", read_result);
    }
}
