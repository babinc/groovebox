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

fn has_brew() -> bool {
    Command::new("brew").arg("--version").output()
        .map(|o| o.status.success()).unwrap_or(false)
}

fn has_apt() -> bool {
    Command::new("apt").arg("--version").output()
        .map(|o| o.status.success()).unwrap_or(false)
}

fn has_dnf() -> bool {
    Command::new("dnf").arg("--version").output()
        .map(|o| o.status.success()).unwrap_or(false)
}

fn has_pacman() -> bool {
    Command::new("pacman").arg("--version").output()
        .map(|o| o.status.success()).unwrap_or(false)
}

fn check_dependencies() -> Result<()> {
    let required = ["yt-dlp", "mpv", "ffmpeg"];
    let missing: Vec<&str> = required.iter().filter(|d| !check_dependency(d)).copied().collect();

    if missing.is_empty() {
        return Ok(());
    }

    eprintln!("groovebox: missing dependencies: {}", missing.join(", "));

    // Detect package manager and build install command
    let install_cmd = if has_brew() {
        let pkgs = missing.join(" ");
        Some(format!("brew install {pkgs}"))
    } else if has_apt() {
        let pkgs: Vec<&str> = missing.iter().map(|&d| match d {
            "yt-dlp" => "yt-dlp",
            other => other,
        }).collect();
        Some(format!("sudo apt install -y {}", pkgs.join(" ")))
    } else if has_dnf() {
        let pkgs = missing.join(" ");
        Some(format!("sudo dnf install -y {pkgs}"))
    } else if has_pacman() {
        let pkgs = missing.join(" ");
        Some(format!("sudo pacman -S --noconfirm {pkgs}"))
    } else {
        None
    };

    let Some(cmd) = install_cmd else {
        eprintln!("Please install them manually and try again.");
        std::process::exit(1);
    };

    eprintln!("\nInstall now with: {cmd}");
    eprint!("Run this command? [Y/n] ");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if !input.is_empty() && input != "y" && input != "yes" {
        eprintln!("Please install dependencies manually and try again.");
        std::process::exit(1);
    }

    eprintln!("Installing...");
    let status = if cmd.starts_with("sudo") {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        Command::new(parts[0]).args(&parts[1..]).status()?
    } else {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        Command::new(parts[0]).args(&parts[1..]).status()?
    };

    if !status.success() {
        eprintln!("Installation failed. Please install manually and try again.");
        std::process::exit(1);
    }

    // Verify everything got installed
    let still_missing: Vec<&str> = missing.iter().filter(|d| !check_dependency(d)).copied().collect();
    if !still_missing.is_empty() {
        eprintln!("Still missing after install: {}", still_missing.join(", "));
        eprintln!("You may need to install these manually.");
        std::process::exit(1);
    }

    eprintln!("All dependencies installed!\n");
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
