use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use anyhow::Result;
use std::fs;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Config {
    #[serde(default)]
    pub remotes: HashMap<String, String>,
}

impl Config {
    pub fn load(repo_path: &Path) -> Result<Self> {
        let config_path = repo_path.join(".rune").join("config");
        if !config_path.exists() {
            return Ok(Config::default());
        }
        
        let content = fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&content).unwrap_or_default();
        Ok(config)
    }

    pub fn save(&self, repo_path: &Path) -> Result<()> {
        let config_path = repo_path.join(".rune").join("config");
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string(self)?;
        fs::write(config_path, content)?;
        Ok(())
    }

    pub fn get_remote(&self, name: &str) -> Option<&String> {
        self.remotes.get(name)
    }

    pub fn set_remote(&mut self, name: &str, url: &str) {
        self.remotes.insert(name.to_string(), url.to_string());
    }
}
