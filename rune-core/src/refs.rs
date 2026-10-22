use crate::object::ObjectId;
use std::fs;
use std::io;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeadRef {
    Detached(ObjectId),
    Branch(String),
}

pub fn read_head<P: AsRef<Path>>(repo_path: P) -> io::Result<HeadRef> {
    let head_path = repo_path.as_ref().join(".rune").join("HEAD");
    let content = fs::read_to_string(head_path)?;
    let content = content.trim();

    if let Some(branch_name) = content.strip_prefix("ref: refs/heads/") {
        Ok(HeadRef::Branch(branch_name.to_string()))
    } else {
        match ObjectId::from_str(content) {
            Ok(hash) => Ok(HeadRef::Detached(hash)),
            Err(_) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid HEAD content",
            )),
        }
    }
}

pub fn write_head<P: AsRef<Path>>(repo_path: P, head_ref: &HeadRef) -> io::Result<()> {
    let head_path = repo_path.as_ref().join(".rune").join("HEAD");
    let content = match head_ref {
        HeadRef::Detached(hash) => format!("{}\n", hash),
        HeadRef::Branch(name) => format!("ref: refs/heads/{}\n", name),
    };
    fs::write(head_path, content)
}

pub fn read_ref<P: AsRef<Path>>(repo_path: P, name: &str) -> io::Result<Option<ObjectId>> {
    let ref_path = repo_path
        .as_ref()
        .join(".rune")
        .join("refs")
        .join("heads")
        .join(name);
    
    if !ref_path.exists() {
        return Ok(None);
    }
    
    let content = fs::read_to_string(ref_path)?;
    let content = content.trim();
    match ObjectId::from_str(content) {
        Ok(hash) => Ok(Some(hash)),
        Err(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid ref content",
        )),
    }
}

pub fn write_ref<P: AsRef<Path>>(repo_path: P, name: &str, hash: &ObjectId) -> io::Result<()> {
    let refs_dir = repo_path
        .as_ref()
        .join(".rune")
        .join("refs")
        .join("heads");
    
    let ref_path = refs_dir.join(name);
    if let Some(p) = ref_path.parent() {
        fs::create_dir_all(p)?;
    }
    
    fs::write(ref_path, format!("{}\n", hash))
}

pub fn delete_ref<P: AsRef<Path>>(repo_path: P, name: &str) -> io::Result<()> {
    let ref_path = repo_path
        .as_ref()
        .join(".rune")
        .join("refs")
        .join("heads")
        .join(name);
        
    if ref_path.exists() {
        fs::remove_file(ref_path)
    } else {
        Err(io::Error::new(io::ErrorKind::NotFound, "Branch not found"))
    }
}

pub fn list_refs<P: AsRef<Path>>(repo_path: P) -> io::Result<Vec<String>> {
    let refs_dir = repo_path
        .as_ref()
        .join(".rune")
        .join("refs")
        .join("heads");
        
    if !refs_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut branches = Vec::new();
    for entry in fs::read_dir(refs_dir)? {
        let entry = entry?;
        if let Some(name) = entry.file_name().to_str() {
            branches.push(name.to_string());
        }
    }
    
    branches.sort();
    Ok(branches)
}

pub fn list_remote_refs<P: AsRef<Path>>(repo_path: P) -> io::Result<Vec<(String, String)>> {
    let refs_dir = repo_path
        .as_ref()
        .join(".rune")
        .join("refs")
        .join("remotes");
        
    if !refs_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut branches = Vec::new();
    for remote_entry in fs::read_dir(refs_dir)? {
        let remote_entry = remote_entry?;
        if let Some(remote_name) = remote_entry.file_name().to_str() {
            let remote_dir = remote_entry.path();
            if remote_dir.is_dir() {
                for branch_entry in fs::read_dir(remote_dir)? {
                    let branch_entry = branch_entry?;
                    if let Some(branch_name) = branch_entry.file_name().to_str() {
                        branches.push((remote_name.to_string(), branch_name.to_string()));
                    }
                }
            }
        }
    }
    
    branches.sort();
    Ok(branches)
}
