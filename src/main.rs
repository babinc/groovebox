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

/// Oldest yt-dlp known to play YouTube audio. Older builds resolve stream URLs
/// via the android_vr client, and YouTube now 403s those unless the request
/// uses small bounded Range headers — which mpv/ffmpeg never send. The result
/// is a track that loads but never advances. Distro packages lag badly here;
/// nightly is usually the only build that works.
const MIN_YTDLP_VERSION: (u32, u32, u32) = (2026, 8, 18);

/// Parse yt-dlp's date-based version ("2026.08.18" or "2026.08.18.122307").
fn parse_ytdlp_version(raw: &str) -> Option<(u32, u32, u32)> {
    let mut parts = raw.trim().split('.');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    Some((year, month, day))
}

/// Warn (but don't block) when yt-dlp is too old to stream. Non-fatal because
/// YouTube breaks things on its own schedule — a hard gate would just go stale.
fn check_ytdlp_version() {
    let Ok(output) = Command::new("yt-dlp").arg("--version").output() else {
        return;
    };
    let raw = String::from_utf8_lossy(&output.stdout);
    let Some(found) = parse_ytdlp_version(&raw) else {
        return;
    };
    if found >= MIN_YTDLP_VERSION {
        return;
    }

    let (y, m, d) = MIN_YTDLP_VERSION;
    eprintln!("groovebox: yt-dlp {} is too old — playback will fail with HTTP 403.", raw.trim());
    eprintln!("           Need {y}.{m:02}.{d:02} or newer. Your package manager's build is");
    eprintln!("           probably stale; install the nightly instead:");
    eprintln!();
    eprintln!("             pipx install --pip-args=--pre \"yt-dlp[default]\"");
    eprintln!();
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

    // yt-dlp is deliberately excluded from package-manager installs: distro
    // builds lag far behind YouTube's stream-URL changes, so installing one
    // here would just hand the user a yt-dlp that can't play anything. Point
    // at the nightly instead. See check_ytdlp_version().
    if missing.contains(&"yt-dlp") {
        eprintln!();
        eprintln!("Install yt-dlp with (your package manager's build is likely too old):");
        eprintln!();
        eprintln!("  pipx install --pip-args=--pre \"yt-dlp[default]\"");
        eprintln!();
    }

    let missing_pkgs: Vec<&str> = missing.iter().filter(|&&d| d != "yt-dlp").copied().collect();
    if missing_pkgs.is_empty() {
        std::process::exit(1);
    }

    // Detect package manager and build install command
    let pkgs = missing_pkgs.join(" ");
    let install_cmd = if has_brew() {
        Some(format!("brew install {pkgs}"))
    } else if has_apt() {
        Some(format!("sudo apt install -y {pkgs}"))
    } else if has_dnf() {
        Some(format!("sudo dnf install -y {pkgs}"))
    } else if has_pacman() {
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
    let still_missing: Vec<&str> = missing_pkgs.iter().filter(|d| !check_dependency(d)).copied().collect();
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
    check_ytdlp_version();

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
