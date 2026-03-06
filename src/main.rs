#![allow(dead_code)]

mod app;
mod audio;
mod models;
mod storage;
mod ui;
mod youtube;

use std::io;
use std::process::Command;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

fn check_dependency(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn check_dependencies() -> Result<()> {
    let deps = [
        ("yt-dlp", "pip install yt-dlp"),
        ("mpv", "sudo apt install mpv"),
        ("ffmpeg", "sudo apt install ffmpeg"),
    ];

    let mut missing = Vec::new();
    for (name, install) in &deps {
        if !check_dependency(name) {
            missing.push(format!("  {name} — install with: {install}"));
        }
    }

    if !missing.is_empty() {
        eprintln!("groovebox: missing dependencies:");
        for m in &missing {
            eprintln!("{m}");
        }
        std::process::exit(1);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    check_dependencies()?;

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let mut app = app::App::new().await?;
    let result = app.run(&mut terminal).await;

    // Terminal teardown
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
