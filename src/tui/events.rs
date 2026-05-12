use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::path::Path;
use std::time::Duration;
use std::time::Instant;

use crate::api::GithubClient;
use crate::app::App;
use crate::git;
use crate::types::{AppState, EntryType, IssueFilter, IssueTab, SparseStep, Tab};

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
        AppState::MyRepos => handle_my_repos(app, client, key.code).await,
        AppState::FileBrowsing => handle_file_browsing(app, client, key.code).await,
        AppState::Cloning => handle_cloning(app, key.code).await,
        AppState::SparseCloning => handle_sparse_cloning(app, key.code).await,
        AppState::FileSaving => handle_file_saving(app, client, key.code).await,
        AppState::Renaming => handle_renaming(app, client, key.code).await,
        AppState::ConfirmDelete => handle_confirm_delete(app, client, key.code).await,
        AppState::ViewingProfile => handle_viewing_profile(app, client, key.code).await,
        AppState::Error(_) | AppState::Help => {
            // return to correct state based on tab
            app.state = if app.tab == Tab::MyRepos {
                AppState::MyRepos
            } else {
                AppState::Browsing
            };
            Ok(false)
        }
        AppState::Previewing => Ok(false),
        AppState::ViewingIssues => handle_viewing_issues(app, key.code, client).await,
        AppState::ViewingIssue => handle_viewing_issue(app, key.code).await,
        AppState::CreatingIssue => handle_creating_issue(app, key.code, client).await,
        AppState::ViewingNotifications => handle_viewing_notifications(app, key.code, client).await,
    }
}

async fn handle_searching(app: &mut App, client: &GithubClient, code: KeyCode) -> Result<bool> {
    match code {
        KeyCode::Esc => {
            app.state = AppState::Browsing;
        }
        KeyCode::Char('2') => {
            return switch_to_my_repos(app, client).await;
        }
        KeyCode::Char('3') => {
            open_notifications(app, client).await;
        }
        KeyCode::Enter => {
            if !app.search_query.is_empty() {
                do_search(app, client).await;
            }
        }
        KeyCode::Tab => {
            app.cycle_language();
        }
        KeyCode::Backspace => app.search_query.backspace(),
        KeyCode::Delete => app.search_query.delete(),
        KeyCode::Left => app.search_query.move_left(),
        KeyCode::Right => app.search_query.move_right(),
        KeyCode::Home => app.search_query.move_home(),
        KeyCode::End => app.search_query.move_end(),
        KeyCode::Char(c) => app.search_query.insert(c),
        _ => {}
    }
    Ok(false)
}

async fn handle_browsing(app: &mut App, client: &GithubClient, code: KeyCode) -> Result<bool> {
    match code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Char('1') => {
            app.tab = Tab::Search;
            app.state = AppState::Browsing;
            app.readme_content = None;
            app.readme_scroll = 0;
        }
        KeyCode::Char('2') => {
            return switch_to_my_repos(app, client).await;
        }
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
        KeyCode::Tab => {
            app.cycle_language();
            if !app.search_query.is_empty() {
                do_search(app, client).await;
            }
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
            if app.active_repo().is_some() {
                app.prefill_clone_path();
                app.state = AppState::Cloning;
            }
        }
        KeyCode::Char('C') => {
            if app.active_repo().is_some() {
                app.prefill_sparse_path();
                app.sparse_dirs_input.clear();
                app.sparse_step = SparseStep::Path;
                app.state = AppState::SparseCloning;
            }
        }
        KeyCode::Char('o') => {
            if let Some(repo) = app.active_repo() {
                let url = repo.html_url.clone();
                if let Err(e) = open::that(&url) {
                    app.set_error(format!("failed to open browser: {}", e));
                }
            }
        }
        KeyCode::Char('s') => {
            if let Some(repo) = app.active_repo() {
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
            if let Some(repo) = app.active_repo() {
                let owner = repo.owner.clone();
                let name = repo.name.clone();
                let full_name = repo.full_name.clone();
                match client.fork(&owner, &name).await {
                    Ok(()) => app.set_status(format!("forked {} — check your GitHub", full_name)),
                    Err(e) => app.set_error(format!("fork failed: {}", e)),
                }
            }
        }
        KeyCode::Char('u') => {
            if let Some(repo) = app.active_repo() {
                let login = repo.owner.clone();
                open_profile(app, client, &login).await;
            }
        }
        KeyCode::Char('i') => {
            if let Some(repo) = app.active_repo() {
                let owner = repo.owner.clone();
                let name = repo.name.clone();
                open_issues(app, client, &owner, &name).await;
            }
        }
        KeyCode::Char('3') => {
            open_notifications(app, client).await;
        }
        KeyCode::Char('l') | KeyCode::Enter => {
            if let Some(repo) = app.active_repo() {
                let owner = repo.owner.clone();
                let name = repo.name.clone();
                app.file_entries.clear();
                app.file_selected = 0;
                app.file_path_stack.clear();
                app.file_content = None;
                app.file_scroll = 0;
                app.loading = true;
                app.prev_state = None;
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
            app.state = match app.prev_state.take() {
                Some(AppState::ViewingProfile) => AppState::ViewingProfile,
                _ if app.tab == Tab::MyRepos => AppState::MyRepos,
                _ => AppState::Browsing,
            };
            app.file_content = None;
        }
        KeyCode::Char('h') => {
            app.file_path_stack.pop();
            if let Some(repo) = app.active_repo() {
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
            app.state = match app.prev_state.take() {
                Some(AppState::ViewingProfile) => AppState::ViewingProfile,
                _ if app.tab == Tab::MyRepos => AppState::MyRepos,
                _ => AppState::Browsing,
            };
            app.file_content = None;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.file_next();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.file_prev();
        }
        KeyCode::Char('J') => {
            app.file_scroll = app.file_scroll.saturating_add(1);
        }
        KeyCode::Char('K') => {
            app.file_scroll = app.file_scroll.saturating_sub(1);
        }
        KeyCode::Char('c') => {
            if let Some(entry) = app.selected_entry() {
                if entry.entry_type == EntryType::File {
                    app.file_save_path_input.clear();
                    app.state = AppState::FileSaving;
                }
            }
        }
        KeyCode::Char('l') | KeyCode::Enter => {
            if let Some(entry) = app.selected_entry().cloned() {
                match entry.entry_type {
                    EntryType::Dir => {
                        if let Some(repo) = app.active_repo() {
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
                        if let Some(repo) = app.active_repo() {
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
        KeyCode::Backspace => app.clone_path_input.backspace(),
        KeyCode::Delete => app.clone_path_input.delete(),
        KeyCode::Left => app.clone_path_input.move_left(),
        KeyCode::Right => app.clone_path_input.move_right(),
        KeyCode::Home => app.clone_path_input.move_home(),
        KeyCode::End => app.clone_path_input.move_end(),
        KeyCode::Enter => {
            if let Some(repo) = app.active_repo() {
                let url = repo.clone_url.clone();
                let path = app.clone_path_input.to_path();
                if path.is_empty() {
                    app.set_error("clone path cannot be empty");
                    return Ok(false);
                }
                let full_name = repo.full_name.clone();
                app.set_status(format!("cloning {}...", full_name));
                app.state = if app.tab == Tab::MyRepos {
                    AppState::MyRepos
                } else {
                    AppState::Browsing
                };
                let tx = app.bg_tx.clone();
                tokio::task::spawn_blocking(move || {
                    let res = git::clone_repo(&url, &path)
                        .map(|_| format!("cloned to {}", path))
                        .map_err(|e| format!("clone failed: {}", e));
                    let _ = tx.send(res);
                });
            }
        }
        KeyCode::Char(c) => app.clone_path_input.insert(c),
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
            SparseStep::Path => app.sparse_path_input.backspace(),
            SparseStep::Dirs => app.sparse_dirs_input.backspace(),
        },
        KeyCode::Delete => match app.sparse_step {
            SparseStep::Path => app.sparse_path_input.delete(),
            SparseStep::Dirs => app.sparse_dirs_input.delete(),
        },
        KeyCode::Left => match app.sparse_step {
            SparseStep::Path => app.sparse_path_input.move_left(),
            SparseStep::Dirs => app.sparse_dirs_input.move_left(),
        },
        KeyCode::Right => match app.sparse_step {
            SparseStep::Path => app.sparse_path_input.move_right(),
            SparseStep::Dirs => app.sparse_dirs_input.move_right(),
        },
        KeyCode::Home => match app.sparse_step {
            SparseStep::Path => app.sparse_path_input.move_home(),
            SparseStep::Dirs => app.sparse_dirs_input.move_home(),
        },
        KeyCode::End => match app.sparse_step {
            SparseStep::Path => app.sparse_path_input.move_end(),
            SparseStep::Dirs => app.sparse_dirs_input.move_end(),
        },
        KeyCode::Enter => match app.sparse_step {
            SparseStep::Path => {
                if !app.sparse_path_input.as_str().trim().is_empty() {
                    app.sparse_step = SparseStep::Dirs;
                }
            }
            SparseStep::Dirs => {
                if let Some(repo) = app.active_repo() {
                    let url = repo.clone_url.clone();
                    let branch = repo.default_branch.clone();
                    let path = app.sparse_path_input.to_path();
                    let dirs_raw = app.sparse_dirs_input.as_str().trim().to_string();
                    let dirs: Vec<String> = if dirs_raw.is_empty() {
                        vec![]
                    } else {
                        dirs_raw.split_whitespace().map(|s| s.to_string()).collect()
                    };
                    let full_name = repo.full_name.clone();
                    app.set_status(format!("sparse cloning {}...", full_name));
                    app.state = AppState::Browsing;
                    let tx = app.bg_tx.clone();
                    tokio::task::spawn_blocking(move || {
                        let dir_refs: Vec<&str> = dirs.iter().map(|s| s.as_str()).collect();
                        let res = git::sparse_clone(&url, &path, &branch, &dir_refs)
                            .map(|_| format!("sparse cloned to {}", path))
                            .map_err(|e| format!("sparse clone failed: {}", e));
                        let _ = tx.send(res);
                    });
                }
            }
        },
        KeyCode::Char(c) => match app.sparse_step {
            SparseStep::Path => app.sparse_path_input.insert(c),
            SparseStep::Dirs => app.sparse_dirs_input.insert(c),
        },
        _ => {}
    }
    Ok(false)
}

async fn handle_file_saving(app: &mut App, client: &GithubClient, code: KeyCode) -> Result<bool> {
    match code {
        KeyCode::Esc => {
            app.state = AppState::FileBrowsing;
        }
        KeyCode::Backspace => app.file_save_path_input.backspace(),
        KeyCode::Delete => app.file_save_path_input.delete(),
        KeyCode::Left => app.file_save_path_input.move_left(),
        KeyCode::Right => app.file_save_path_input.move_right(),
        KeyCode::Home => app.file_save_path_input.move_home(),
        KeyCode::End => app.file_save_path_input.move_end(),
        KeyCode::Char(c) => app.file_save_path_input.insert(c),
        KeyCode::Enter => {
            let dest = app.file_save_path_input.to_path();
            if dest.is_empty() {
                app.set_error("save path cannot be empty");
                return Ok(false);
            }
            if let (Some(repo), Some(entry)) =
                (app.active_repo().cloned(), app.selected_entry().cloned())
            {
                let owner = repo.owner.clone();
                let name = repo.name.clone();
                let file_path = entry.path.clone();
                let file_name = entry.name.clone();
                app.set_status(format!("downloading {}...", file_name));
                app.state = AppState::FileBrowsing;
                match client.get_file_content(&owner, &name, &file_path).await {
                    Ok(content) => {
                        let tx = app.bg_tx.clone();
                        tokio::task::spawn_blocking(move || {
                            let result = (|| -> std::result::Result<(), String> {
                                if let Some(parent) = Path::new(&dest).parent() {
                                    if !parent.as_os_str().is_empty() {
                                        std::fs::create_dir_all(parent)
                                            .map_err(|e| e.to_string())?;
                                    }
                                }
                                std::fs::write(&dest, content.as_bytes())
                                    .map_err(|e| e.to_string())?;
                                Ok(())
                            })();
                            let msg = result
                                .map(|_| format!("saved to {}", dest))
                                .map_err(|e| format!("save failed: {}", e));
                            let _ = tx.send(msg);
                        });
                    }
                    Err(e) => app.set_error(format!("download failed: {}", e)),
                }
            }
        }
        _ => {}
    }
    Ok(false)
}

async fn switch_to_my_repos(app: &mut App, client: &GithubClient) -> Result<bool> {
    app.tab = Tab::MyRepos;
    app.state = AppState::MyRepos;
    app.readme_content = None;
    app.readme_scroll = 0;
    if app.my_repos.is_empty() {
        app.my_repos_loading = true;
        match client.list_my_repos().await {
            Ok(repos) => app.my_repos = repos,
            Err(e) => app.set_error(format!("failed to load your repos: {}", e)),
        }
        app.my_repos_loading = false;
    }
    // show profile README if a repo named after the user exists
    if let Some(login) = app.my_repos.first().map(|r| r.owner.clone()) {
        let has_profile_repo = app.my_repos.iter().any(|r| r.name == login);
        if has_profile_repo {
            if let Ok(md) = client.get_readme(&login, &login).await {
                app.readme_content = Some(md);
            }
        }
    }
    Ok(false)
}

async fn handle_my_repos(app: &mut App, client: &GithubClient, code: KeyCode) -> Result<bool> {
    match code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Char('1') => {
            app.tab = Tab::Search;
            app.state = AppState::Browsing;
            app.readme_content = None;
            app.readme_scroll = 0;
        }
        KeyCode::Char('2') => {
            // refresh
            app.my_repos_loading = true;
            match client.list_my_repos().await {
                Ok(repos) => {
                    app.my_repos = repos;
                    app.my_repos_selected = 0;
                }
                Err(e) => app.set_error(format!("failed to load your repos: {}", e)),
            }
            app.my_repos_loading = false;
        }
        KeyCode::Char('j') | KeyCode::Down => app.my_repos_next(),
        KeyCode::Char('k') | KeyCode::Up => app.my_repos_prev(),
        KeyCode::Char('J') => {
            app.readme_scroll = app.readme_scroll.saturating_add(1);
        }
        KeyCode::Char('K') => {
            app.readme_scroll = app.readme_scroll.saturating_sub(1);
        }
        KeyCode::Char('?') => {
            app.state = AppState::Help;
        }
        KeyCode::Char('o') => {
            if let Some(repo) = app.selected_my_repo() {
                let url = repo.html_url.clone();
                if let Err(e) = open::that(&url) {
                    app.set_error(format!("failed to open browser: {}", e));
                }
            }
        }
        KeyCode::Char('c') => {
            if app.selected_my_repo().is_some() {
                app.prefill_clone_path();
                app.state = AppState::Cloning;
            }
        }
        KeyCode::Char('l') | KeyCode::Enter => {
            if let Some(repo) = app.selected_my_repo() {
                let owner = repo.owner.clone();
                let name = repo.name.clone();
                app.file_entries.clear();
                app.file_selected = 0;
                app.file_path_stack.clear();
                app.file_content = None;
                app.file_scroll = 0;
                app.loading = true;
                app.prev_state = None;
                app.state = AppState::FileBrowsing;
                match client.get_contents(&owner, &name, "").await {
                    Ok(entries) => app.file_entries = entries,
                    Err(e) => app.set_error(format!("failed to load files: {}", e)),
                }
                app.loading = false;
            }
        }
        KeyCode::Char('u') => {
            if let Some(repo) = app.selected_my_repo() {
                let login = repo.owner.clone();
                open_profile(app, client, &login).await;
            }
        }
        KeyCode::Char('R') => {
            if let Some(repo) = app.selected_my_repo() {
                let name = repo.name.clone();
                app.rename_input.clear();
                for c in name.chars() {
                    app.rename_input.insert(c);
                }
                app.state = AppState::Renaming;
            }
        }
        KeyCode::Char('D') => {
            if app.selected_my_repo().is_some() {
                app.state = AppState::ConfirmDelete;
            }
        }
        KeyCode::Char('A') => {
            if let Some(repo) = app.selected_my_repo() {
                let owner = repo.owner.clone();
                let name = repo.name.clone();
                let archived = repo.archived;
                let full_name = repo.full_name.clone();
                match client.set_archived(&owner, &name, !archived).await {
                    Ok(()) => {
                        // update in-place
                        if let Some(r) = app.my_repos.get_mut(app.my_repos_selected) {
                            r.archived = !archived;
                        }
                        let action = if !archived { "archived" } else { "unarchived" };
                        app.set_status(format!("{} {}", action, full_name));
                    }
                    Err(e) => app.set_error(format!("archive failed: {}", e)),
                }
            }
        }
        KeyCode::Char('i') => {
            if let Some(repo) = app.selected_my_repo() {
                let owner = repo.owner.clone();
                let name = repo.name.clone();
                open_issues(app, client, &owner, &name).await;
            }
        }
        KeyCode::Char('3') => {
            open_notifications(app, client).await;
        }
        _ => {}
    }
    Ok(false)
}

async fn handle_renaming(app: &mut App, client: &GithubClient, code: KeyCode) -> Result<bool> {
    match code {
        KeyCode::Esc => {
            app.state = AppState::MyRepos;
        }
        KeyCode::Backspace => app.rename_input.backspace(),
        KeyCode::Delete => app.rename_input.delete(),
        KeyCode::Left => app.rename_input.move_left(),
        KeyCode::Right => app.rename_input.move_right(),
        KeyCode::Home => app.rename_input.move_home(),
        KeyCode::End => app.rename_input.move_end(),
        KeyCode::Char(c) => app.rename_input.insert(c),
        KeyCode::Enter => {
            let new_name = app.rename_input.as_str().trim().to_string();
            if new_name.is_empty() {
                app.set_error("name cannot be empty");
                return Ok(false);
            }
            if let Some(repo) = app.selected_my_repo() {
                let owner = repo.owner.clone();
                let old_name = repo.name.clone();
                match client.rename_repo(&owner, &old_name, &new_name).await {
                    Ok(()) => {
                        if let Some(r) = app.my_repos.get_mut(app.my_repos_selected) {
                            r.name = new_name.clone();
                            r.full_name = format!("{}/{}", owner, new_name);
                        }
                        app.set_status(format!("renamed to {}", new_name));
                        app.state = AppState::MyRepos;
                    }
                    Err(e) => app.set_error(format!("rename failed: {}", e)),
                }
            }
        }
        _ => {}
    }
    Ok(false)
}

async fn handle_confirm_delete(
    app: &mut App,
    client: &GithubClient,
    code: KeyCode,
) -> Result<bool> {
    match code {
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
            app.state = AppState::MyRepos;
        }
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(repo) = app.selected_my_repo() {
                let owner = repo.owner.clone();
                let name = repo.name.clone();
                let full_name = repo.full_name.clone();
                match client.delete_repo(&owner, &name).await {
                    Ok(()) => {
                        app.my_repos.remove(app.my_repos_selected);
                        if app.my_repos_selected > 0 && app.my_repos_selected >= app.my_repos.len()
                        {
                            app.my_repos_selected -= 1;
                        }
                        app.set_status(format!("deleted {}", full_name));
                        app.state = AppState::MyRepos;
                    }
                    Err(e) => app.set_error(format!("delete failed: {}", e)),
                }
            }
        }
        _ => {}
    }
    Ok(false)
}

async fn open_profile(app: &mut App, client: &GithubClient, login: &str) {
    app.profile_loading = true;
    app.profile_user = None;
    app.profile_repos.clear();
    app.profile_repos_selected = 0;
    app.readme_content = None;
    app.readme_scroll = 0;

    let (profile_result, repos_result) = tokio::join!(
        client.get_user_profile(login),
        client.list_user_repos(login),
    );

    match profile_result {
        Ok(profile) => app.profile_user = Some(profile),
        Err(e) => {
            app.set_error(format!("failed to load profile: {}", e));
            app.profile_loading = false;
            return;
        }
    }
    match repos_result {
        Ok(repos) => {
            app.profile_repos = repos;
            app.readme_pending = Some(Instant::now());
        }
        Err(e) => app.set_error(format!("failed to load user repos: {}", e)),
    }

    app.profile_loading = false;
    app.state = AppState::ViewingProfile;
}

async fn handle_viewing_profile(
    app: &mut App,
    client: &GithubClient,
    code: KeyCode,
) -> Result<bool> {
    match code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Esc | KeyCode::Char('h') => {
            app.state = if app.tab == Tab::MyRepos {
                AppState::MyRepos
            } else {
                AppState::Browsing
            };
            app.readme_content = None;
            app.readme_scroll = 0;
        }
        KeyCode::Char('j') | KeyCode::Down => app.profile_next(),
        KeyCode::Char('k') | KeyCode::Up => app.profile_prev(),
        KeyCode::Char('J') => {
            app.readme_scroll = app.readme_scroll.saturating_add(1);
        }
        KeyCode::Char('K') => {
            app.readme_scroll = app.readme_scroll.saturating_sub(1);
        }
        KeyCode::Char('u') => {
            // navigate to the owner of the selected profile repo (follow the rabbit hole)
            if let Some(repo) = app.selected_profile_repo() {
                let login = repo.owner.clone();
                if app.profile_user.as_ref().map(|p| &p.login) != Some(&login) {
                    open_profile(app, client, &login).await;
                }
            }
        }
        KeyCode::Char('o') => {
            if let Some(profile) = &app.profile_user {
                let url = profile.html_url.clone();
                if let Err(e) = open::that(&url) {
                    app.set_error(format!("failed to open browser: {}", e));
                }
            }
        }
        KeyCode::Char('c') => {
            if app.selected_profile_repo().is_some() {
                app.prefill_clone_path();
                app.state = AppState::Cloning;
            }
        }
        KeyCode::Char('l') | KeyCode::Enter => {
            if let Some(repo) = app.selected_profile_repo() {
                let owner = repo.owner.clone();
                let name = repo.name.clone();
                app.file_entries.clear();
                app.file_selected = 0;
                app.file_path_stack.clear();
                app.file_content = None;
                app.file_scroll = 0;
                app.loading = true;
                app.prev_state = Some(AppState::ViewingProfile);
                app.state = AppState::FileBrowsing;
                match client.get_contents(&owner, &name, "").await {
                    Ok(entries) => app.file_entries = entries,
                    Err(e) => app.set_error(format!("failed to load files: {}", e)),
                }
                app.loading = false;
            }
        }
        KeyCode::Char('i') => {
            if let Some(repo) = app.selected_profile_repo() {
                let owner = repo.owner.clone();
                let name = repo.name.clone();
                open_issues(app, client, &owner, &name).await;
            }
        }
        KeyCode::Char('3') => {
            open_notifications(app, client).await;
        }
        KeyCode::Char('?') => {
            app.state = AppState::Help;
        }
        _ => {}
    }
    Ok(false)
}

async fn do_search(app: &mut App, client: &GithubClient) {
    let query = app.search_query.as_str().to_string();
    let lang = app.language_filter.clone();
    app.loading = true;
    app.state = AppState::Browsing;
    app.selected = 0;
    app.readme_content = None;

    let (search_result, rate_result) = tokio::join!(
        client.search_repos(&query, lang.as_deref()),
        client.get_rate_limit(),
    );

    match search_result {
        Ok(result) => {
            app.results = result.repos;
            app.set_status(format!("{} results", result.total_count));
            app.readme_pending = Some(Instant::now());
        }
        Err(e) => app.set_error(format!("search failed: {}", e)),
    }
    if let Ok(rl) = rate_result {
        app.rate_limit = Some(rl);
    }
    app.loading = false;
}

async fn open_issues(app: &mut App, client: &GithubClient, owner: &str, name: &str) {
    app.issues_loading = true;
    app.issues.clear();
    app.issues_selected = 0;
    app.current_issue = None;
    app.issue_comments.clear();
    app.issue_scroll = 0;
    let is_pr = app.issue_tab == IssueTab::PullRequests;
    match client
        .list_issues(owner, name, &app.issue_filter, is_pr)
        .await
    {
        Ok(issues) => app.issues = issues,
        Err(e) => {
            app.set_error(format!("failed to load issues: {}", e));
            app.issues_loading = false;
            return;
        }
    }
    app.issues_loading = false;
    app.state = AppState::ViewingIssues;
}

async fn open_notifications(app: &mut App, client: &GithubClient) {
    app.notifications_loading = true;
    app.notifications.clear();
    app.notifications_selected = 0;
    match client
        .list_notifications(app.notifications_unread_only)
        .await
    {
        Ok(notifs) => app.notifications = notifs,
        Err(e) => {
            app.set_error(format!("failed to load notifications: {}", e));
            app.notifications_loading = false;
            return;
        }
    }
    app.notifications_loading = false;
    app.state = AppState::ViewingNotifications;
}

async fn handle_viewing_issues(
    app: &mut App,
    code: KeyCode,
    client: &GithubClient,
) -> Result<bool> {
    match code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Esc | KeyCode::Char('h') => {
            app.state = match app.tab {
                Tab::MyRepos => AppState::MyRepos,
                Tab::Search => AppState::Browsing,
            };
        }
        KeyCode::Char('1') => {
            app.tab = Tab::Search;
            app.state = AppState::Browsing;
        }
        KeyCode::Char('2') => {
            return switch_to_my_repos(app, client).await;
        }
        KeyCode::Char('3') => {
            open_notifications(app, client).await;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.issues.is_empty() {
                app.issues_selected = (app.issues_selected + 1) % app.issues.len();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if !app.issues.is_empty() {
                if app.issues_selected == 0 {
                    app.issues_selected = app.issues.len() - 1;
                } else {
                    app.issues_selected -= 1;
                }
            }
        }
        KeyCode::Char('J') => {
            app.issue_scroll = app.issue_scroll.saturating_add(1);
        }
        KeyCode::Char('K') => {
            app.issue_scroll = app.issue_scroll.saturating_sub(1);
        }
        // Tab: switch between Issues and PRs
        KeyCode::Tab => {
            app.issue_tab = match app.issue_tab {
                IssueTab::Issues => IssueTab::PullRequests,
                IssueTab::PullRequests => IssueTab::Issues,
            };
            if let Some(repo) = app.active_repo() {
                let owner = repo.owner.clone();
                let name = repo.name.clone();
                let is_pr = app.issue_tab == IssueTab::PullRequests;
                app.issues_loading = true;
                app.issues.clear();
                app.issues_selected = 0;
                match client
                    .list_issues(&owner, &name, &app.issue_filter, is_pr)
                    .await
                {
                    Ok(issues) => app.issues = issues,
                    Err(e) => app.set_error(format!("failed to load: {}", e)),
                }
                app.issues_loading = false;
            }
        }
        // f: cycle filter (open/closed/all)
        KeyCode::Char('f') => {
            app.issue_filter = app.issue_filter.next();
            if let Some(repo) = app.active_repo() {
                let owner = repo.owner.clone();
                let name = repo.name.clone();
                let is_pr = app.issue_tab == IssueTab::PullRequests;
                app.issues_loading = true;
                app.issues.clear();
                app.issues_selected = 0;
                match client
                    .list_issues(&owner, &name, &app.issue_filter, is_pr)
                    .await
                {
                    Ok(issues) => app.issues = issues,
                    Err(e) => app.set_error(format!("failed to load: {}", e)),
                }
                app.issues_loading = false;
            }
        }
        // n: new issue
        KeyCode::Char('n') => {
            if app.issue_tab == IssueTab::Issues {
                app.new_issue_title.clear();
                app.new_issue_body.clear();
                app.new_issue_focus_body = false;
                app.state = AppState::CreatingIssue;
            }
        }
        // x: close selected issue
        KeyCode::Char('x') => {
            if let Some(issue) = app.issues.get(app.issues_selected) {
                if !issue.pull_request && issue.state == "open" {
                    let number = issue.number;
                    if let Some(repo) = app.active_repo() {
                        let owner = repo.owner.clone();
                        let name = repo.name.clone();
                        match client.close_issue(&owner, &name, number).await {
                            Ok(()) => {
                                if let Some(i) = app.issues.get_mut(app.issues_selected) {
                                    i.state = "closed".to_string();
                                }
                                app.set_status(format!("closed #{}", number));
                            }
                            Err(e) => app.set_error(format!("close failed: {}", e)),
                        }
                    }
                }
            }
        }
        // Enter/l: open issue to read body + comments
        KeyCode::Enter | KeyCode::Char('l') => {
            if let Some(issue) = app.issues.get(app.issues_selected).cloned() {
                let number = issue.number;
                app.current_issue = Some(issue);
                app.issue_comments.clear();
                app.issue_scroll = 0;
                if let Some(repo) = app.active_repo() {
                    let owner = repo.owner.clone();
                    let name = repo.name.clone();
                    match client.get_issue_comments(&owner, &name, number).await {
                        Ok(comments) => app.issue_comments = comments,
                        Err(e) => app.set_error(format!("failed to load comments: {}", e)),
                    }
                }
                app.state = AppState::ViewingIssue;
            }
        }
        KeyCode::Char('o') => {
            if let Some(issue) = app.issues.get(app.issues_selected) {
                let url = issue.html_url.clone();
                if let Err(e) = open::that(&url) {
                    app.set_error(format!("failed to open browser: {}", e));
                }
            }
        }
        _ => {}
    }
    Ok(false)
}

async fn handle_viewing_issue(app: &mut App, code: KeyCode) -> Result<bool> {
    match code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Esc | KeyCode::Char('h') => {
            app.current_issue = None;
            app.issue_scroll = 0;
            app.state = AppState::ViewingIssues;
        }
        KeyCode::Char('j') | KeyCode::Down | KeyCode::Char('J') => {
            app.issue_scroll = app.issue_scroll.saturating_add(1);
        }
        KeyCode::Char('k') | KeyCode::Up | KeyCode::Char('K') => {
            app.issue_scroll = app.issue_scroll.saturating_sub(1);
        }
        _ => {}
    }
    Ok(false)
}

async fn handle_creating_issue(
    app: &mut App,
    code: KeyCode,
    client: &GithubClient,
) -> Result<bool> {
    match code {
        KeyCode::Esc => {
            app.state = AppState::ViewingIssues;
        }
        KeyCode::Tab => {
            app.new_issue_focus_body = !app.new_issue_focus_body;
        }
        KeyCode::Backspace => {
            if app.new_issue_focus_body {
                app.new_issue_body.backspace();
            } else {
                app.new_issue_title.backspace();
            }
        }
        KeyCode::Delete => {
            if app.new_issue_focus_body {
                app.new_issue_body.delete();
            } else {
                app.new_issue_title.delete();
            }
        }
        KeyCode::Left => {
            if app.new_issue_focus_body {
                app.new_issue_body.move_left();
            } else {
                app.new_issue_title.move_left();
            }
        }
        KeyCode::Right => {
            if app.new_issue_focus_body {
                app.new_issue_body.move_right();
            } else {
                app.new_issue_title.move_right();
            }
        }
        KeyCode::Home => {
            if app.new_issue_focus_body {
                app.new_issue_body.move_home();
            } else {
                app.new_issue_title.move_home();
            }
        }
        KeyCode::End => {
            if app.new_issue_focus_body {
                app.new_issue_body.move_end();
            } else {
                app.new_issue_title.move_end();
            }
        }
        KeyCode::Char(c) => {
            if app.new_issue_focus_body {
                app.new_issue_body.insert(c);
            } else {
                app.new_issue_title.insert(c);
            }
        }
        KeyCode::Enter => {
            if !app.new_issue_focus_body {
                // Enter on title field: switch focus to body
                app.new_issue_focus_body = true;
            } else {
                let title = app.new_issue_title.as_str().trim().to_string();
                if title.is_empty() {
                    app.set_error("title cannot be empty");
                    return Ok(false);
                }
                let body = app.new_issue_body.as_str().trim().to_string();
                if let Some(repo) = app.active_repo() {
                    let owner = repo.owner.clone();
                    let name = repo.name.clone();
                    match client.create_issue(&owner, &name, &title, &body).await {
                        Ok(number) => {
                            app.set_status(format!("created issue #{}", number));
                            // reload issues list
                            let is_pr = app.issue_tab == IssueTab::PullRequests;
                            if let Ok(issues) = client
                                .list_issues(&owner, &name, &app.issue_filter, is_pr)
                                .await
                            {
                                app.issues = issues;
                                app.issues_selected = 0;
                            }
                            app.state = AppState::ViewingIssues;
                        }
                        Err(e) => app.set_error(format!("create issue failed: {}", e)),
                    }
                }
            }
        }
        _ => {}
    }
    Ok(false)
}

async fn handle_viewing_notifications(
    app: &mut App,
    code: KeyCode,
    client: &GithubClient,
) -> Result<bool> {
    match code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Esc | KeyCode::Char('h') => {
            app.state = match app.tab {
                Tab::MyRepos => AppState::MyRepos,
                Tab::Search => AppState::Browsing,
            };
        }
        KeyCode::Char('1') => {
            app.tab = Tab::Search;
            app.state = AppState::Browsing;
        }
        KeyCode::Char('2') => {
            return switch_to_my_repos(app, client).await;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.notifications.is_empty() {
                app.notifications_selected =
                    (app.notifications_selected + 1) % app.notifications.len();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if !app.notifications.is_empty() {
                if app.notifications_selected == 0 {
                    app.notifications_selected = app.notifications.len() - 1;
                } else {
                    app.notifications_selected -= 1;
                }
            }
        }
        // r: mark selected as read
        KeyCode::Char('r') => {
            if let Some(notif) = app.notifications.get(app.notifications_selected).cloned() {
                match client.mark_notification_read(&notif.id).await {
                    Ok(()) => {
                        if let Some(n) = app.notifications.get_mut(app.notifications_selected) {
                            n.unread = false;
                        }
                        app.set_status("marked as read");
                    }
                    Err(e) => app.set_error(format!("mark read failed: {}", e)),
                }
            }
        }
        // R: mark all as read
        KeyCode::Char('R') => match client.mark_all_notifications_read().await {
            Ok(()) => {
                for n in &mut app.notifications {
                    n.unread = false;
                }
                app.set_status("all marked as read");
            }
            Err(e) => app.set_error(format!("mark all read failed: {}", e)),
        },
        // f: toggle unread-only filter
        KeyCode::Char('f') => {
            app.notifications_unread_only = !app.notifications_unread_only;
            open_notifications(app, client).await;
        }
        // o: open in browser
        KeyCode::Char('o') => {
            if let Some(notif) = app.notifications.get(app.notifications_selected) {
                if let Some(url) = &notif.subject_url {
                    // GitHub API gives API urls; convert to html url best-effort
                    let html_url = url
                        .replace("api.github.com/repos", "github.com")
                        .replace("/pulls/", "/pull/")
                        .replace("/issues/", "/issues/");
                    if let Err(e) = open::that(&html_url) {
                        app.set_error(format!("failed to open browser: {}", e));
                    }
                }
            }
        }
        _ => {}
    }
    Ok(false)
}
