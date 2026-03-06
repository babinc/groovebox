use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::app::state::{AppState, Focus};
use crate::audio::types::PlayStatus;
use crate::ui::theme;

pub fn draw(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    thumb_image: Option<&mut ratatui_image::protocol::StatefulProtocol>,
) {
    let focused = state.focus == Focus::Center;
    let border_color = if focused { theme::green() } else { theme::surface1() };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme::base()));

    let display_track = state.playback.current_track.as_ref()
        .or(state.preview_track.as_ref());

    if display_track.is_none() {
        let inner = block.inner(area);
        f.render_widget(block, area);
        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "groovebox",
                Style::default().fg(theme::surface1()).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "press / to search  |  tab to navigate",
                Style::default().fg(theme::surface2()),
            )),
        ])
        .alignment(Alignment::Center);
        f.render_widget(msg, inner);
        return;
    }

    let inner = block.inner(area);
    f.render_widget(block, area);

    let track = display_track.unwrap();
    let is_playing = matches!(state.playback.status, PlayStatus::Playing | PlayStatus::Paused);

    // Layout: thumbnail (large, centered) | track info | nerd facts
    // Adapt thumbnail height to available space
    let has_thumb = thumb_image.is_some();
    let thumb_height = if !has_thumb {
        0
    } else if inner.height >= 22 {
        12
    } else if inner.height >= 16 {
        8
    } else if inner.height >= 12 {
        5
    } else {
        0
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(thumb_height),  // big thumbnail
            Constraint::Length(if thumb_height > 0 { 1 } else { 0 }),   // spacer
            Constraint::Length(3),   // track info
            Constraint::Length(1),   // separator
            Constraint::Min(1),      // nerd facts
        ])
        .split(inner);

    // Thumbnail — centered and large
    if thumb_height > 0 {
        if let Some(protocol) = thumb_image {
            let img_width = inner.width.min(40);
            let x_offset = (inner.width.saturating_sub(img_width)) / 2;
            let thumb_area = Rect {
                x: chunks[0].x + x_offset,
                y: chunks[0].y,
                width: img_width,
                height: chunks[0].height,
            };
            let image = ratatui_image::StatefulImage::default();
            f.render_stateful_widget(image, thumb_area, protocol);
        }
    }

    // Track info — centered
    let status = if is_playing {
        match state.playback.status {
            PlayStatus::Playing => " playing ",
            PlayStatus::Paused => " paused ",
            _ => "",
        }
    } else {
        ""
    };

    let status_color = match state.playback.status {
        PlayStatus::Playing => theme::green(),
        PlayStatus::Paused => theme::yellow(),
        _ => theme::surface2(),
    };

    let mut info_lines = vec![
        Line::from(Span::styled(
            &track.title,
            Style::default().fg(theme::text()).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            &track.artist,
            Style::default().fg(theme::subtext0()),
        )),
    ];

    if !status.is_empty() {
        info_lines.push(Line::from(Span::styled(
            status,
            Style::default().fg(theme::mantle()).bg(status_color),
        )));
    }

    let info = Paragraph::new(info_lines).alignment(Alignment::Center);
    f.render_widget(info, chunks[2]);

    // Separator
    let sep = Paragraph::new(Line::from(Span::styled(
        "─".repeat(inner.width as usize),
        Style::default().fg(theme::surface1()),
    )));
    f.render_widget(sep, chunks[3]);

    // Nerd facts
    draw_nerd_facts(f, chunks[4], state);
}

fn draw_nerd_facts(f: &mut Frame, area: Rect, state: &AppState) {
    let pb = &state.playback;

    let facts: Vec<(&str, String)> = [
        pb.codec.as_ref().map(|c| ("codec", c.clone())),
        pb.bitrate.map(|b| ("bitrate", format!("{b} kbps"))),
        pb.sample_rate.map(|s| ("sample", format!("{:.1} kHz", s as f64 / 1000.0))),
        pb.channels.map(|c| ("channels", match c { 1 => "mono".into(), 2 => "stereo".into(), n => format!("{n}ch") })),
        pb.current_track.as_ref().and_then(|t| t.filesize.map(|fs| ("size", format!("{:.1} MB", fs as f64 / (1024.0 * 1024.0))))),
    ]
    .into_iter()
    .flatten()
    .collect();

    let mut lines: Vec<Line<'static>> = Vec::new();

    if facts.is_empty() {
        return;
    }

    // Render as a compact centered block
    for (label, value) in facts {
        lines.push(Line::from(vec![
            Span::styled(format!("  {label:<9} "), Style::default().fg(theme::surface2())),
            Span::styled(value, Style::default().fg(theme::subtext0())),
        ]));
    }

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(paragraph, area);
}
