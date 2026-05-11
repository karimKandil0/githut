mod api;
mod app;
mod config;
mod git;
mod input;
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

    let cfg = config::load().unwrap_or_else(|e| {
        eprintln!("config warning: {}", e);
        config::Config::default()
    });

    let mut app = app::App::new(cfg);

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

        // drain background task results (clone / sparse-clone)
        while let Ok(msg) = app.bg_rx.try_recv() {
            match msg {
                Ok(s) => app.set_status(s),
                Err(e) => app.set_error(e),
            }
        }

        // debounced fetch — fires 300ms after last j/k
        if app.take_readme_pending() {
            if let Some(repo) = app.active_repo() {
                let owner = repo.owner.clone();
                let name = repo.name.clone();
                let full_name = repo.full_name.clone();
                app.loading = true;
                terminal.draw(|f| tui::ui::draw(f, app))?;
                let (readme, starred) = tokio::join!(
                    client.get_readme(&owner, &name),
                    client.is_starred(&owner, &name),
                );
                if let Ok(md) = readme {
                    app.readme_content = Some(md);
                }
                if starred {
                    app.starred.insert(full_name);
                }
                app.loading = false;
            }
        }
    }
    Ok(())
}
