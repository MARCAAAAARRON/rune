use thiserror::Error;

#[derive(Error, Debug)]
pub enum ObjectError {
    #[error("Invalid hash format: {0}")]
    InvalidHash(String),
    
    #[error("Failed to decode object: {0}")]
    DecodeError(String),
    
    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch {
        expected: String,
        actual: String,
    },
    
    #[error("Decompression failed or output was too large")]
    DecompressionError,
}

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Object not found: {0}")]
    NotFound(String),
    
    #[error("Object validation failed: {0}")]
    Validation(#[from] ObjectError),
}

#[derive(Error, Debug)]
pub enum InitError {
    #[error("Repository already exists at this path")]
    AlreadyExists,
    
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum AddError {
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Store Error: {0}")]
    Store(#[from] StoreError),
}

#[derive(Error, Debug)]
pub enum RefsError {
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Invalid reference name: {0}")]
    InvalidName(String),
    
    #[error("Branch not found: {0}")]
    NotFound(String),
}

#[derive(Error, Debug)]
pub enum CommitError {
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Store Error: {0}")]
    Store(#[from] StoreError),
    
    #[error("Refs Error: {0}")]
    Refs(#[from] RefsError),
    
    #[error("Nothing to commit")]
    NothingToCommit,
}
