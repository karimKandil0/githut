use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repo {
    pub id: u64,
    pub full_name: String,
    pub name: String,
    pub owner: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub stargazers_count: u64,
    pub forks_count: u64,
    pub html_url: String,
    pub clone_url: String,
    pub default_branch: String,
}

#[derive(Debug, Clone)]
pub enum AppState {
    Searching,
    Browsing,
    Previewing,
    Cloning,
    SparseCloning,
    Error(String),
    Help,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub repos: Vec<Repo>,
    pub total_count: u64,
}
