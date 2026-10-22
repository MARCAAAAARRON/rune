use crate::object::{FileMode, ObjectId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexEntry {
    pub path: String,
    pub hash: ObjectId,
    pub mode: FileMode,
    pub size: u64,
    pub mtime: u64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Index {
    pub entries: BTreeMap<String, IndexEntry>,
}

impl Index {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    pub fn load<P: AsRef<Path>>(repo_path: P) -> io::Result<Self> {
        let index_path = repo_path.as_ref().join(".rune").join("index");
        if !index_path.exists() {
            return Ok(Self::new());
        }

        let mut file = fs::File::open(index_path)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;

        bincode::deserialize(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn save<P: AsRef<Path>>(&self, repo_path: P) -> io::Result<()> {
        let index_path = repo_path.as_ref().join(".rune").join("index");
        let encoded =
            bincode::serialize(self).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let mut temp_path = index_path.clone();
        temp_path.set_extension("lock");

        let mut file = fs::File::create(&temp_path)?;
        file.write_all(&encoded)?;
        file.sync_all()?;

        fs::rename(temp_path, index_path)?;
        Ok(())
    }

    pub fn add_entry(&mut self, entry: IndexEntry) {
        self.entries.insert(entry.path.clone(), entry);
    }

    pub fn remove_entry(&mut self, path: &str) {
        self.entries.remove(path);
    }
}
