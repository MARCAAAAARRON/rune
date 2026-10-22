use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::str::FromStr;

use crate::object::ObjectId;
use crate::remote::Remote;

pub struct LocalRemote {
    path: PathBuf,
}

impl LocalRemote {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        fs::create_dir_all(path.join(".rune").join("objects"))?;
        fs::create_dir_all(path.join(".rune").join("refs").join("heads"))?;
        Ok(Self { path })
    }
    
    fn object_path(&self, id: &ObjectId) -> PathBuf {
        let hash_str = id.to_string();
        self.path.join(".rune").join("objects").join(&hash_str[0..2]).join(&hash_str[2..])
    }
    
    fn ref_path(&self, name: &str) -> PathBuf {
        self.path.join(".rune").join("refs").join("heads").join(name)
    }
}

#[async_trait]
impl Remote for LocalRemote {
    async fn list_refs(&self) -> Result<HashMap<String, ObjectId>> {
        let refs_dir = self.path.join(".rune").join("refs").join("heads");
        let mut refs = HashMap::new();
        
        if !refs_dir.exists() {
            return Ok(refs);
        }
        
        for entry in fs::read_dir(refs_dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            let content = fs::read_to_string(entry.path())?;
            let content = content.trim();
            if let Ok(hash) = ObjectId::from_str(content) {
                refs.insert(name, hash);
            }
        }
        
        Ok(refs)
    }
    
    async fn has_object(&self, id: &ObjectId) -> Result<bool> {
        Ok(self.object_path(id).exists())
    }
    
    async fn fetch_object(&self, id: &ObjectId) -> Result<Vec<u8>> {
        let path = self.object_path(id);
        fs::read(&path).with_context(|| format!("Object {} not found on remote", id))
    }
    
    async fn upload_object(&self, id: &ObjectId, data: &[u8]) -> Result<()> {
        let path = self.object_path(id);
        if path.exists() {
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, data)?;
        Ok(())
    }
    
    async fn update_ref(&self, name: &str, expected_old: Option<ObjectId>, new: ObjectId) -> Result<()> {
        let ref_path = self.ref_path(name);
        
        // Ensure atomic file creation by checking expected_old carefully
        let current = if ref_path.exists() {
            let content = fs::read_to_string(&ref_path)?;
            let content = content.trim();
            Some(ObjectId::from_str(content)?)
        } else {
            None
        };
        
        if current != expected_old {
            anyhow::bail!("Ref update failed: expected {:?}, found {:?}", expected_old, current);
        }
        
        if let Some(parent) = ref_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Write to temp and rename for atomicity
        let temp_path = ref_path.with_extension("tmp");
        fs::write(&temp_path, format!("{}\n", new))?;
        fs::rename(&temp_path, &ref_path)?;
        
        Ok(())
    }
}
