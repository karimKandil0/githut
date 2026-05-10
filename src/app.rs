use crate::types::{AppState, Repo};

pub struct App {
    pub state: AppState,
    pub results: Vec<Repo>,
    pub selected: usize,
    pub search_query: String,
    pub language_filter: Option<String>,
    pub readme_content: Option<String>,
    pub readme_scroll: u16,
    pub clone_path_input: String,
    pub status_msg: Option<String>,
    pub loading: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: AppState::Searching,
            results: Vec::new(),
            selected: 0,
            search_query: String::new(),
            language_filter: None,
            readme_content: None,
            readme_scroll: 0,
            clone_path_input: String::new(),
            status_msg: None,
            loading: false,
        }
    }

    pub fn next(&mut self) {
        if self.results.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.results.len();
        self.readme_content = None;
        self.readme_scroll = 0;
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
}
