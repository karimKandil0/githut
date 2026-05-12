use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::markdown;
use crate::types::{AppState, EntryType, IssueTab, SparseStep, Tab};

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Outer: tab bar + main + status bar
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let tab_area = outer[0];
    let main_area = outer[1];
    let status_area = outer[2];

    draw_tab_bar(f, app, tab_area);

    // Main: left pane + right readme pane
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_area);

    let left_area = panes[0];
    let right_area = panes[1];

    if matches!(
        app.state,
        AppState::SearchingCode | AppState::ViewingCodeResults
    ) {
        draw_code_search(f, app, left_area, right_area);
    } else if matches!(app.state, AppState::ViewingIssues) {
        draw_issues_list(f, app, left_area);
        draw_issue_preview(f, app, right_area);
    } else if matches!(app.state, AppState::ViewingIssue) {
        draw_issues_list(f, app, left_area);
        draw_issue_detail(f, app, right_area);
    } else if matches!(app.state, AppState::ViewingNotifications) {
        draw_notifications(f, app, main_area);
    } else if matches!(app.state, AppState::FileBrowsing) {
        draw_file_browser(f, app, left_area);
        draw_file_content(f, app, right_area);
    } else if matches!(app.state, AppState::ViewingProfile) {
        let left_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(6), Constraint::Min(0)])
            .split(left_area);
        draw_profile_header(f, app, left_split[0]);
        draw_profile_repos(f, app, left_split[1]);
        draw_readme(f, app, right_area);
    } else if app.tab == Tab::MyRepos {
        // Left: search bar replaced by my repos header + list
        let left_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(left_area);
        draw_my_repos_header(f, app, left_split[0]);
        draw_my_repos_list(f, app, left_split[1]);
        draw_readme(f, app, right_area);
    } else {
        let left_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(left_area);
        draw_search_bar(f, app, left_split[0]);
        draw_results(f, app, left_split[1]);
        draw_readme(f, app, right_area);
    }

    draw_status_bar(f, app, status_area);

    // Overlays on top
    match &app.state.clone() {
        AppState::Error(msg) => draw_error_overlay(f, area, msg),
        AppState::Help => draw_help_overlay(f, area),
        AppState::Cloning => draw_input_overlay(
            f,
            area,
            "Clone path:",
            &app.clone_path_input.display_with_cursor(),
        ),
        AppState::SparseCloning => draw_sparse_overlay(f, area, app),
        AppState::FileSaving => draw_input_overlay(
            f,
            area,
            "Save file to path:",
            &app.file_save_path_input.display_with_cursor(),
        ),
        AppState::Renaming => draw_input_overlay(
            f,
            area,
            "Rename repo:",
            &app.rename_input.display_with_cursor(),
        ),
        AppState::ConfirmDelete => {
            let name = app
                .selected_my_repo()
                .map(|r| r.full_name.as_str())
                .unwrap_or("this repo");
            draw_confirm_overlay(f, area, &format!("Delete {}? (y/n)", name));
        }
        AppState::CreatingIssue => draw_create_issue_overlay(f, area, app),
        AppState::CreatingRepo => draw_create_repo_overlay(f, area, app),
        _ => {}
    }
}

fn draw_profile_header(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(profile) = &app.profile_user else {
        let loading = if app.profile_loading {
            "Loading profile..."
        } else {
            ""
        };
        f.render_widget(
            Paragraph::new(loading).style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    };

    let name_line = Line::from(vec![
        Span::styled(
            profile.login.clone(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        if let Some(name) = &profile.name {
            Span::styled(format!("  ({})", name), Style::default().fg(Color::White))
        } else {
            Span::raw("")
        },
    ]);

    let stats_line = Line::from(Span::styled(
        format!(
            "repos: {}  followers: {}  following: {}",
            profile.public_repos, profile.followers, profile.following
        ),
        Style::default().fg(Color::DarkGray),
    ));

    let mut lines = vec![name_line, stats_line];
    if let Some(bio) = &profile.bio {
        if !bio.is_empty() {
            lines.push(Line::from(Span::styled(
                bio.chars().take(80).collect::<String>(),
                Style::default().fg(Color::White),
            )));
        }
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_profile_repos(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .profile_repos
        .iter()
        .map(|repo| {
            let lang = repo.language.as_deref().unwrap_or("—");
            let stars = format_stars(repo.stargazers_count);
            let is_starred = app.starred.contains(&repo.full_name);

            let star_span = if is_starred {
                Span::styled("★ ", Style::default().fg(Color::Yellow))
            } else {
                Span::styled("  ", Style::default())
            };

            let mut badges = String::new();
            if repo.archived {
                badges.push_str("[archived] ");
            }
            if repo.fork {
                badges.push_str("[fork] ");
            }

            let line1 = Line::from(vec![
                star_span,
                Span::styled(
                    format!("{:<38}", repo.name),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("{:<12}", lang), Style::default().fg(Color::Green)),
                Span::styled(stars, Style::default().fg(Color::Yellow)),
            ]);
            let desc = repo
                .description
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(60)
                .collect::<String>();
            let line2 = Line::from(vec![
                Span::styled(
                    format!("  {}", badges),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(desc, Style::default().fg(Color::DarkGray)),
            ]);
            ListItem::new(vec![line1, line2])
        })
        .collect();

    let mut state = ListState::default();
    if !app.profile_repos.is_empty() {
        state.select(Some(app.profile_repos_selected));
    }

    let login = app
        .profile_user
        .as_ref()
        .map(|p| p.login.as_str())
        .unwrap_or("user");
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(format!(
            "{}'s repos ({})",
            login,
            app.profile_repos.len()
        )))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_tab_bar(f: &mut Frame, app: &App, area: Rect) {
    let notifs_active = matches!(app.state, AppState::ViewingNotifications);
    let tabs = vec![
        ("1", "Search", app.tab == Tab::Search && !notifs_active),
        ("2", "My Repos", app.tab == Tab::MyRepos && !notifs_active),
        ("3", "Notifications", notifs_active),
    ];
    let mut spans = vec![Span::raw(" ")];
    for (key, label, active) in tabs {
        if active {
            spans.push(Span::styled(
                format!("[{}:{}]", key, label),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {}:{} ", key, label),
                Style::default().fg(Color::DarkGray),
            ));
        }
        spans.push(Span::raw("  "));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_my_repos_header(f: &mut Frame, app: &App, area: Rect) {
    let loading = if app.my_repos_loading {
        " [loading...]"
    } else {
        ""
    };
    let title = format!("My Repos{}", loading);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(title);
    f.render_widget(block, area);
}

fn draw_my_repos_list(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .my_repos
        .iter()
        .map(|repo| {
            let lang = repo.language.as_deref().unwrap_or("—");
            let stars = format_stars(repo.stargazers_count);
            let is_starred = app.starred.contains(&repo.full_name);

            let mut badges = String::new();
            if repo.archived {
                badges.push_str("[archived] ");
            }
            if repo.fork {
                badges.push_str("[fork] ");
            }

            let star_span = if is_starred {
                Span::styled("★ ", Style::default().fg(Color::Yellow))
            } else {
                Span::styled("  ", Style::default())
            };

            let line1 = Line::from(vec![
                star_span,
                Span::styled(
                    format!("{:<38}", repo.name),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("{:<12}", lang), Style::default().fg(Color::Green)),
                Span::styled(stars, Style::default().fg(Color::Yellow)),
            ]);
            let desc = repo
                .description
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(60)
                .collect::<String>();
            let line2 = Line::from(vec![
                Span::styled(
                    format!("  {}", badges),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(desc, Style::default().fg(Color::DarkGray)),
            ]);
            ListItem::new(vec![line1, line2])
        })
        .collect();

    let mut state = ListState::default();
    if !app.my_repos.is_empty() {
        state.select(Some(app.my_repos_selected));
    }

    let count = app.my_repos.len();
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Repos ({})", count)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let is_searching = matches!(app.state, AppState::Searching);
    let border_style = if is_searching {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let text = if app.search_query.is_empty() && !is_searching {
        "Press / to search...".to_string()
    } else if is_searching {
        app.search_query.display_with_cursor()
    } else {
        app.search_query.as_str().to_string()
    };

    let lang_label = match app.current_language() {
        Some(l) => format!("Search [{}]", l),
        None => "Search".to_string(),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(lang_label);

    let para = Paragraph::new(text).block(block);
    f.render_widget(para, area);
}

fn draw_results(f: &mut Frame, app: &mut App, area: Rect) {
    let loading_msg = if app.loading { " [loading...]" } else { "" };

    let title = format!("Results ({}){}", app.results.len(), loading_msg);

    let items: Vec<ListItem> = app
        .results
        .iter()
        .map(|repo| {
            let lang = repo.language.as_deref().unwrap_or("—");
            let stars = format_stars(repo.stargazers_count);
            let is_starred = app.starred.contains(&repo.full_name);
            let desc = repo
                .description
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(60)
                .collect::<String>();

            let star_span = if is_starred {
                Span::styled("★ ", Style::default().fg(Color::Yellow))
            } else {
                Span::styled("  ", Style::default())
            };

            let line1 = Line::from(vec![
                star_span,
                Span::styled(
                    format!("{:<38}", repo.full_name),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("{:<12}", lang), Style::default().fg(Color::Green)),
                Span::styled(stars, Style::default().fg(Color::Yellow)),
            ]);
            let line2 = Line::from(Span::styled(
                format!("  {}", desc),
                Style::default().fg(Color::DarkGray),
            ));

            ListItem::new(vec![line1, line2])
        })
        .collect();

    let mut state = ListState::default();
    if !app.results.is_empty() {
        state.select(Some(app.selected));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_readme(f: &mut Frame, app: &App, area: Rect) {
    let title = match app.selected_repo() {
        Some(repo) => format!("README — {}", repo.full_name),
        None => "README".to_string(),
    };

    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let text: Text = match &app.readme_content {
        Some(md) => Text::from(markdown::render(md)),
        None => {
            let msg = if app.loading {
                "Loading README..."
            } else if app.selected_repo().is_some() {
                "Loading README..."
            } else {
                "Search for repos to get started"
            };
            Text::from(Line::from(Span::styled(
                msg,
                Style::default().fg(Color::DarkGray),
            )))
        }
    };

    let para = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .scroll((app.readme_scroll, 0));
    f.render_widget(para, inner);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    if let Some(msg) = &app.status_msg {
        // show rate limit on the right if available
        let rl_text = app.rate_limit.as_ref().map(|rl| {
            format!(
                " search:{}/{} core:{}/{} ",
                rl.search_remaining, rl.search_limit, rl.core_remaining, rl.core_limit
            )
        });

        if let Some(rl) = rl_text {
            let left = Paragraph::new(msg.as_str()).style(Style::default().fg(Color::Yellow));
            let right = Paragraph::new(rl)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(ratatui::layout::Alignment::Right);
            f.render_widget(left, area);
            f.render_widget(right, area);
        } else {
            let para = Paragraph::new(msg.as_str()).style(Style::default().fg(Color::Yellow));
            f.render_widget(para, area);
        }
        return;
    }

    // no status msg — show rate limit on right
    if let Some(rl) = &app.rate_limit {
        let rl_text = format!(
            " search:{}/{} core:{}/{} ",
            rl.search_remaining, rl.search_limit, rl.core_remaining, rl.core_limit
        );
        let right = Paragraph::new(rl_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Right);
        f.render_widget(right, area);
    }

    let pairs: &[(&str, &str)] = match &app.state {
        AppState::Searching => &[
            ("Enter", "search"),
            ("Tab", "language filter"),
            ("Esc", "cancel"),
        ],
        AppState::Browsing => &[
            ("/", "search"),
            ("Tab", "language"),
            ("j/k", "nav"),
            ("J/K", "scroll readme"),
            ("l", "browse files"),
            ("i", "issues/PRs"),
            ("S", "code search"),
            ("u", "view profile"),
            ("s", "star/unstar"),
            ("f", "fork"),
            ("c", "clone"),
            ("C", "sparse clone"),
            ("o", "browser"),
            ("r", "refresh"),
            ("3", "notifications"),
            ("?", "help"),
            ("q", "quit"),
        ],
        AppState::FileBrowsing => &[
            ("j/k", "nav"),
            ("J/K", "scroll preview"),
            ("l", "open"),
            ("c", "save file"),
            ("h", "up / back"),
            ("Esc", "back to repos"),
            ("q", "quit"),
        ],
        AppState::MyRepos => &[
            ("j/k", "nav"),
            ("J/K", "scroll readme"),
            ("l", "browse files"),
            ("u", "view profile"),
            ("c", "clone"),
            ("R", "rename"),
            ("D", "delete"),
            ("A", "archive"),
            ("o", "browser"),
            ("?", "help"),
            ("q", "quit"),
        ],
        AppState::ViewingProfile => &[
            ("j/k", "nav"),
            ("J/K", "scroll readme"),
            ("l", "browse files"),
            ("u", "go to owner"),
            ("c", "clone"),
            ("o", "open profile"),
            ("Esc/h", "back"),
            ("?", "help"),
            ("q", "quit"),
        ],
        AppState::ViewingIssues => &[
            ("j/k", "nav"),
            ("l/Enter", "open issue"),
            ("Tab", "issues/PRs"),
            ("f", "filter open/closed/all"),
            ("n", "new issue"),
            ("x", "close issue"),
            ("o", "browser"),
            ("Esc/h", "back"),
            ("q", "quit"),
        ],
        AppState::ViewingIssue => &[
            ("j/k/J/K", "scroll"),
            ("Esc/h", "back to list"),
            ("q", "quit"),
        ],
        AppState::ViewingNotifications => &[
            ("j/k", "nav"),
            ("r", "mark read"),
            ("R", "mark all read"),
            ("f", "toggle unread filter"),
            ("o", "browser"),
            ("Esc/h", "back"),
            ("q", "quit"),
        ],
        AppState::CreatingIssue => &[
            ("Tab", "switch title/body"),
            ("Enter (title)", "go to body"),
            ("Enter (body)", "submit"),
            ("Esc", "cancel"),
        ],
        AppState::SearchingCode => &[("Enter", "search"), ("Esc", "back")],
        AppState::ViewingCodeResults => &[
            ("j/k", "nav"),
            ("l/Enter", "load file"),
            ("J/K", "scroll file"),
            ("o", "browser"),
            ("h/Esc", "back to search"),
            ("q", "quit"),
        ],
        AppState::CreatingRepo => &[
            ("Tab", "next field"),
            ("Space (private)", "toggle"),
            ("Enter", "create"),
            ("Esc", "cancel"),
        ],
        AppState::Cloning | AppState::SparseCloning | AppState::FileSaving | AppState::Renaming => {
            &[("Enter", "confirm"), ("Esc", "cancel")]
        }
        AppState::ConfirmDelete => &[("y", "confirm delete"), ("n / Esc", "cancel")],
        AppState::Error(_) => &[("any key", "dismiss")],
        AppState::Help => &[("any key", "close")],
        _ => &[("q", "quit")],
    };

    let mut spans = vec![Span::raw(" ")];
    for (i, (key, action)) in pairs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(
            *key,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(":{}", action),
            Style::default().fg(Color::DarkGray),
        ));
    }

    let para = Paragraph::new(Line::from(spans));
    f.render_widget(para, area);
}

fn draw_file_browser(f: &mut Frame, app: &mut App, area: Rect) {
    let repo_name = app
        .active_repo()
        .map(|r| r.full_name.clone())
        .unwrap_or_default();
    let current_path = app.current_file_path().to_string();
    let title = if current_path.is_empty() {
        format!("Files — {}", repo_name)
    } else {
        format!("Files — {}/{}", repo_name, current_path)
    };

    let items: Vec<ListItem> = app
        .file_entries
        .iter()
        .map(|entry| {
            let (prefix, style) = match entry.entry_type {
                EntryType::Dir => (
                    "▶ ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                EntryType::File => (" ", Style::default().fg(Color::White)),
            };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(entry.name.clone(), style),
            ]))
        })
        .collect();

    let mut state = ListState::default();
    if !app.file_entries.is_empty() {
        state.select(Some(app.file_selected));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_file_content(f: &mut Frame, app: &App, area: Rect) {
    let title = app
        .selected_entry()
        .map(|e| e.path.clone())
        .unwrap_or_else(|| "Preview".to_string());

    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let text: Text = match &app.file_content {
        Some(content) => {
            let is_md = app
                .selected_entry()
                .map(|e| e.name.ends_with(".md") || e.name.ends_with(".markdown"))
                .unwrap_or(false);
            if is_md {
                Text::from(markdown::render(content))
            } else {
                Text::raw(content.clone())
            }
        }
        None => {
            let msg = if app.loading {
                "Loading..."
            } else {
                match app.selected_entry() {
                    Some(e) if e.entry_type == EntryType::Dir => "Press l to enter directory",
                    Some(_) => "Press l to preview file",
                    None => "",
                }
            };
            Text::from(Line::from(Span::styled(
                msg,
                Style::default().fg(Color::DarkGray),
            )))
        }
    };

    let para = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .scroll((app.file_scroll, 0));
    f.render_widget(para, inner);
}

fn draw_sparse_overlay(f: &mut Frame, area: Rect, app: &App) {
    let popup = centered_rect(55, 25, area);
    f.render_widget(Clear, popup);

    let (title, input_display, hint) = match app.sparse_step {
        SparseStep::Path => (
            "Sparse clone — step 1/2: path",
            app.sparse_path_input.display_with_cursor(),
            "Enter destination path, then press Enter",
        ),
        SparseStep::Dirs => (
            "Sparse clone — step 2/2: directories",
            app.sparse_dirs_input.display_with_cursor(),
            "Space-separated dirs (e.g. src docs). Leave empty for all.",
        ),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(title);
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    f.render_widget(Paragraph::new(input_display), layout[0]);
    f.render_widget(
        Paragraph::new(hint).style(Style::default().fg(Color::DarkGray)),
        layout[1],
    );
}

fn draw_confirm_overlay(f: &mut Frame, area: Rect, msg: &str) {
    let popup = centered_rect(55, 15, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title("Confirm");
    let para = Paragraph::new(msg).block(block).wrap(Wrap { trim: true });
    f.render_widget(para, popup);
}

fn draw_error_overlay(f: &mut Frame, area: Rect, msg: &str) {
    let popup = centered_rect(60, 30, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title("Error — press any key to dismiss");
    let para = Paragraph::new(msg).block(block).wrap(Wrap { trim: true });
    f.render_widget(para, popup);
}

fn draw_help_overlay(f: &mut Frame, area: Rect) {
    let popup = centered_rect(52, 75, area);
    f.render_widget(Clear, popup);

    fn section(title: &'static str) -> Line<'static> {
        Line::from(Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
    }
    fn row(key: &'static str, action: &'static str) -> Line<'static> {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{:<14}", key),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(action, Style::default().fg(Color::DarkGray)),
        ])
    }

    let lines: Vec<Line> = vec![
        section("search"),
        row("/", "focus search input"),
        row("Enter", "confirm search"),
        row("Tab", "cycle language filter"),
        row("r", "refresh results"),
        Line::raw(""),
        section("navigation"),
        row("j / k", "move up / down"),
        row("J / K", "scroll preview pane"),
        row("l / Enter", "open file browser"),
        row("h", "go up one dir / back to repos"),
        row("Esc", "back / close overlay"),
        Line::raw(""),
        section("repo actions"),
        row("c", "clone to local path"),
        row("C", "sparse clone (path + dirs)"),
        row("s", "star / unstar"),
        row("f", "fork"),
        row("o", "open in browser"),
        Line::raw(""),
        section("file browser"),
        row("l / Enter", "enter dir / preview file"),
        row("c", "save file to local path"),
        row("h", "go up one dir"),
        Line::raw(""),
        section("profile view (u on any repo)"),
        row("u", "go to that repo's owner"),
        row("o", "open profile in browser"),
        Line::raw(""),
        section("my repos (tab 2)"),
        row("R", "rename repo"),
        row("D", "delete repo (confirms y/n)"),
        row("A", "archive / unarchive"),
        Line::raw(""),
        section("issues & PRs (i on any repo)"),
        row("Tab", "toggle issues / PRs"),
        row("f", "cycle filter open/closed/all"),
        row("n", "create new issue"),
        row("x", "close selected issue"),
        row("l/Enter", "open issue + comments"),
        Line::raw(""),
        section("notifications (3 from anywhere)"),
        row("r", "mark selected as read"),
        row("R", "mark all as read"),
        row("f", "toggle unread-only filter"),
        row("o", "open in browser"),
        Line::raw(""),
        section("general"),
        row("1 / 2 / 3", "switch tabs"),
        row("?", "toggle this help"),
        row("q", "quit"),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title("Help — press any key to close");
    let para = Paragraph::new(lines).block(block);
    f.render_widget(para, popup);
}

fn draw_issues_list(f: &mut Frame, app: &mut App, area: Rect) {
    let repo_name = app
        .active_repo()
        .map(|r| r.full_name.clone())
        .unwrap_or_default();

    let loading_suffix = if app.issues_loading {
        " loading..."
    } else {
        ""
    };
    let tab_label = match app.issue_tab {
        IssueTab::Issues => "[Issues] PRs  ",
        IssueTab::PullRequests => " Issues [PRs] ",
    };
    let title = format!(
        "{}  {}  {} ({}){}",
        repo_name,
        tab_label,
        app.issue_filter.label(),
        app.issues.len(),
        loading_suffix
    );

    let items: Vec<ListItem> = app
        .issues
        .iter()
        .map(|issue| {
            let state_color = if issue.state == "open" {
                Color::Green
            } else {
                Color::Red
            };
            let state_sym = if issue.state == "open" {
                "● "
            } else {
                "○ "
            };

            let labels: String = if issue.labels.is_empty() {
                String::new()
            } else {
                format!(" [{}]", issue.labels.join(", "))
            };

            let line1 = Line::from(vec![
                Span::styled(state_sym, Style::default().fg(state_color)),
                Span::styled(
                    format!("#{:<6}", issue.number),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    issue.title.chars().take(50).collect::<String>(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(labels, Style::default().fg(Color::Cyan)),
            ]);
            let line2 = Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("@{}  ", issue.user_login),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("💬 {}  ", issue.comments),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    issue.created_at.get(..10).unwrap_or("").to_string(),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            ListItem::new(vec![line1, line2])
        })
        .collect();

    let mut state = ListState::default();
    if !app.issues.is_empty() {
        state.select(Some(app.issues_selected));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_issue_preview(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(match app.issue_tab {
            IssueTab::Issues => "Issue Preview — l/Enter to open",
            IssueTab::PullRequests => "PR Preview — l/Enter to open",
        });
    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = match app.issues.get(app.issues_selected) {
        Some(issue) => {
            let mut lines = vec![
                Line::from(Span::styled(
                    format!("#{} {}", issue.number, issue.title),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!(
                        "@{}  {}  {}",
                        issue.user_login,
                        issue.state,
                        &issue.created_at.get(..10).unwrap_or("")
                    ),
                    Style::default().fg(Color::DarkGray),
                )),
                Line::raw(""),
            ];
            if let Some(body) = &issue.body {
                for line in body.lines().take(30) {
                    lines.push(Line::from(Span::raw(line.to_string())));
                }
            } else {
                lines.push(Line::from(Span::styled(
                    "(no description)",
                    Style::default().fg(Color::DarkGray),
                )));
            }
            Text::from(lines)
        }
        None => Text::from(Line::from(Span::styled(
            if app.issues_loading {
                "Loading..."
            } else {
                "No issues"
            },
            Style::default().fg(Color::DarkGray),
        ))),
    };

    f.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }), inner);
}

fn draw_issue_detail(f: &mut Frame, app: &App, area: Rect) {
    let Some(issue) = &app.current_issue else {
        f.render_widget(
            Paragraph::new("").block(Block::default().borders(Borders::ALL)),
            area,
        );
        return;
    };

    let title = format!(
        "#{} — {} — {} comment(s)",
        issue.number,
        issue.state,
        app.issue_comments.len()
    );
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines = vec![
        Line::from(Span::styled(
            issue.title.clone(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!(
                "@{}  {}  {}",
                issue.user_login,
                issue.state,
                &issue.created_at.get(..10).unwrap_or("")
            ),
            Style::default().fg(Color::DarkGray),
        )),
        Line::raw(""),
    ];

    if let Some(body) = &issue.body {
        for line in body.lines() {
            lines.push(Line::from(Span::raw(line.to_string())));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "(no description)",
            Style::default().fg(Color::DarkGray),
        )));
    }

    if !app.issue_comments.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "── comments ──",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        for comment in &app.issue_comments {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                format!(
                    "@{}  {}",
                    comment.user_login,
                    &comment.created_at.get(..10).unwrap_or("")
                ),
                Style::default().fg(Color::Yellow),
            )));
            for line in comment.body.lines() {
                lines.push(Line::from(Span::raw(line.to_string())));
            }
        }
    }

    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((app.issue_scroll, 0)),
        inner,
    );
}

fn draw_notifications(f: &mut Frame, app: &mut App, area: Rect) {
    let filter_label = if app.notifications_unread_only {
        "Unread"
    } else {
        "All"
    };
    let loading_suffix = if app.notifications_loading {
        " [loading...]"
    } else {
        ""
    };
    let title = format!(
        "Notifications [{}] ({}){}",
        filter_label,
        app.notifications.len(),
        loading_suffix
    );

    let items: Vec<ListItem> = app
        .notifications
        .iter()
        .map(|notif| {
            let unread_sym = if notif.unread {
                Span::styled("● ", Style::default().fg(Color::Cyan))
            } else {
                Span::styled("○ ", Style::default().fg(Color::DarkGray))
            };

            let type_color = match notif.subject_type.as_str() {
                "PullRequest" => Color::Magenta,
                "Issue" => Color::Green,
                "Release" => Color::Yellow,
                _ => Color::DarkGray,
            };

            let line1 = Line::from(vec![
                unread_sym,
                Span::styled(
                    format!("[{:<12}]  ", notif.subject_type),
                    Style::default().fg(type_color),
                ),
                Span::styled(
                    notif.subject_title.chars().take(55).collect::<String>(),
                    Style::default()
                        .fg(if notif.unread {
                            Color::White
                        } else {
                            Color::DarkGray
                        })
                        .add_modifier(if notif.unread {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
            ]);
            let line2 = Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("{}  ", notif.repo_full_name),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    format!("reason: {}  ", notif.reason),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    notif.updated_at.get(..10).unwrap_or("").to_string(),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            ListItem::new(vec![line1, line2])
        })
        .collect();

    let mut state = ListState::default();
    if !app.notifications.is_empty() {
        state.select(Some(app.notifications_selected));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, area, &mut state);
}

fn draw_create_issue_overlay(f: &mut Frame, area: Rect, app: &App) {
    let popup = centered_rect(65, 35, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title("New Issue — Tab to switch fields, Enter(body) to submit, Esc to cancel");
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // "Title:" label
            Constraint::Length(1), // title input
            Constraint::Length(1), // spacer
            Constraint::Length(1), // "Body:" label
            Constraint::Min(0),    // body input
        ])
        .split(inner);

    let title_style = if !app.new_issue_focus_body {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let body_style = if app.new_issue_focus_body {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    f.render_widget(Paragraph::new("Title:").style(title_style), layout[0]);
    f.render_widget(
        Paragraph::new(app.new_issue_title.display_with_cursor()),
        layout[1],
    );
    f.render_widget(Paragraph::new("Body:").style(body_style), layout[3]);
    f.render_widget(
        Paragraph::new(app.new_issue_body.display_with_cursor()).wrap(Wrap { trim: false }),
        layout[4],
    );
}

fn draw_code_search(f: &mut Frame, app: &mut App, left_area: Rect, right_area: Rect) {
    // Left: search bar + results list
    let left_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(left_area);

    let repo_label = format!("{}/{}", app.code_repo_owner, app.code_repo_name);
    let is_searching = matches!(app.state, AppState::SearchingCode);
    let border_style = if is_searching {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let input_text = if is_searching {
        app.code_query.display_with_cursor()
    } else {
        app.code_query.as_str().to_string()
    };
    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(format!("Code search — {}", repo_label));
    f.render_widget(
        Paragraph::new(input_text).block(search_block),
        left_split[0],
    );

    let loading_suffix = if app.code_loading {
        " [loading...]"
    } else {
        ""
    };
    let items: Vec<ListItem> = app
        .code_results
        .iter()
        .map(|r| {
            ListItem::new(Line::from(Span::styled(
                r.path.clone(),
                Style::default().fg(Color::Cyan),
            )))
        })
        .collect();
    let mut state = ListState::default();
    if !app.code_results.is_empty() {
        state.select(Some(app.code_selected));
    }
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(format!(
            "Results ({}){}",
            app.code_results.len(),
            loading_suffix
        )))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );
    f.render_stateful_widget(list, left_split[1], &mut state);

    // Right: file content
    let selected_path = app
        .code_results
        .get(app.code_selected)
        .map(|r| r.path.clone())
        .unwrap_or_else(|| "Preview".to_string());
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("{} — press l/Enter to load", selected_path));
    let inner = block.inner(right_area);
    f.render_widget(block, right_area);

    let text: Text = match &app.file_content {
        Some(content) => {
            let is_md = selected_path.ends_with(".md") || selected_path.ends_with(".markdown");
            if is_md {
                Text::from(markdown::render(content))
            } else {
                Text::raw(content.clone())
            }
        }
        None => Text::from(Line::from(Span::styled(
            if app.loading {
                "Loading..."
            } else {
                "press l/Enter to load file"
            },
            Style::default().fg(Color::DarkGray),
        ))),
    };
    f.render_widget(
        Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .scroll((app.file_scroll, 0)),
        inner,
    );
}

fn draw_create_repo_overlay(f: &mut Frame, area: Rect, app: &App) {
    let popup = centered_rect(60, 30, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title("New Repo — Tab to switch fields, Enter to create, Esc to cancel");
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Name:
            Constraint::Length(1), // name input
            Constraint::Length(1), // spacer
            Constraint::Length(1), // Description:
            Constraint::Length(1), // desc input
            Constraint::Length(1), // spacer
            Constraint::Length(1), // Private:
        ])
        .split(inner);

    let focused = |i: u8| {
        if app.new_repo_focus == i {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        }
    };

    f.render_widget(Paragraph::new("Name:").style(focused(0)), layout[0]);
    f.render_widget(
        Paragraph::new(app.new_repo_name.display_with_cursor()),
        layout[1],
    );
    f.render_widget(Paragraph::new("Description:").style(focused(1)), layout[3]);
    f.render_widget(
        Paragraph::new(app.new_repo_desc.display_with_cursor()),
        layout[4],
    );

    let private_label = if app.new_repo_private {
        "Private: [x]  (Space to toggle)"
    } else {
        "Private: [ ]  (Space to toggle)"
    };
    f.render_widget(Paragraph::new(private_label).style(focused(2)), layout[6]);
}

fn draw_input_overlay(f: &mut Frame, area: Rect, prompt: &str, input: &str) {
    let popup = centered_rect(50, 10, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(format!("{} (Enter to confirm, Esc to cancel)", prompt));
    let para = Paragraph::new(input.to_string()).block(block);
    f.render_widget(para, popup);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn format_stars(n: u64) -> String {
    if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1000.0)
    } else {
        n.to_string()
    }
}
