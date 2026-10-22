use std::path::{Path, PathBuf};
use std::fs;
use crate::object::{Object, ObjectId};
use crate::codec;
use crate::error::{StoreError, InitError};

pub struct ObjectStore {
    base_path: PathBuf,
}

impl ObjectStore {
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Self {
        Self {
            base_path: repo_path.as_ref().join(".rune").join("objects"),
        }
    }
    
    fn object_path(&self, id: &ObjectId) -> PathBuf {
        let hex = id.to_string();
        self.base_path.join(&hex[0..2]).join(&hex[2..])
    }
    
    pub fn write(&self, object: &Object) -> Result<ObjectId, StoreError> {
        let encoded = codec::encode(object);
        let hash = codec::hash_bytes(&encoded);
        
        let path = self.object_path(&hash);
        if path.exists() {
            // Deduplication: identical content -> same hash -> already exists
            return Ok(hash);
        }
        
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let compressed = codec::compress(&encoded).map_err(StoreError::Validation)?;
        fs::write(path, compressed)?;
        
        Ok(hash)
    }
    
    pub fn read(&self, id: &ObjectId) -> Result<Object, StoreError> {
        let path = self.object_path(id);
        if !path.exists() {
            return Err(StoreError::NotFound(id.to_string()));
        }
        
        let compressed = fs::read(path)?;
        let encoded = codec::decompress(&compressed).map_err(StoreError::Validation)?;
        
        // TRD §9.2: Object substitution / hash mismatch
        let computed_hash = codec::hash_bytes(&encoded);
        if &computed_hash != id {
            return Err(StoreError::Validation(crate::error::ObjectError::HashMismatch {
                expected: id.to_string(),
                actual: computed_hash.to_string(),
            }));
        }
        
        codec::decode(&encoded).map_err(StoreError::Validation)
    }
    
    pub fn exists(&self, id: &ObjectId) -> bool {
        self.object_path(id).exists()
    }
}

pub fn init<P: AsRef<Path>>(repo_path: P) -> Result<(), InitError> {
    let rune_dir = repo_path.as_ref().join(".rune");
    if rune_dir.exists() {
        return Err(InitError::AlreadyExists);
    }
    
    fs::create_dir_all(rune_dir.join("objects"))?;
    fs::create_dir_all(rune_dir.join("refs").join("heads"))?;
    fs::write(rune_dir.join("HEAD"), "ref: refs/heads/main\n")?;
    fs::write(rune_dir.join("config"), "")?; // Empty for now, will be JSON later
    
    Ok(())
}
