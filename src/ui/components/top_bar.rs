use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::app::state::{AppState, RepeatMode};
use crate::audio::types::PlayStatus;
use crate::ui::theme;

pub fn draw(f: &mut Frame, area: Rect, state: &AppState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::surface1()))
        .style(Style::default().bg(theme::mantle()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let (status_icon, status_color) = match state.playback.status {
        PlayStatus::Playing => ("", theme::green()),
        PlayStatus::Paused => ("", theme::yellow()),
        PlayStatus::Buffering => ("", theme::peach()),
        PlayStatus::Stopped => ("", theme::overlay0()),
    };

    let track_info = if let Some(ref track) = state.playback.current_track {
        format!("{} - {}", track.artist, track.title)
    } else {
        String::new()
    };

    // Left: logo + track info
    let left_spans = vec![
        Span::styled(
            format!(" groovebox v{} ", env!("CARGO_PKG_VERSION")),
            Style::default().fg(theme::mantle()).bg(theme::mauve()).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(format!("{status_icon} "), Style::default().fg(status_color)),
        Span::styled(track_info, Style::default().fg(theme::text())),
    ];

    // Right: shuffle/repeat badges + volume
    let mut right_parts: Vec<Span> = Vec::new();
    if state.shuffle {
        right_parts.push(Span::styled(" shfl ", Style::default().fg(theme::mantle()).bg(theme::peach())));
        right_parts.push(Span::raw(" "));
    }
    match state.repeat {
        RepeatMode::Off => {}
        RepeatMode::One => {
            right_parts.push(Span::styled(" rpt1 ", Style::default().fg(theme::mantle()).bg(theme::mauve())));
            right_parts.push(Span::raw(" "));
        }
        RepeatMode::All => {
            right_parts.push(Span::styled(" rpt* ", Style::default().fg(theme::mantle()).bg(theme::blue())));
            right_parts.push(Span::raw(" "));
        }
    }
    let vol_pct = state.playback.volume as u32;
    let vol_color = if vol_pct > 100 { theme::red() } else { theme::subtext0() };
    right_parts.push(Span::styled(format!("vol {vol_pct}% "), Style::default().fg(vol_color)));

    // Calculate right side width for layout split
    let right_width: u16 = right_parts.iter().map(|s| s.width() as u16).sum();

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(right_width)])
        .split(inner);

    let left = Paragraph::new(Line::from(left_spans));
    f.render_widget(left, chunks[0]);

    let right = Paragraph::new(Line::from(right_parts)).alignment(Alignment::Right);
    f.render_widget(right, chunks[1]);
}
