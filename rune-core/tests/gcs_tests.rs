use rune_core::object::ObjectId;
use rune_core::remote::Remote;
use rune_core::remote::gcs::GcsRemote;
use object_store::memory::InMemory;

#[tokio::test]
async fn test_gcs_remote_update_ref_concurrency() {
    let client = Box::new(InMemory::new());
    let remote = GcsRemote::with_client(client);
    
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
    
    // Assert Bob's error contains expected error details
    let err_str = bob_result.unwrap_err().to_string();
    assert!(err_str.contains("Precondition failed") || err_str.contains("Ref update failed") || err_str.contains("Update failed"));
    
    let refs = remote.list_refs().await.unwrap();
    assert_eq!(refs.get("main"), Some(&obj_b));
}
