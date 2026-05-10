use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::markdown;
use crate::types::{AppState, EntryType};

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Outer: main + status bar
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let main_area = outer[0];
    let status_area = outer[1];

    // Main: left results pane + right readme pane
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_area);

    let left_area = panes[0];
    let right_area = panes[1];

    // Left: search bar + results list
    let left_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(left_area);

    let search_area = left_split[0];
    let list_area = left_split[1];

    if matches!(app.state, AppState::FileBrowsing) {
        draw_file_browser(f, app, left_area);
        draw_file_content(f, app, right_area);
    } else {
        draw_search_bar(f, app, search_area);
        draw_results(f, app, list_area);
        draw_readme(f, app, right_area);
    }
    draw_status_bar(f, app, status_area);

    // Overlays on top
    match &app.state.clone() {
        AppState::Error(msg) => draw_error_overlay(f, area, msg),
        AppState::Help => draw_help_overlay(f, area),
        AppState::Cloning => {
            draw_input_overlay(f, area, "Clone path:", &app.clone_path_input.clone())
        }
        _ => {}
    }
}

fn draw_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let is_searching = matches!(app.state, AppState::Searching);
    let border_style = if is_searching {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let query_display = if app.search_query.is_empty() && !is_searching {
        "Press / to search...".to_string()
    } else {
        app.search_query.clone()
    };

    let cursor = if is_searching { "_" } else { "" };
    let text = format!("{}{}", query_display, cursor);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title("Search");

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
            let desc = repo
                .description
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(60)
                .collect::<String>();

            let line1 = Line::from(vec![
                Span::styled(
                    format!("{:<40}", repo.full_name),
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
        let para = Paragraph::new(msg.as_str()).style(Style::default().fg(Color::Yellow));
        f.render_widget(para, area);
        return;
    }

    let pairs: &[(&str, &str)] = match &app.state {
        AppState::Searching => &[("Enter", "search"), ("Esc", "cancel")],
        AppState::Browsing => &[
            ("/", "search"),
            ("j/k", "nav"),
            ("J/K", "scroll readme"),
            ("l", "browse files"),
            ("c", "clone"),
            ("o", "browser"),
            ("r", "refresh"),
            ("?", "help"),
            ("q", "quit"),
        ],
        AppState::FileBrowsing => &[
            ("j/k", "nav"),
            ("J/K", "scroll preview"),
            ("l", "open"),
            ("h", "up / back"),
            ("Esc", "back to repos"),
            ("q", "quit"),
        ],
        AppState::Cloning => &[("Enter", "confirm"), ("Esc", "cancel")],
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
        .selected_repo()
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
    let popup = centered_rect(50, 60, area);
    f.render_widget(Clear, popup);
    let help_text = "\
 /:search        focus search input
 Enter           confirm search
 j/k             navigate list
 J/K             scroll preview pane
 l / Enter       open file browser for selected repo
 h               go up / back
 c               clone selected repo
 o               open in browser
 r               refresh results
 ?               toggle this help
 Esc             back / close overlay
 q               quit";
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title("Help — press any key to close");
    let para = Paragraph::new(help_text).block(block);
    f.render_widget(para, popup);
}

fn draw_input_overlay(f: &mut Frame, area: Rect, prompt: &str, input: &str) {
    let popup = centered_rect(50, 10, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(format!("{} (Enter to confirm, Esc to cancel)", prompt));
    let para = Paragraph::new(format!("{}_", input)).block(block);
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
