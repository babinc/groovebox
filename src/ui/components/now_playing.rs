use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

use crate::app::state::{AppState, ContentView, Focus};
use crate::audio::types::PlayStatus;
use crate::ui::components::progress_bar;
use crate::ui::theme;
use crate::youtube::chapters;

pub fn draw(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    thumb_image: Option<&mut ratatui_image::protocol::StatefulProtocol>,
) {
    let focused = state.focus == Focus::Center;
    let border_color = if focused { theme::mauve() } else { theme::surface1() };

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
    let h = inner.height;

    // Minimal mode: just title + artist
    if h < 10 {
        let info = build_track_info(state, track);
        let info_area = Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(3) / 2,
            width: inner.width,
            height: 3.min(inner.height),
        };
        f.render_widget(info, info_area);
        return;
    }

    // Determine which sections to show
    let has_thumb = thumb_image.is_some();
    let show_elapsed = h >= 16;
    let show_facts = h >= 14;
    let has_queue = state.queue.len() > 1;
    let show_queue_ctx = h >= 20 && has_queue;
    let has_desc = track.description.as_ref().map_or(false, |d| !d.is_empty());
    let show_desc = h >= 24 && has_desc;

    // Use cached chapters (parsed once when track changes, not every frame)
    let has_chapter = !state.cached_chapters.is_empty() && show_elapsed;

    // Calculate fixed section heights, give remainder to thumbnail
    let mut fixed_height: u16 = 1 + 3; // spacer + info (always present)
    if show_elapsed { fixed_height += 2; }
    if has_chapter { fixed_height += 1; }
    if show_facts { fixed_height += 1 + 5; }
    if show_queue_ctx { fixed_height += 1 + 2; }
    if show_desc { fixed_height += 1 + 8; }

    let thumb_height = if has_thumb {
        // Use remaining space but cap at 20 rows to avoid oversized render areas
        h.saturating_sub(fixed_height).min(20).max(5)
    } else {
        0
    };

    // Build layout
    let mut constraints = Vec::new();
    let mut section_ids: Vec<&str> = Vec::new();

    // Thumbnail — sized to fill remaining space
    if thumb_height > 0 {
        constraints.push(Constraint::Length(thumb_height));
        section_ids.push("thumb");
    }

    // Track info (no extra spacer — 1 blank line is enough)
    constraints.push(Constraint::Length(1));
    section_ids.push("spacer1");
    constraints.push(Constraint::Length(3));
    section_ids.push("info");

    // Elapsed time
    if show_elapsed {
        constraints.push(Constraint::Length(2));
        section_ids.push("elapsed");
    }

    // Current chapter (from description timestamps)
    if has_chapter {
        constraints.push(Constraint::Length(1));
        section_ids.push("chapter");
    }

    // Separator + nerd facts
    if show_facts {
        constraints.push(Constraint::Length(1));
        section_ids.push("sep1");
        constraints.push(Constraint::Length(5));
        section_ids.push("facts");
    }

    // Separator + queue context
    if show_queue_ctx {
        constraints.push(Constraint::Length(1));
        section_ids.push("sep2");
        constraints.push(Constraint::Length(2));
        section_ids.push("queue_ctx");
    }

    // Separator + description
    if show_desc {
        constraints.push(Constraint::Length(1));
        section_ids.push("sep3");
        constraints.push(Constraint::Length(8));
        section_ids.push("desc");
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let idx = |name: &str| -> Option<usize> {
        section_ids.iter().position(|&s| s == name)
    };

    // Thumbnail — centered, capped for pixel protocol compatibility
    if let (Some(i), Some(protocol)) = (idx("thumb"), thumb_image) {
        let chunk = chunks[i];
        let max_h = chunk.height.min(16);
        let img_width = ((max_h as u32) * 7 / 2).min(inner.width as u32).min(56).max(10) as u16;
        let x_offset = inner.width.saturating_sub(img_width) / 2;
        let y_offset = chunk.height.saturating_sub(max_h) / 2;
        let thumb_area = Rect {
            x: chunk.x + x_offset,
            y: chunk.y + y_offset,
            width: img_width,
            height: max_h,
        };
        let image = ratatui_image::StatefulImage::default();
        f.render_stateful_widget(image, thumb_area, protocol);
    }

    // Track info
    if let Some(i) = idx("info") {
        let info = build_track_info(state, track);
        f.render_widget(info, chunks[i]);
    }

    // Elapsed time
    if let Some(i) = idx("elapsed") {
        draw_elapsed_time(f, chunks[i], state);
    }

    // Current chapter
    if let Some(i) = idx("chapter") {
        if let Some(ch) = chapters::current_chapter(&state.cached_chapters, state.playback.position) {
            draw_chapter(f, chunks[i], ch);
        }
    }

    // Separator + nerd facts
    if let Some(i) = idx("sep1") {
        draw_separator(f, chunks[i], inner.width);
    }
    if let Some(i) = idx("facts") {
        draw_nerd_facts(f, chunks[i], state, inner.width);
    }

    // Separator + queue context
    if let Some(i) = idx("sep2") {
        draw_separator(f, chunks[i], inner.width);
    }
    if let Some(i) = idx("queue_ctx") {
        draw_queue_context(f, chunks[i], state);
    }

    // Separator + description
    if let Some(i) = idx("sep3") {
        draw_separator(f, chunks[i], inner.width);
    }
    if let Some(i) = idx("desc") {
        draw_description(f, chunks[i], track, state.frame_count);
    }
}

fn build_track_info<'a>(state: &'a AppState, track: &'a crate::models::Track) -> Paragraph<'a> {
    let status = match state.playback.status {
        PlayStatus::Playing => " playing ",
        PlayStatus::Paused => " paused ",
        _ => "",
    };

    let status_color = match state.playback.status {
        PlayStatus::Playing => theme::green(),
        PlayStatus::Paused => theme::yellow(),
        _ => theme::surface2(),
    };

    let mut info_lines = vec![
        Line::from(Span::styled(
            track.title.as_str(),
            Style::default().fg(theme::text()).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            track.artist.as_str(),
            Style::default().fg(theme::subtext0()),
        )),
    ];

    if !status.is_empty() {
        info_lines.push(Line::from(Span::styled(
            status,
            Style::default().fg(theme::mantle()).bg(status_color),
        )));
    }

    Paragraph::new(info_lines).alignment(Alignment::Center)
}

fn draw_separator(f: &mut Frame, area: Rect, width: u16) {
    let sep = Paragraph::new(Line::from(Span::styled(
        "─".repeat(width as usize),
        Style::default().fg(theme::surface1()),
    )));
    f.render_widget(sep, area);
}

fn draw_elapsed_time(f: &mut Frame, area: Rect, state: &AppState) {
    let pb = &state.playback;
    let pos = progress_bar::format_time(pb.position);
    let dur = progress_bar::format_time(pb.duration);

    let line = Line::from(vec![
        Span::styled(
            pos,
            Style::default().fg(theme::text()).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" / ", Style::default().fg(theme::overlay0())),
        Span::styled(dur, Style::default().fg(theme::subtext0())),
    ]);

    let time_area = Rect {
        x: area.x,
        y: area.y + 1.min(area.height.saturating_sub(1)),
        width: area.width,
        height: 1.min(area.height),
    };
    let p = Paragraph::new(line).alignment(Alignment::Center);
    f.render_widget(p, time_area);
}

fn draw_chapter(f: &mut Frame, area: Rect, chapter: &chapters::Chapter) {
    let line = Line::from(vec![
        Span::styled("♫ ", Style::default().fg(theme::green())),
        Span::styled(
            chapter.title.as_str(),
            Style::default().fg(theme::text()).add_modifier(Modifier::BOLD),
        ),
    ]);
    let p = Paragraph::new(line).alignment(Alignment::Center);
    f.render_widget(p, area);
}

fn draw_nerd_facts(f: &mut Frame, area: Rect, state: &AppState, panel_width: u16) {
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

    if facts.is_empty() {
        return;
    }

    // Render with fixed-width columns, manually centered
    let col_width = 22u16; // "  channels  stereo  " fits in ~22
    let left_pad = panel_width.saturating_sub(col_width) / 2;
    let pad = " ".repeat(left_pad as usize);

    let lines: Vec<Line<'static>> = facts.into_iter().map(|(label, value)| {
        Line::from(vec![
            Span::raw(pad.clone()),
            Span::styled(format!("{label:<10}"), Style::default().fg(theme::overlay1())),
            Span::styled(value, Style::default().fg(theme::text())),
        ])
    }).collect();

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);
}

fn draw_queue_context(f: &mut Frame, area: Rect, state: &AppState) {
    let queue = &state.queue;
    let idx = state.queue_index;

    if queue.is_empty() {
        return;
    }

    let current_idx = idx.unwrap_or(0);
    let max_title_width = area.width.saturating_sub(12) as usize;

    let is_playlist = matches!(state.content_view, ContentView::PlaylistTracks(_));
    let position_text = if is_playlist {
        format!("track {} of {}", current_idx + 1, queue.len())
    } else {
        format!("{} of {}", current_idx + 1, queue.len())
    };

    let mut lines = Vec::new();

    lines.push(Line::from(Span::styled(
        position_text,
        Style::default().fg(theme::subtext0()),
    )));

    if current_idx + 1 < queue.len() {
        let next_title = truncate_str(&queue[current_idx + 1].title, max_title_width);
        lines.push(Line::from(vec![
            Span::styled("up next  ", Style::default().fg(theme::overlay0())),
            Span::styled(next_title, Style::default().fg(theme::overlay1())),
        ]));
    }

    let p = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(p, area);
}

fn draw_description(f: &mut Frame, area: Rect, track: &crate::models::Track, frame_count: usize) {
    let desc = match &track.description {
        Some(d) if !d.is_empty() => d.as_str(),
        _ => return,
    };

    // Add horizontal padding to match centered content above
    let pad = 2u16;
    let padded = Rect {
        x: area.x + pad,
        y: area.y,
        width: area.width.saturating_sub(pad * 2),
        height: area.height,
    };

    // Calculate total wrapped lines to know when to loop
    let wrap_width = padded.width as usize;
    let total_lines = if wrap_width > 0 {
        desc.lines()
            .map(|line| ((line.len() as f64 / wrap_width as f64).ceil() as u16).max(1))
            .sum::<u16>()
    } else {
        1
    };

    // Scroll down slowly, pause at bottom, then loop back to top
    let max_scroll = total_lines.saturating_sub(padded.height);
    let scroll_offset = if max_scroll > 0 {
        // Add extra "pause" frames at top and bottom
        let pause = padded.height as u16; // pause for ~1 screen worth of frames
        let cycle = max_scroll + pause;
        let pos = ((frame_count / 90) as u16) % cycle;
        pos.min(max_scroll)
    } else {
        0
    };

    let p = Paragraph::new(desc)
        .style(Style::default().fg(theme::overlay1()))
        .wrap(Wrap { trim: true })
        .scroll((scroll_offset, 0));

    f.render_widget(p, padded);
}

pub fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else if max_len > 3 {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{truncated}...")
    } else {
        s.chars().take(max_len).collect()
    }
}
