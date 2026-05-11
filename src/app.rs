use std::collections::HashSet;
use std::time::Instant;
use tokio::sync::mpsc;

use crate::input::TextInput;
use crate::types::{AppState, FileEntry, Repo, SparseStep};

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
    pub loading: bool,
    // file browser
    pub file_entries: Vec<FileEntry>,
    pub file_selected: usize,
    pub file_path_stack: Vec<String>,
    pub file_content: Option<String>,
    pub file_scroll: u16,
    pub readme_pending: Option<Instant>,
    pub starred: HashSet<String>,
    // sparse clone
    pub sparse_path_input: TextInput,
    pub sparse_dirs_input: TextInput,
    pub sparse_step: SparseStep,
    // file save
    pub file_save_path_input: TextInput,
    // background task results
    pub bg_tx: mpsc::UnboundedSender<Result<String, String>>,
    pub bg_rx: mpsc::UnboundedReceiver<Result<String, String>>,
}

impl App {
    pub fn new() -> Self {
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
            loading: false,
            file_entries: Vec::new(),
            file_selected: 0,
            file_path_stack: Vec::new(),
            file_content: None,
            file_scroll: 0,
            readme_pending: None,
            starred: HashSet::new(),
            sparse_path_input: TextInput::new(),
            sparse_dirs_input: TextInput::new(),
            sparse_step: SparseStep::Path,
            file_save_path_input: TextInput::new(),
            bg_tx,
            bg_rx,
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

    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.state = AppState::Error(msg.into());
        self.loading = false;
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_msg = Some(msg.into());
    }

    pub fn clear_status(&mut self) {
        self.status_msg = None;
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
