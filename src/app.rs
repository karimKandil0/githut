use std::collections::HashSet;
use std::time::Instant;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::input::{expand_path, TextInput};
use crate::types::{
    AppState, FileEntry, Issue, IssueComment, IssueFilter, IssueTab, Notification, RateLimit, Repo,
    SparseStep, Tab, UserProfile,
};

pub const LANGUAGE_CYCLE: &[Option<&str>] = &[
    None,
    Some("Rust"),
    Some("Python"),
    Some("Go"),
    Some("TypeScript"),
    Some("JavaScript"),
    Some("C"),
    Some("C++"),
    Some("Java"),
    Some("Zig"),
    Some("Nix"),
    Some("Shell"),
];

pub struct App {
    pub state: AppState,
    pub results: Vec<Repo>,
    pub selected: usize,
    pub search_query: TextInput,
    pub language_filter: Option<String>,
    pub readme_content: Option<String>,
    pub readme_scroll: u16,
    pub clone_path_input: TextInput,
    pub status_msg: Option<String>,
    pub status_set_at: Option<Instant>,
    pub loading: bool,
    // file browser
    pub file_entries: Vec<FileEntry>,
    pub file_selected: usize,
    pub file_path_stack: Vec<String>,
    pub file_content: Option<String>,
    pub file_scroll: u16,
    pub readme_pending: Option<Instant>,
    pub starred: HashSet<String>,
    pub rate_limit: Option<RateLimit>,
    pub language_idx: usize,
    // tab
    pub tab: Tab,
    // my repos
    pub my_repos: Vec<Repo>,
    pub my_repos_selected: usize,
    pub my_repos_loading: bool,
    pub rename_input: TextInput,
    // profile view
    pub profile_user: Option<UserProfile>,
    pub profile_repos: Vec<Repo>,
    pub profile_repos_selected: usize,
    pub profile_loading: bool,
    pub prev_state: Option<AppState>, // state before entering FileBrowsing
    // sparse clone
    pub sparse_path_input: TextInput,
    pub sparse_dirs_input: TextInput,
    pub sparse_step: SparseStep,
    // file save
    pub file_save_path_input: TextInput,
    // issues / PRs
    pub issues: Vec<Issue>,
    pub issues_selected: usize,
    pub issue_tab: IssueTab,
    pub issue_filter: IssueFilter,
    pub issues_loading: bool,
    pub current_issue: Option<Issue>,
    pub issue_comments: Vec<IssueComment>,
    pub issue_scroll: u16,
    pub new_issue_title: TextInput,
    pub new_issue_body: TextInput,
    pub new_issue_focus_body: bool, // false = title focused, true = body focused
    // notifications
    pub notifications: Vec<Notification>,
    pub notifications_selected: usize,
    pub notifications_loading: bool,
    pub notifications_unread_only: bool,
    // background task results
    pub bg_tx: mpsc::UnboundedSender<Result<String, String>>,
    pub bg_rx: mpsc::UnboundedReceiver<Result<String, String>>,
    // config
    pub config: Config,
}

impl App {
    pub fn new(config: Config) -> Self {
        let (bg_tx, bg_rx) = mpsc::unbounded_channel();
        Self {
            state: AppState::Searching,
            results: Vec::new(),
            selected: 0,
            search_query: TextInput::new(),
            language_filter: None,
            readme_content: None,
            readme_scroll: 0,
            clone_path_input: TextInput::new(),
            status_msg: None,
            status_set_at: None,
            loading: false,
            file_entries: Vec::new(),
            file_selected: 0,
            file_path_stack: Vec::new(),
            file_content: None,
            file_scroll: 0,
            readme_pending: None,
            starred: HashSet::new(),
            rate_limit: None,
            language_idx: 0,
            tab: Tab::Search,
            my_repos: Vec::new(),
            my_repos_selected: 0,
            my_repos_loading: false,
            rename_input: TextInput::new(),
            profile_user: None,
            profile_repos: Vec::new(),
            profile_repos_selected: 0,
            profile_loading: false,
            prev_state: None,
            sparse_path_input: TextInput::new(),
            sparse_dirs_input: TextInput::new(),
            sparse_step: SparseStep::Path,
            file_save_path_input: TextInput::new(),
            issues: Vec::new(),
            issues_selected: 0,
            issue_tab: IssueTab::Issues,
            issue_filter: IssueFilter::Open,
            issues_loading: false,
            current_issue: None,
            issue_comments: Vec::new(),
            issue_scroll: 0,
            new_issue_title: TextInput::new(),
            new_issue_body: TextInput::new(),
            new_issue_focus_body: false,
            notifications: Vec::new(),
            notifications_selected: 0,
            notifications_loading: false,
            notifications_unread_only: true,
            bg_tx,
            bg_rx,
            config,
        }
    }

    /// Pre-fill clone/sparse path inputs with default_clone_path from config if set.
    pub fn prefill_clone_path(&mut self) {
        self.clone_path_input.clear();
        if let Some(default) = &self.config.default_clone_path.clone() {
            let expanded = expand_path(default.trim());
            for c in expanded.chars() {
                self.clone_path_input.insert(c);
            }
        }
    }

    pub fn profile_next(&mut self) {
        if self.profile_repos.is_empty() {
            return;
        }
        self.profile_repos_selected = (self.profile_repos_selected + 1) % self.profile_repos.len();
        self.readme_content = None;
        self.readme_scroll = 0;
        self.readme_pending = Some(Instant::now());
    }

    pub fn profile_prev(&mut self) {
        if self.profile_repos.is_empty() {
            return;
        }
        if self.profile_repos_selected == 0 {
            self.profile_repos_selected = self.profile_repos.len() - 1;
        } else {
            self.profile_repos_selected -= 1;
        }
        self.readme_content = None;
        self.readme_scroll = 0;
        self.readme_pending = Some(Instant::now());
    }

    pub fn selected_profile_repo(&self) -> Option<&Repo> {
        self.profile_repos.get(self.profile_repos_selected)
    }

    pub fn my_repos_next(&mut self) {
        if self.my_repos.is_empty() {
            return;
        }
        self.my_repos_selected = (self.my_repos_selected + 1) % self.my_repos.len();
        self.readme_content = None;
        self.readme_scroll = 0;
        self.readme_pending = Some(Instant::now());
    }

    pub fn my_repos_prev(&mut self) {
        if self.my_repos.is_empty() {
            return;
        }
        if self.my_repos_selected == 0 {
            self.my_repos_selected = self.my_repos.len() - 1;
        } else {
            self.my_repos_selected -= 1;
        }
        self.readme_content = None;
        self.readme_scroll = 0;
        self.readme_pending = Some(Instant::now());
    }

    pub fn selected_my_repo(&self) -> Option<&Repo> {
        self.my_repos.get(self.my_repos_selected)
    }

    pub fn current_language(&self) -> Option<&str> {
        LANGUAGE_CYCLE[self.language_idx]
    }

    pub fn cycle_language(&mut self) {
        self.language_idx = (self.language_idx + 1) % LANGUAGE_CYCLE.len();
        self.language_filter = LANGUAGE_CYCLE[self.language_idx].map(|s| s.to_string());
    }

    pub fn prefill_sparse_path(&mut self) {
        self.sparse_path_input.clear();
        if let Some(default) = &self.config.default_clone_path.clone() {
            let expanded = expand_path(default.trim());
            for c in expanded.chars() {
                self.sparse_path_input.insert(c);
            }
        }
    }

    pub fn next(&mut self) {
        if self.results.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.results.len();
        self.readme_content = None;
        self.readme_scroll = 0;
        self.readme_pending = Some(Instant::now());
    }

    pub fn prev(&mut self) {
        if self.results.is_empty() {
            return;
        }
        if self.selected == 0 {
            self.selected = self.results.len() - 1;
        } else {
            self.selected -= 1;
        }
        self.readme_content = None;
        self.readme_scroll = 0;
        self.readme_pending = Some(Instant::now());
    }

    pub fn take_readme_pending(&mut self) -> bool {
        if let Some(t) = self.readme_pending {
            if t.elapsed().as_millis() >= 300 {
                self.readme_pending = None;
                return true;
            }
        }
        false
    }

    pub fn selected_repo(&self) -> Option<&Repo> {
        self.results.get(self.selected)
    }

    /// Returns the active repo regardless of which tab/state is active.
    pub fn active_repo(&self) -> Option<&Repo> {
        // in profile view, or file-browsing that originated from profile
        let from_profile = matches!(self.state, AppState::ViewingProfile)
            || matches!(&self.prev_state, Some(AppState::ViewingProfile));
        if from_profile {
            return self.profile_repos.get(self.profile_repos_selected);
        }
        match self.tab {
            Tab::MyRepos => self.my_repos.get(self.my_repos_selected),
            Tab::Search => self.results.get(self.selected),
        }
    }

    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.state = AppState::Error(msg.into());
        self.loading = false;
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_msg = Some(msg.into());
        self.status_set_at = Some(Instant::now());
    }

    pub fn clear_status(&mut self) {
        self.status_msg = None;
        self.status_set_at = None;
    }

    pub fn tick_status(&mut self) {
        if let Some(set_at) = self.status_set_at {
            if set_at.elapsed().as_secs() >= 4 {
                self.clear_status();
            }
        }
    }

    pub fn file_next(&mut self) {
        if self.file_entries.is_empty() {
            return;
        }
        self.file_selected = (self.file_selected + 1) % self.file_entries.len();
        self.file_content = None;
        self.file_scroll = 0;
    }

    pub fn file_prev(&mut self) {
        if self.file_entries.is_empty() {
            return;
        }
        if self.file_selected == 0 {
            self.file_selected = self.file_entries.len() - 1;
        } else {
            self.file_selected -= 1;
        }
        self.file_content = None;
        self.file_scroll = 0;
    }

    pub fn selected_entry(&self) -> Option<&FileEntry> {
        self.file_entries.get(self.file_selected)
    }

    pub fn current_file_path(&self) -> &str {
        self.file_path_stack
            .last()
            .map(|s| s.as_str())
            .unwrap_or("")
    }
}
