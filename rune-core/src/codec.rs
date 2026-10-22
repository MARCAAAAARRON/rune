use crate::object::{Blob, Commit, Object, ObjectId, ObjectType, Tree, TreeEntry, FileMode};
use crate::error::ObjectError;
use sha2::{Digest, Sha256};
use std::convert::TryInto;
use std::io::Read;

/// Compute the SHA-256 hash of the given bytes.
pub fn hash_bytes(data: &[u8]) -> ObjectId {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    ObjectId(result.into())
}

/// Encode an object to its binary representation: `<type> <length>\0<content>`
pub fn encode(object: &Object) -> Vec<u8> {
    let mut content = Vec::new();
    let obj_type = match object {
        Object::Blob(b) => {
            content.extend_from_slice(&b.content);
            ObjectType::Blob
        }
        Object::Tree(t) => {
            for entry in &t.entries {
                content.extend_from_slice(entry.mode.as_octal_str().as_bytes());
                content.push(b' ');
                content.extend_from_slice(entry.name.as_bytes());
                content.push(0);
                content.extend_from_slice(&entry.hash.0);
            }
            ObjectType::Tree
        }
        Object::Commit(c) => {
            content.extend_from_slice(b"tree ");
            content.extend_from_slice(c.tree_hash.to_string().as_bytes());
            content.push(b'\n');
            
            for p in &c.parent_hashes {
                content.extend_from_slice(b"parent ");
                content.extend_from_slice(p.to_string().as_bytes());
                content.push(b'\n');
            }
            
            content.extend_from_slice(b"author ");
            content.extend_from_slice(c.author.as_bytes());
            content.push(b'\n');
            
            content.extend_from_slice(b"committer ");
            content.extend_from_slice(c.committer.as_bytes());
            content.push(b'\n');
            
            content.extend_from_slice(b"timestamp ");
            content.extend_from_slice(c.timestamp.to_string().as_bytes());
            content.push(b'\n');
            
            content.push(b'\n');
            content.extend_from_slice(c.message.as_bytes());
            
            ObjectType::Commit
        }
    };
    
    let mut header = format!("{} {}\0", obj_type.as_str(), content.len()).into_bytes();
    header.append(&mut content);
    header
}

/// Decode binary representation into an Object
pub fn decode(bytes: &[u8]) -> Result<Object, ObjectError> {
    let space_idx = bytes.iter().position(|&b| b == b' ').ok_or_else(|| {
        ObjectError::DecodeError("Missing space in object header".into())
    })?;
    
    let null_idx = bytes.iter().position(|&b| b == 0).ok_or_else(|| {
        ObjectError::DecodeError("Missing null byte in object header".into())
    })?;
    
    if null_idx < space_idx {
        return Err(ObjectError::DecodeError("Null byte before space in header".into()));
    }
    
    let obj_type_str = std::str::from_utf8(&bytes[0..space_idx])
        .map_err(|_| ObjectError::DecodeError("Invalid UTF-8 in object type".into()))?;
        
    let length_str = std::str::from_utf8(&bytes[space_idx + 1..null_idx])
        .map_err(|_| ObjectError::DecodeError("Invalid UTF-8 in object length".into()))?;
        
    let expected_len: usize = length_str.parse().map_err(|_| {
        ObjectError::DecodeError("Invalid object length format".into())
    })?;
    
    let content = &bytes[null_idx + 1..];
    
    if content.len() != expected_len {
        return Err(ObjectError::DecodeError(format!(
            "Length mismatch: header says {}, actual is {}",
            expected_len,
            content.len()
        )));
    }
    
    match obj_type_str {
        "blob" => Ok(Object::Blob(Blob { content: content.to_vec() })),
        "tree" => decode_tree(content).map(Object::Tree),
        "commit" => decode_commit(content).map(Object::Commit),
        _ => Err(ObjectError::DecodeError(format!("Unknown object type: {}", obj_type_str))),
    }
}

fn decode_tree(content: &[u8]) -> Result<Tree, ObjectError> {
    let mut entries = Vec::new();
    let mut pos = 0;
    
    while pos < content.len() {
        let space_idx = content[pos..].iter().position(|&b| b == b' ')
            .ok_or_else(|| ObjectError::DecodeError("Tree entry missing space".into()))? + pos;
            
        let mode_str = std::str::from_utf8(&content[pos..space_idx])
            .map_err(|_| ObjectError::DecodeError("Invalid UTF-8 in mode".into()))?;
            
        let mode = match mode_str {
            "100644" => FileMode::Regular,
            "100755" => FileMode::Executable,
            "120000" => FileMode::Symlink,
            "40000" | "040000" => FileMode::Directory,
            _ => return Err(ObjectError::DecodeError(format!("Unknown mode: {}", mode_str))),
        };
        
        let null_idx = content[space_idx..].iter().position(|&b| b == 0)
            .ok_or_else(|| ObjectError::DecodeError("Tree entry missing null byte".into()))? + space_idx;
            
        let name = std::str::from_utf8(&content[space_idx + 1..null_idx])
            .map_err(|_| ObjectError::DecodeError("Invalid UTF-8 in tree entry name".into()))?
            .to_string();
            
        if name.contains('/') || name.contains('\\') || name == "." || name == ".." {
            return Err(ObjectError::DecodeError(format!("Invalid path in tree entry: {}", name)));
        }
            
        if null_idx + 1 + 32 > content.len() {
            return Err(ObjectError::DecodeError("Tree entry truncated".into()));
        }
        
        let hash_bytes: [u8; 32] = content[null_idx + 1..null_idx + 1 + 32].try_into().unwrap();
        let hash = ObjectId(hash_bytes);
        
        entries.push(TreeEntry { mode, name, hash });
        pos = null_idx + 1 + 32;
    }
    
    Ok(Tree { entries })
}

fn decode_commit(content: &[u8]) -> Result<Commit, ObjectError> {
    let content_str = std::str::from_utf8(content)
        .map_err(|_| ObjectError::DecodeError("Commit object not valid UTF-8".into()))?;
        
    let mut tree_hash = None;
    let mut parent_hashes = Vec::new();
    let mut author = None;
    let mut committer = None;
    let mut timestamp = None;
    
    let mut lines = content_str.split('\n');
    
    for line in lines.by_ref() {
        if line.is_empty() {
            break;
        }
        
        if let Some(rest) = line.strip_prefix("tree ") {
            tree_hash = Some(rest.parse().map_err(|_| ObjectError::DecodeError("Invalid tree hash".into()))?);
        } else if let Some(rest) = line.strip_prefix("parent ") {
            parent_hashes.push(rest.parse().map_err(|_| ObjectError::DecodeError("Invalid parent hash".into()))?);
        } else if let Some(rest) = line.strip_prefix("author ") {
            author = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("committer ") {
            committer = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("timestamp ") {
            timestamp = Some(rest.parse().map_err(|_| ObjectError::DecodeError("Invalid timestamp".into()))?);
        }
    }
    
    let message = lines.collect::<Vec<_>>().join("\n");
    
    Ok(Commit {
        tree_hash: tree_hash.ok_or_else(|| ObjectError::DecodeError("Missing tree hash".into()))?,
        parent_hashes,
        author: author.ok_or_else(|| ObjectError::DecodeError("Missing author".into()))?,
        committer: committer.ok_or_else(|| ObjectError::DecodeError("Missing committer".into()))?,
        timestamp: timestamp.ok_or_else(|| ObjectError::DecodeError("Missing timestamp".into()))?,
        message,
    })
}

const MAX_DECOMPRESSED_SIZE: usize = 100 * 1024 * 1024; // 100MB limit per TRD §9.4

pub fn compress(data: &[u8]) -> Result<Vec<u8>, ObjectError> {
    zstd::encode_all(data, 0).map_err(|_| ObjectError::DecompressionError)
}

pub fn decompress(data: &[u8]) -> Result<Vec<u8>, ObjectError> {
    let decoder = zstd::stream::Decoder::new(data).map_err(|_| ObjectError::DecompressionError)?;
    let mut out = Vec::new();
    
    // Read up to MAX_DECOMPRESSED_SIZE + 1 bytes to prevent OOM on zip bombs
    decoder.take((MAX_DECOMPRESSED_SIZE + 1) as u64)
        .read_to_end(&mut out)
        .map_err(|_| ObjectError::DecompressionError)?;
        
    if out.len() > MAX_DECOMPRESSED_SIZE {
        return Err(ObjectError::DecompressionError);
    }
    Ok(out)
}
