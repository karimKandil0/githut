use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use termimad::MadSkin;

use crate::app::App;
use crate::types::AppState;

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

    draw_search_bar(f, app, search_area);
    draw_results(f, app, list_area);
    draw_readme(f, app, right_area);
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

    let content = match &app.readme_content {
        Some(md) => render_markdown(md, inner.width as usize),
        None => {
            if app.loading {
                "Loading README...".to_string()
            } else if app.selected_repo().is_some() {
                "Press Enter to load README".to_string()
            } else {
                "Search for repos to get started".to_string()
            }
        }
    };

    let para = Paragraph::new(content)
        .wrap(Wrap { trim: false })
        .scroll((app.readme_scroll, 0));
    f.render_widget(para, inner);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let hints =
        " /:search  j/k:nav  Enter:readme  c:clone  s:star  f:fork  o:browser  ?:help  q:quit";
    let msg = app.status_msg.as_deref().unwrap_or(hints);
    let para = Paragraph::new(msg).style(Style::default().fg(Color::DarkGray));
    f.render_widget(para, area);
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
 Enter           confirm search / load readme
 j/k             navigate results
 c               clone selected repo
 s               star/unstar repo
 f               fork repo
 o               open in browser
 r               refresh results
 ?               toggle this help
 Esc             clear input / close overlay
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

fn render_markdown(md: &str, _width: usize) -> String {
    let skin = MadSkin::default();
    skin.text(md, None).to_string()
}
