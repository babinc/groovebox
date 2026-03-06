use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::audio::types::PlaybackState;
use crate::ui::theme;

pub fn draw(f: &mut Frame, area: Rect, playback: &PlaybackState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(7),
            Constraint::Min(1),
            Constraint::Length(7),
        ])
        .split(area);

    let position = format_time(playback.position);
    let duration = format_time(playback.duration);

    let time_left = Paragraph::new(Line::from(
        Span::styled(format!(" {position}"), Style::default().fg(theme::SUBTEXT0)),
    ));
    f.render_widget(time_left, chunks[0]);

    // Custom progress bar using Unicode block characters
    let width = chunks[1].width as usize;
    if width > 0 {
        let ratio = if playback.duration > 0.0 {
            (playback.position / playback.duration).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let filled = (ratio * width as f64) as usize;

        let mut spans: Vec<Span> = Vec::new();

        if filled > 0 {
            spans.push(Span::styled(
                "━".repeat(filled),
                Style::default().fg(theme::MAUVE),
            ));
        }

        // Cursor position
        if filled < width {
            spans.push(Span::styled("╸", Style::default().fg(theme::MAUVE)));
            if filled + 1 < width {
                spans.push(Span::styled(
                    "─".repeat(width - filled - 1),
                    Style::default().fg(theme::SURFACE1),
                ));
            }
        }

        let bar = Paragraph::new(Line::from(spans));
        f.render_widget(bar, chunks[1]);
    }

    let time_right = Paragraph::new(Line::from(
        Span::styled(format!("{duration} "), Style::default().fg(theme::SURFACE2)),
    ));
    f.render_widget(time_right, chunks[2]);
}

fn format_time(seconds: f64) -> String {
    let total = seconds as u64;
    let mins = total / 60;
    let secs = total % 60;
    format!("{mins:02}:{secs:02}")
}
