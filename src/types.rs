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
    ViewingIssues, // browsing issues/PRs list for a repo
    ViewingIssue,  // reading a single issue + comments
    CreatingIssue, // composing a new issue (title input)
    ViewingNotifications,
    SearchingCode,      // typing a code search query within a repo
    ViewingCodeResults, // browsing code search results
    CreatingRepo,       // new repo overlay
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

#[derive(Debug, Clone)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub state: String, // "open" | "closed"
    pub user_login: String,
    pub body: Option<String>,
    pub comments: u64,
    pub created_at: String,
    pub html_url: String,
    pub pull_request: bool, // true if this is actually a PR
    pub labels: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct IssueComment {
    pub id: u64,
    pub user_login: String,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub id: String,
    pub repo_full_name: String,
    pub subject_title: String,
    pub subject_type: String, // "Issue", "PullRequest", "Release", etc.
    pub reason: String,       // "mention", "subscribed", "review_requested", etc.
    pub unread: bool,
    pub updated_at: String,
    pub subject_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CodeResult {
    pub path: String,
    pub repo_full_name: String,
    pub html_url: String,
    pub sha: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IssueFilter {
    Open,
    Closed,
    All,
}

impl IssueFilter {
    pub fn as_str(&self) -> &'static str {
        match self {
            IssueFilter::Open => "open",
            IssueFilter::Closed => "closed",
            IssueFilter::All => "all",
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            IssueFilter::Open => "Open",
            IssueFilter::Closed => "Closed",
            IssueFilter::All => "All",
        }
    }
    pub fn next(&self) -> Self {
        match self {
            IssueFilter::Open => IssueFilter::Closed,
            IssueFilter::Closed => IssueFilter::All,
            IssueFilter::All => IssueFilter::Open,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum IssueTab {
    Issues,
    PullRequests,
}
