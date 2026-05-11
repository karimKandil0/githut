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
    pub archived: bool,
    pub fork: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Search,
    MyRepos,
}

#[derive(Debug, Clone)]
pub enum AppState {
    Searching,
    Browsing,
    Previewing,
    FileBrowsing,
    Cloning,
    SparseCloning,
    FileSaving,
    MyRepos,
    Renaming,
    ConfirmDelete,
    ViewingProfile,
    Error(String),
    Help,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SparseStep {
    Path,
    Dirs,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntryType {
    File,
    Dir,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub entry_type: EntryType,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub repos: Vec<Repo>,
    pub total_count: u64,
}

#[derive(Debug, Clone)]
pub struct UserProfile {
    pub login: String,
    pub name: Option<String>,
    pub bio: Option<String>,
    pub followers: u64,
    pub following: u64,
    pub public_repos: u64,
    pub html_url: String,
}

#[derive(Debug, Clone)]
pub struct RateLimit {
    pub search_remaining: u32,
    pub search_limit: u32,
    pub core_remaining: u32,
    pub core_limit: u32,
}
