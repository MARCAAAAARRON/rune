use crate::object::ObjectId;
use anyhow::Result;
use std::collections::HashMap;
use async_trait::async_trait;

pub mod local;
pub mod gcs;

#[async_trait]
pub trait Remote {
    /// List all references on the remote
    async fn list_refs(&self) -> Result<HashMap<String, ObjectId>>;
    
    /// Check if the remote has a given object
    async fn has_object(&self, id: &ObjectId) -> Result<bool>;
    
    /// Fetch an object from the remote
    async fn fetch_object(&self, id: &ObjectId) -> Result<Vec<u8>>;
    
    /// Upload an object to the remote
    async fn upload_object(&self, id: &ObjectId, data: &[u8]) -> Result<()>;
    
    /// Update a reference on the remote. 
    /// If `expected_old` is provided, it acts as a compare-and-swap mechanism to prevent concurrency issues.
    async fn update_ref(&self, name: &str, expected_old: Option<ObjectId>, new: ObjectId) -> Result<()>;
}
