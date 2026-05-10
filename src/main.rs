mod api;
mod app;
mod git;
mod markdown;
mod tui;
mod types;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

#[tokio::main]
async fn main() -> Result<()> {
    let token = api::auth::get_token().await.unwrap_or_else(|e| {
        eprintln!("auth error: {}", e);
        std::process::exit(1);
    });

    let client = api::GithubClient::new(&token).unwrap_or_else(|e| {
        eprintln!("client init error: {}", e);
        std::process::exit(1);
    });

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = app::App::new();

    let result = run_app(&mut terminal, &mut app, &client).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut app::App,
    client: &api::GithubClient,
) -> Result<()> {
    loop {
        terminal.draw(|f| tui::ui::draw(f, app))?;

        if tui::events::handle_events(app, client).await? {
            break;
        }
    }
    Ok(())
}
