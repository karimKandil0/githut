use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::time::Duration;

use crate::api::GithubClient;
use crate::app::App;
use crate::git;
use crate::types::{AppState, EntryType};

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
        AppState::Previewing => handle_previewing(app, key.code),
        AppState::FileBrowsing => handle_file_browsing(app, client, key.code).await,
        AppState::Cloning => handle_cloning(app, key.code).await,
        AppState::Error(_) | AppState::Help => {
            app.state = AppState::Browsing;
            Ok(false)
        }
        AppState::SparseCloning => Ok(false),
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
        KeyCode::Char('r') => {
            if !app.search_query.is_empty() {
                do_search(app, client).await;
            }
        }
        KeyCode::Char('?') => {
            app.state = AppState::Help;
        }
        KeyCode::Char('t') => {
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
        KeyCode::Char('c') => {
            if app.selected_repo().is_some() {
                app.clone_path_input.clear();
                app.state = AppState::Cloning;
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
        KeyCode::Enter => {
            if let Some(repo) = app.selected_repo() {
                let owner = repo.owner.clone();
                let name = repo.name.clone();
                app.loading = true;
                app.readme_content = None;
                match client.get_readme(&owner, &name).await {
                    Ok(md) => {
                        app.readme_content = Some(md);
                        app.readme_scroll = 0;
                        app.state = AppState::Previewing;
                    }
                    Err(e) => app.set_error(format!("readme fetch failed: {}", e)),
                }
                app.loading = false;
            }
        }
        _ => {}
    }
    Ok(false)
}

fn handle_previewing(app: &mut App, code: KeyCode) -> Result<bool> {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.state = AppState::Browsing;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.readme_scroll = app.readme_scroll.saturating_add(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.readme_scroll = app.readme_scroll.saturating_sub(1);
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

async fn handle_file_browsing(app: &mut App, client: &GithubClient, code: KeyCode) -> Result<bool> {
    match code {
        KeyCode::Char('q') => return Ok(true),
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
        KeyCode::Char('h') => {
            // go up one directory
            if app.file_path_stack.pop().is_some() {
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
        }
        Err(e) => app.set_error(format!("search failed: {}", e)),
    }
    app.loading = false;
}
