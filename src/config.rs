use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const HISTORY_MAX: usize = 50;

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

pub fn history_path() -> PathBuf {
    let base = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base)
        .join(".config")
        .join("githut")
        .join("history")
}

pub fn load_history() -> Vec<String> {
    let path = history_path();
    if !path.exists() {
        return Vec::new();
    }
    std::fs::read_to_string(&path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.to_string())
        .collect()
}

pub fn save_history(history: &[String]) {
    let path = history_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let content = history
        .iter()
        .take(HISTORY_MAX)
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::write(&path, content);
}

pub fn push_history(query: &str) {
    if query.trim().is_empty() {
        return;
    }
    let mut history = load_history();
    history.retain(|h| h != query);
    history.insert(0, query.to_string());
    history.truncate(HISTORY_MAX);
    save_history(&history);
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
