use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::app::state::{AppState, RepeatMode};
use crate::audio::types::PlayStatus;
use crate::ui::theme;

pub fn draw(f: &mut Frame, area: Rect, state: &AppState) {
    let (status_icon, status_color) = match state.playback.status {
        PlayStatus::Playing => ("", theme::GREEN),
        PlayStatus::Paused => ("", theme::YELLOW),
        PlayStatus::Buffering => ("", theme::PEACH),
        PlayStatus::Stopped => ("", theme::OVERLAY0),
    };

    let track_info = if let Some(ref track) = state.playback.current_track {
        format!("{} - {}", track.artist, track.title)
    } else {
        String::new()
    };

    let mut right_parts: Vec<Span> = Vec::new();

    if state.shuffle {
        right_parts.push(Span::styled(" shfl ", Style::default().fg(theme::MANTLE).bg(theme::PEACH)));
        right_parts.push(Span::raw(" "));
    }
    match state.repeat {
        RepeatMode::Off => {}
        RepeatMode::One => {
            right_parts.push(Span::styled(" rpt1 ", Style::default().fg(theme::MANTLE).bg(theme::MAUVE)));
            right_parts.push(Span::raw(" "));
        }
        RepeatMode::All => {
            right_parts.push(Span::styled(" rpt* ", Style::default().fg(theme::MANTLE).bg(theme::BLUE)));
            right_parts.push(Span::raw(" "));
        }
    }

    let vol_pct = state.playback.volume as u32;
    let vol_color = if vol_pct > 100 { theme::RED } else { theme::SUBTEXT0 };
    right_parts.push(Span::styled(format!("vol {vol_pct}%"), Style::default().fg(vol_color)));
    right_parts.push(Span::raw("  "));

    // Keybinding hints
    let hint_style = Style::default().fg(theme::SURFACE2);
    let key_style = Style::default().fg(theme::OVERLAY1);
    right_parts.extend([
        Span::styled("/", key_style), Span::styled("search ", hint_style),
        Span::styled("⏎", key_style), Span::styled("play ", hint_style),
        Span::styled("␣", key_style), Span::styled("pause ", hint_style),
        Span::styled("±", key_style), Span::styled("vol ", hint_style),
    ]);

    let mut spans = vec![
        Span::styled(" groovebox ", Style::default().fg(theme::MANTLE).bg(theme::MAUVE).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::styled(format!("{status_icon} "), Style::default().fg(status_color)),
        Span::styled(track_info, Style::default().fg(theme::TEXT)),
        Span::raw("  "),
    ];
    spans.extend(right_parts);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::SURFACE0))
        .style(Style::default().bg(theme::MANTLE));

    let paragraph = Paragraph::new(Line::from(spans)).block(block);
    f.render_widget(paragraph, area);
}
