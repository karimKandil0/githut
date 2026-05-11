use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::time::Duration;

use std::time::Instant;

use crate::api::GithubClient;
use crate::app::App;
use crate::git;
use crate::types::{AppState, EntryType, SparseStep};

/// Returns true if the app should quit.
pub async fn handle_events(app: &mut App, client: &GithubClient) -> Result<bool> {
    if !event::poll(Duration::from_millis(100))? {
        return Ok(false);
    }

    let Event::Key(key) = event::read()? else {
        return Ok(false);
    };

    if key.kind != KeyEventKind::Press {
        return Ok(false);
    }

    match &app.state.clone() {
        AppState::Searching => handle_searching(app, client, key.code).await,
        AppState::Browsing => handle_browsing(app, client, key.code).await,
        AppState::FileBrowsing => handle_file_browsing(app, client, key.code).await,
        AppState::Cloning => handle_cloning(app, key.code).await,
        AppState::SparseCloning => handle_sparse_cloning(app, key.code).await,
        AppState::Error(_) | AppState::Help => {
            app.state = AppState::Browsing;
            Ok(false)
        }
        AppState::Previewing | AppState::SparseCloning => Ok(false),
    }
}

async fn handle_searching(app: &mut App, client: &GithubClient, code: KeyCode) -> Result<bool> {
    match code {
        KeyCode::Esc => {
            app.state = AppState::Browsing;
        }
        KeyCode::Enter => {
            if !app.search_query.is_empty() {
                do_search(app, client).await;
            }
        }
        KeyCode::Backspace => {
            app.search_query.pop();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
        }
        _ => {}
    }
    Ok(false)
}

async fn handle_browsing(app: &mut App, client: &GithubClient, code: KeyCode) -> Result<bool> {
    match code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Char('/') => {
            app.state = AppState::Searching;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.next();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.prev();
        }
        // J/K scroll the readme preview
        KeyCode::Char('J') => {
            app.readme_scroll = app.readme_scroll.saturating_add(1);
        }
        KeyCode::Char('K') => {
            app.readme_scroll = app.readme_scroll.saturating_sub(1);
        }
        KeyCode::Char('r') => {
            if !app.search_query.is_empty() {
                do_search(app, client).await;
            }
        }
        KeyCode::Char('?') => {
            app.state = AppState::Help;
        }
        KeyCode::Char('c') => {
            if app.selected_repo().is_some() {
                app.clone_path_input.clear();
                app.state = AppState::Cloning;
            }
        }
        KeyCode::Char('C') => {
            if app.selected_repo().is_some() {
                app.sparse_path_input.clear();
                app.sparse_dirs_input.clear();
                app.sparse_step = SparseStep::Path;
                app.state = AppState::SparseCloning;
            }
        }
        KeyCode::Char('o') => {
            if let Some(repo) = app.selected_repo() {
                let url = repo.html_url.clone();
                if let Err(e) = open::that(&url) {
                    app.set_error(format!("failed to open browser: {}", e));
                }
            }
        }
        KeyCode::Char('s') => {
            if let Some(repo) = app.selected_repo() {
                let owner = repo.owner.clone();
                let name = repo.name.clone();
                let full_name = repo.full_name.clone();
                let already = app.starred.contains(&full_name);
                if already {
                    match client.unstar(&owner, &name).await {
                        Ok(()) => {
                            app.starred.remove(&full_name);
                            app.set_status(format!("unstarred {}", full_name));
                        }
                        Err(e) => app.set_error(format!("unstar failed: {}", e)),
                    }
                } else {
                    match client.star(&owner, &name).await {
                        Ok(()) => {
                            app.starred.insert(full_name.clone());
                            app.set_status(format!("starred {}", full_name));
                        }
                        Err(e) => app.set_error(format!("star failed: {}", e)),
                    }
                }
            }
        }
        KeyCode::Char('f') => {
            if let Some(repo) = app.selected_repo() {
                let owner = repo.owner.clone();
                let name = repo.name.clone();
                let full_name = repo.full_name.clone();
                match client.fork(&owner, &name).await {
                    Ok(()) => app.set_status(format!("forked {} — check your GitHub", full_name)),
                    Err(e) => app.set_error(format!("fork failed: {}", e)),
                }
            }
        }
        KeyCode::Char('l') | KeyCode::Enter => {
            if let Some(repo) = app.selected_repo() {
                let owner = repo.owner.clone();
                let name = repo.name.clone();
                app.file_entries.clear();
                app.file_selected = 0;
                app.file_path_stack.clear();
                app.file_content = None;
                app.file_scroll = 0;
                app.loading = true;
                app.state = AppState::FileBrowsing;
                match client.get_contents(&owner, &name, "").await {
                    Ok(entries) => app.file_entries = entries,
                    Err(e) => app.set_error(format!("failed to load files: {}", e)),
                }
                app.loading = false;
            }
        }
        _ => {}
    }
    Ok(false)
}

async fn handle_file_browsing(app: &mut App, client: &GithubClient, code: KeyCode) -> Result<bool> {
    match code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Esc | KeyCode::Char('h') if app.file_path_stack.is_empty() => {
            // at root — go back to repo browsing
            app.state = AppState::Browsing;
            app.file_content = None;
        }
        KeyCode::Char('h') => {
            // go up one directory
            app.file_path_stack.pop();
            if let Some(repo) = app.selected_repo() {
                let owner = repo.owner.clone();
                let name = repo.name.clone();
                let path = app.current_file_path().to_string();
                app.file_entries.clear();
                app.file_selected = 0;
                app.file_content = None;
                app.loading = true;
                match client.get_contents(&owner, &name, &path).await {
                    Ok(entries) => app.file_entries = entries,
                    Err(e) => app.set_error(format!("failed to load files: {}", e)),
                }
                app.loading = false;
            }
        }
        KeyCode::Esc => {
            app.state = AppState::Browsing;
            app.file_content = None;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.file_next();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.file_prev();
        }
        // J/K scroll the file preview
        KeyCode::Char('J') => {
            app.file_scroll = app.file_scroll.saturating_add(1);
        }
        KeyCode::Char('K') => {
            app.file_scroll = app.file_scroll.saturating_sub(1);
        }
        KeyCode::Char('l') | KeyCode::Enter => {
            if let Some(entry) = app.selected_entry().cloned() {
                match entry.entry_type {
                    EntryType::Dir => {
                        if let Some(repo) = app.selected_repo() {
                            let owner = repo.owner.clone();
                            let name = repo.name.clone();
                            app.file_path_stack.push(entry.path.clone());
                            app.file_entries.clear();
                            app.file_selected = 0;
                            app.file_content = None;
                            app.loading = true;
                            match client.get_contents(&owner, &name, &entry.path).await {
                                Ok(entries) => app.file_entries = entries,
                                Err(e) => app.set_error(format!("failed to load dir: {}", e)),
                            }
                            app.loading = false;
                        }
                    }
                    EntryType::File => {
                        if let Some(repo) = app.selected_repo() {
                            let owner = repo.owner.clone();
                            let name = repo.name.clone();
                            app.loading = true;
                            app.file_content = None;
                            match client.get_file_content(&owner, &name, &entry.path).await {
                                Ok(content) => {
                                    app.file_content = Some(content);
                                    app.file_scroll = 0;
                                }
                                Err(e) => app.set_error(format!("failed to load file: {}", e)),
                            }
                            app.loading = false;
                        }
                    }
                }
            }
        }
        _ => {}
    }
    Ok(false)
}

async fn handle_cloning(app: &mut App, code: KeyCode) -> Result<bool> {
    match code {
        KeyCode::Esc => {
            app.state = AppState::Browsing;
        }
        KeyCode::Backspace => {
            app.clone_path_input.pop();
        }
        KeyCode::Enter => {
            if let Some(repo) = app.selected_repo() {
                let url = repo.clone_url.clone();
                let path = app.clone_path_input.trim().to_string();
                if path.is_empty() {
                    app.set_error("clone path cannot be empty");
                    return Ok(false);
                }
                app.set_status(format!("cloning {}...", repo.full_name));
                app.state = AppState::Browsing;
                match git::clone_repo(&url, &path) {
                    Ok(()) => app.set_status(format!("cloned to {}", path)),
                    Err(e) => app.set_error(format!("clone failed: {}", e)),
                }
            }
        }
        KeyCode::Char(c) => {
            app.clone_path_input.push(c);
        }
        _ => {}
    }
    Ok(false)
}

async fn handle_sparse_cloning(app: &mut App, code: KeyCode) -> Result<bool> {
    match code {
        KeyCode::Esc => {
            app.state = AppState::Browsing;
        }
        KeyCode::Backspace => match app.sparse_step {
            SparseStep::Path => {
                app.sparse_path_input.pop();
            }
            SparseStep::Dirs => {
                app.sparse_dirs_input.pop();
            }
        },
        KeyCode::Enter => match app.sparse_step {
            SparseStep::Path => {
                if !app.sparse_path_input.trim().is_empty() {
                    app.sparse_step = SparseStep::Dirs;
                }
            }
            SparseStep::Dirs => {
                if let Some(repo) = app.selected_repo() {
                    let url = repo.clone_url.clone();
                    let branch = repo.default_branch.clone();
                    let path = app.sparse_path_input.trim().to_string();
                    let dirs_raw = app.sparse_dirs_input.trim().to_string();
                    let dirs: Vec<&str> = if dirs_raw.is_empty() {
                        vec![]
                    } else {
                        dirs_raw.split_whitespace().collect()
                    };
                    let full_name = repo.full_name.clone();
                    app.set_status(format!("sparse cloning {}...", full_name));
                    app.state = AppState::Browsing;
                    match git::sparse_clone(&url, &path, &branch, &dirs) {
                        Ok(()) => app.set_status(format!("sparse cloned to {}", path)),
                        Err(e) => app.set_error(format!("sparse clone failed: {}", e)),
                    }
                }
            }
        },
        KeyCode::Char(c) => match app.sparse_step {
            SparseStep::Path => app.sparse_path_input.push(c),
            SparseStep::Dirs => app.sparse_dirs_input.push(c),
        },
        _ => {}
    }
    Ok(false)
}

async fn do_search(app: &mut App, client: &GithubClient) {
    let query = app.search_query.clone();
    let lang = app.language_filter.clone();
    app.loading = true;
    app.state = AppState::Browsing;
    app.selected = 0;
    app.readme_content = None;

    match client.search_repos(&query, lang.as_deref()).await {
        Ok(result) => {
            app.results = result.repos;
            app.set_status(format!("{} results", result.total_count));
            app.readme_pending = Some(Instant::now());
        }
        Err(e) => app.set_error(format!("search failed: {}", e)),
    }
    app.loading = false;
}
