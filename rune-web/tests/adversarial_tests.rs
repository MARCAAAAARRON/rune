use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use rune_core::{
    object::{Blob, Commit, FileMode, Object, Tree, TreeEntry},
    store::ObjectStore,
};
use rune_web::app;
use std::path::PathBuf;
use tempfile::TempDir;
use tower::ServiceExt;

struct AdversarialRepo {
    dir: TempDir,
    store: ObjectStore,
}

impl AdversarialRepo {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        rune_core::init(dir.path()).unwrap();
        let store = ObjectStore::new(dir.path());
        Self { dir, store }
    }

    fn generate_path_traversal_commit(&self) -> String {
        let normal_blob = Object::Blob(Blob {
            content: b"Hello".to_vec(),
        });
        let normal_blob_hash = self.store.write(&normal_blob).unwrap();

        let malicious_tree = Object::Tree(Tree {
            entries: vec![
                TreeEntry {
                    mode: FileMode::Regular,
                    name: "../etc/passwd".to_string(),
                    hash: normal_blob_hash,
                },
            ],
        });
        // We write the bytes directly to bypass any ObjectStore validation if it exists on write.
        // Wait, store.write doesn't re-validate if it's already an Object, but reading it might fail.
        let tree_hash = self.store.write(&malicious_tree).unwrap();

        let malicious_commit = Object::Commit(Commit {
            tree_hash,
            parent_hashes: vec![],
            author: "Attacker".to_string(),
            committer: "Attacker".to_string(),
            timestamp: 0,
            message: "Path traversal".to_string(),
        });
        self.store.write(&malicious_commit).unwrap().to_string()
    }

    fn generate_big_blob_commit(&self) -> String {
        let big_blob = Object::Blob(Blob {
            content: vec![b'A'; 3 * 1024 * 1024],
        });
        let big_blob_hash = self.store.write(&big_blob).unwrap();

        let tree = Object::Tree(Tree {
            entries: vec![
                TreeEntry {
                    mode: FileMode::Regular,
                    name: "big.txt".to_string(),
                    hash: big_blob_hash,
                },
            ],
        });
        let tree_hash = self.store.write(&tree).unwrap();

        let commit = Object::Commit(Commit {
            tree_hash,
            parent_hashes: vec![],
            author: "Attacker".to_string(),
            committer: "Attacker".to_string(),
            timestamp: 0,
            message: "Big blob".to_string(),
        });
        self.store.write(&commit).unwrap().to_string()
    }
}

#[tokio::test]
async fn test_path_traversal_rejection() {
    let repo = AdversarialRepo::new();
    let commit_hash = repo.generate_path_traversal_commit();

    let router = app(repo.dir.path().to_path_buf());

    // Attempt to access the path traversal file
    let req = Request::builder()
        .uri(format!("/commit/{}/blob/../etc/passwd", commit_hash))
        .header("Host", "127.0.0.1:8080")
        .body(Body::empty())
        .unwrap();

    let res = router.oneshot(req).await.unwrap();
    
    // Axum router itself will collapse `/../`, but if not, our validate_path will catch it.
    // Let's also check if we manually pass it. Wait, the URL parsing might collapse `../`.
    // Let's pass `%2E%2E%2Fetc%2Fpasswd` to prevent client collapsing.
    assert_ne!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_url_encoded_path_traversal() {
    let repo = AdversarialRepo::new();
    let commit_hash = repo.generate_path_traversal_commit();

    let router = app(repo.dir.path().to_path_buf());

    let req = Request::builder()
        .uri(format!("/commit/{}/blob/%2E%2E/etc/passwd", commit_hash))
        .header("Host", "127.0.0.1:8080")
        .body(Body::empty())
        .unwrap();

    let res = router.oneshot(req).await.unwrap();
    // It should be forbidden or not found, not 200 OK.
    assert_ne!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_oversized_blob_rendering_rejection() {
    let repo = AdversarialRepo::new();
    let commit_hash = repo.generate_big_blob_commit();

    let router = app(repo.dir.path().to_path_buf());

    let req = Request::builder()
        .uri(format!("/commit/{}/blob/big.txt", commit_hash))
        .header("Host", "127.0.0.1:8080")
        .body(Body::empty())
        .unwrap();

    let res = router.oneshot(req).await.unwrap();
    let status = res.status();
    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(body_bytes.to_vec()).unwrap();
    
    assert_eq!(status, StatusCode::OK, "Failed with {}: {}", status, body);
    
    assert!(body.contains("File too large to display"));
}
