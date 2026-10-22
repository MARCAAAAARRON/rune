use std::path::{Path, PathBuf};
use std::fs;
use tempfile::TempDir;
use rune_core::store;

pub struct TempRepo {
    pub dir: TempDir,
    pub path: PathBuf,
}

impl TempRepo {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_path_buf();
        
        // Temporarily change current dir or let init handle path.
        // For now, let's assume `store::init` takes a path, or operates on CWD.
        // Let's modify init to take an optional path for testing, or just use CWD and we'll have to change CWD in tests.
        // To be safe and clean, we should pass the repo path. Let's update `init` to take a path.
        
        Self { dir, path }
    }
}
