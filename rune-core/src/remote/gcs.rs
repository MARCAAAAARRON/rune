use std::collections::HashMap;
use anyhow::{Result, Context};
use async_trait::async_trait;
use std::str::FromStr;
use bytes::Bytes;
use futures::stream::StreamExt;

use object_store::{ObjectStore, path::Path as StorePath};
use object_store::gcp::GoogleCloudStorageBuilder;
use object_store::{PutMode, PutOptions, UpdateVersion};
use object_store::ObjectStoreExt;

use crate::object::ObjectId;
use crate::remote::Remote;

pub struct GcsRemote {
    client: Box<dyn ObjectStore>,
}

impl GcsRemote {
    pub fn new(bucket: &str) -> Result<Self> {
        let builder = GoogleCloudStorageBuilder::new()
            .with_bucket_name(bucket);
            
        // This automatically uses Application Default Credentials 
        // (gcloud auth application-default login)
        let client = builder.build()
            .context("Failed to build GCS client. Ensure you have run `gcloud auth application-default login`")?;
            
        Ok(Self {
            client: Box::new(client),
        })
    }

    pub fn with_client(client: Box<dyn ObjectStore>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Remote for GcsRemote {
    async fn list_refs(&self) -> Result<HashMap<String, ObjectId>> {
        let prefix = StorePath::from("refs/heads");
        let mut refs = HashMap::new();
        
        let mut list_stream = self.client.list(Some(&prefix));
        while let Some(meta) = list_stream.next().await {
            let meta = meta?;
            let path_str = meta.location.as_ref();
            if let Some(name) = path_str.strip_prefix("refs/heads/") {
                let data = self.client.get(&meta.location).await?.bytes().await?;
                let content = String::from_utf8_lossy(&data);
                if let Ok(hash) = ObjectId::from_str(content.trim()) {
                    refs.insert(name.to_string(), hash);
                }
            }
        }
        Ok(refs)
    }

    async fn has_object(&self, id: &ObjectId) -> Result<bool> {
        let hash_str = id.to_string();
        let path = StorePath::from(format!("objects/{}/{}", &hash_str[0..2], &hash_str[2..]));
        match self.client.head(&path).await {
            Ok(_) => Ok(true),
            Err(object_store::Error::NotFound { .. }) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    async fn fetch_object(&self, id: &ObjectId) -> Result<Vec<u8>> {
        let hash_str = id.to_string();
        let path = StorePath::from(format!("objects/{}/{}", &hash_str[0..2], &hash_str[2..]));
        let result = self.client.get(&path).await?.bytes().await?;
        Ok(result.to_vec())
    }

    async fn upload_object(&self, id: &ObjectId, data: &[u8]) -> Result<()> {
        let hash_str = id.to_string();
        let path = StorePath::from(format!("objects/{}/{}", &hash_str[0..2], &hash_str[2..]));
        
        // Skip if exists
        if self.has_object(id).await? {
            return Ok(());
        }
        
        self.client.put(&path, Bytes::copy_from_slice(data).into()).await?;
        Ok(())
    }

    async fn update_ref(&self, name: &str, expected_old: Option<ObjectId>, new: ObjectId) -> Result<()> {
        let path = StorePath::from(format!("refs/heads/{}", name));
        
        // Find expected generation for CAS
        let expected_version = if let Some(expected_id) = expected_old {
            match self.client.head(&path).await {
                Ok(meta) => {
                    let data = self.client.get(&path).await?.bytes().await?;
                    let content = String::from_utf8_lossy(&data);
                    if let Ok(hash) = ObjectId::from_str(content.trim()) {
                        if hash != expected_id {
                            anyhow::bail!("Ref update failed: expected {}, found {}", expected_id, hash);
                        }
                    }
                    if let Some(etag) = meta.e_tag {
                        Some(etag)
                    } else {
                        anyhow::bail!("No ETag found on object for CAS");
                    }
                }
                Err(object_store::Error::NotFound { .. }) => {
                    anyhow::bail!("Ref update failed: expected {:?}, but it doesn't exist", expected_old);
                }
                Err(e) => return Err(e.into()),
            }
        } else {
            None // Must not exist
        };

        let data = format!("{}\n", new);
        let bytes = Bytes::from(data);

        let put_opts = match expected_version {
            Some(etag) => PutOptions {
                mode: PutMode::Update(UpdateVersion {
                    e_tag: Some(etag),
                    version: None,
                }),
                tags: Default::default(),
                attributes: Default::default(),
                extensions: Default::default(),
            },
            None => PutOptions {
                mode: PutMode::Create,
                tags: Default::default(),
                attributes: Default::default(),
                extensions: Default::default(),
            },
        };

        self.client.put_opts(&path, bytes.into(), put_opts).await?;
        Ok(())
    }
}
