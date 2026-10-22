use std::fmt;
use std::str::FromStr;
use std::convert::TryInto;

use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct ObjectId(pub [u8; 32]);

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl FromStr for ObjectId {
    type Err = crate::error::ObjectError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 64 {
            return Err(crate::error::ObjectError::InvalidHash(
                "Hash must be exactly 64 hex characters".into(),
            ));
        }
        let bytes = hex::decode(s).map_err(|_| {
            crate::error::ObjectError::InvalidHash("Invalid hex characters".into())
        })?;
        Ok(ObjectId(bytes.try_into().unwrap()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    Blob,
    Tree,
    Commit,
}

impl ObjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ObjectType::Blob => "blob",
            ObjectType::Tree => "tree",
            ObjectType::Commit => "commit",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blob {
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileMode {
    Regular,    // 100644
    Executable, // 100755
    Symlink,    // 120000
    Directory,  // 040000
}

impl FileMode {
    pub fn as_octal_str(&self) -> &'static str {
        match self {
            FileMode::Regular => "100644",
            FileMode::Executable => "100755",
            FileMode::Symlink => "120000",
            FileMode::Directory => "40000",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    pub mode: FileMode,
    pub name: String,
    pub hash: ObjectId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tree {
    pub entries: Vec<TreeEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub tree_hash: ObjectId,
    pub parent_hashes: Vec<ObjectId>,
    pub author: String,
    pub committer: String,
    pub timestamp: u64,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Object {
    Blob(Blob),
    Tree(Tree),
    Commit(Commit),
}
