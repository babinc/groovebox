use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::audio::types::{PlayStatus, PlaybackState};
use crate::ui::theme;

pub fn draw(f: &mut Frame, area: Rect, playback: &PlaybackState, frame_count: usize) {
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
        Span::styled(format!(" {position}"), Style::default().fg(theme::subtext0())),
    ));
    f.render_widget(time_left, chunks[0]);

    // Custom progress bar using Unicode block characters
    let width = chunks[1].width as usize;
    if width > 0 {
        if playback.status == PlayStatus::Buffering {
            draw_buffering(f, chunks[1], width, frame_count);
        } else {
            draw_progress(f, chunks[1], width, playback);
        }
    }

    let time_right = Paragraph::new(Line::from(
        Span::styled(format!("{duration} "), Style::default().fg(theme::surface2())),
    ));
    f.render_widget(time_right, chunks[2]);
}

fn draw_progress(f: &mut Frame, area: Rect, width: usize, playback: &PlaybackState) {
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
            Style::default().fg(theme::mauve()),
        ));
    }

    if filled < width {
        spans.push(Span::styled("╸", Style::default().fg(theme::mauve())));
        if filled + 1 < width {
            spans.push(Span::styled(
                "─".repeat(width - filled - 1),
                Style::default().fg(theme::surface1()),
            ));
        }
    }

    let bar = Paragraph::new(Line::from(spans));
    f.render_widget(bar, area);
}

fn draw_buffering(f: &mut Frame, area: Rect, width: usize, frame_count: usize) {
    // Bouncing highlight animation
    let cycle = width * 2;
    let tick = frame_count % cycle;
    let pos = if tick < width { tick } else { cycle - tick };
    let highlight_len = 6.min(width);
    let start = pos.saturating_sub(highlight_len / 2);
    let end = (start + highlight_len).min(width);

    let mut spans: Vec<Span> = Vec::new();

    if start > 0 {
        spans.push(Span::styled(
            "─".repeat(start),
            Style::default().fg(theme::surface1()),
        ));
    }
    spans.push(Span::styled(
        "━".repeat(end - start),
        Style::default().fg(theme::peach()),
    ));
    if end < width {
        spans.push(Span::styled(
            "─".repeat(width - end),
            Style::default().fg(theme::surface1()),
        ));
    }

    let bar = Paragraph::new(Line::from(spans));
    f.render_widget(bar, area);
}

pub fn format_time(seconds: f64) -> String {
    let total = seconds as u64;
    let hours = total / 3600;
    let mins = (total % 3600) / 60;
    let secs = total % 60;
    if hours > 0 {
        format!("{hours}:{mins:02}:{secs:02}")
    } else {
        format!("{mins:02}:{secs:02}")
    }
}
