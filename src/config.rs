use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Default local path prefix for cloning repos.
    /// e.g. "~/repos" — clone of foo/bar goes to ~/repos/bar
    pub default_clone_path: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_clone_path: None,
        }
    }
}

pub fn config_path() -> PathBuf {
    let base = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base)
        .join(".config")
        .join("githut")
        .join("config.toml")
}

pub fn load() -> Result<Config> {
    let path = config_path();

    if !path.exists() {
        // create parent dirs + default config file
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let default = Config::default();
        let contents = format!(
            "# githut config\n\
             # default_clone_path = \"~/repos\"\n"
        );
        std::fs::write(&path, contents)?;
        return Ok(default);
    }

    let raw = std::fs::read_to_string(&path)?;
    let config: Config = toml::from_str(&raw)?;
    Ok(config)
}
