use std::collections::HashMap;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::app::state::{AppState, ContentView, Focus};
use crate::models::Track;
use crate::ui::theme;

const CARD_HEIGHT: u16 = 5;

pub fn draw(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    thumb_cache: &mut HashMap<String, ratatui_image::protocol::StatefulProtocol>,
) {
    let focused = state.focus == Focus::Queue;
    let border_color = if focused { theme::BLUE } else { theme::SURFACE0 };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme::BASE));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Search input or title bar
    let (search_area, list_area) = if state.focus == Focus::SearchInput {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(inner);
        (Some(chunks[0]), chunks[1])
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(inner);

        let title = match &state.content_view {
            ContentView::SearchResults if !state.search_query.is_empty() => {
                Line::from(vec![
                    Span::styled(
                        format!(" {} ", state.search_query),
                        Style::default().fg(theme::MANTLE).bg(theme::BLUE),
                    ),
                    Span::styled(
                        format!(" {} results", state.search_results.len()),
                        Style::default().fg(theme::SURFACE2),
                    ),
                ])
            }
            ContentView::PlaylistTracks(id) => {
                let name = state.playlists.iter()
                    .find(|p| p.id == Some(*id))
                    .map(|p| p.name.as_str())
                    .unwrap_or("playlist");
                Line::from(vec![
                    Span::styled(
                        format!(" {name} "),
                        Style::default().fg(theme::MANTLE).bg(theme::BLUE),
                    ),
                    Span::styled(
                        format!(" {} tracks", state.search_results.len()),
                        Style::default().fg(theme::SURFACE2),
                    ),
                ])
            }
            ContentView::HistoryList => {
                Line::from(Span::styled(
                    " history ",
                    Style::default().fg(theme::MANTLE).bg(theme::MAUVE),
                ))
            }
            _ => {
                Line::from(Span::styled(
                    " press / to search",
                    Style::default().fg(theme::OVERLAY0),
                ))
            }
        };
        f.render_widget(Paragraph::new(title), chunks[0]);
        (None, chunks[1])
    };

    if let Some(sa) = search_area {
        super::search_input::draw(f, sa, state);
    }

    let tracks = &state.search_results;
    if tracks.is_empty() {
        if state.searching {
            let msg = Paragraph::new(Line::from(Span::styled(
                " searching...",
                Style::default().fg(theme::YELLOW),
            )));
            f.render_widget(msg, list_area);
        }
        return;
    }

    let visible_cards = (list_area.height / CARD_HEIGHT) as usize;
    let scroll = if state.content_index >= visible_cards {
        state.content_index - visible_cards + 1
    } else {
        0
    };

    for (vi, track_idx) in (scroll..tracks.len()).enumerate() {
        if vi >= visible_cards {
            break;
        }

        let track = &tracks[track_idx];
        let y = list_area.y + (vi as u16 * CARD_HEIGHT);

        if y + CARD_HEIGHT > list_area.y + list_area.height {
            break;
        }

        let card_area = Rect {
            x: list_area.x,
            y,
            width: list_area.width,
            height: CARD_HEIGHT,
        };

        draw_track_card(f, card_area, track, track_idx, state, focused, thumb_cache);
    }
}

fn draw_track_card(
    f: &mut Frame,
    area: Rect,
    track: &Track,
    index: usize,
    state: &AppState,
    focused: bool,
    thumb_cache: &mut HashMap<String, ratatui_image::protocol::StatefulProtocol>,
) {
    let is_selected = index == state.content_index;
    let is_playing = state.queue_index == Some(index)
        && state.playback.current_track.as_ref().map(|t| &t.youtube_id) == Some(&track.youtube_id);

    // Left accent bar for playing/selected state
    let accent_char;
    let accent_color;
    if is_playing {
        accent_char = "▌";
        accent_color = theme::GREEN;
    } else if is_selected && focused {
        accent_char = "▌";
        accent_color = theme::BLUE;
    } else {
        accent_char = " ";
        accent_color = theme::BASE;
    }

    // Draw accent bar on first column
    for row in 0..area.height.saturating_sub(1) {
        let accent_area = Rect {
            x: area.x,
            y: area.y + row,
            width: 1,
            height: 1,
        };
        let accent = Paragraph::new(Span::styled(accent_char, Style::default().fg(accent_color)));
        f.render_widget(accent, accent_area);
    }

    // Card layout: [accent 1] [thumbnail 10] [gap 1] [text rest]
    let content_area = Rect {
        x: area.x + 1,
        y: area.y,
        width: area.width.saturating_sub(1),
        height: area.height.saturating_sub(1), // Leave 1 row for separator
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(10),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(content_area);

    // Thumbnail
    if let Some(protocol) = thumb_cache.get_mut(&track.youtube_id) {
        let image = ratatui_image::StatefulImage::default();
        f.render_stateful_widget(image, chunks[0], protocol);
    } else {
        let placeholder = Paragraph::new(vec![
            Line::from(Span::styled("  ┌──────┐", Style::default().fg(theme::SURFACE1))),
            Line::from(Span::styled("  │      │", Style::default().fg(theme::SURFACE1))),
            Line::from(Span::styled("  └──────┘", Style::default().fg(theme::SURFACE1))),
        ]);
        f.render_widget(placeholder, chunks[0]);
    }

    // Text info
    let text_area = chunks[2];
    let max_title_len = text_area.width.saturating_sub(5) as usize;

    let title_display = if track.title.len() > max_title_len && max_title_len > 3 {
        format!("{}...", &track.title[..max_title_len.saturating_sub(3)])
    } else {
        track.title.clone()
    };

    let num_style = if is_playing {
        Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)
    } else if is_selected && focused {
        Style::default().fg(theme::BLUE).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::SURFACE2)
    };

    let title_style = if is_playing {
        Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)
    } else if is_selected && focused {
        Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::SUBTEXT0)
    };

    let artist_style = if is_selected && focused {
        Style::default().fg(theme::SUBTEXT1)
    } else {
        Style::default().fg(theme::OVERLAY0)
    };

    let playing_indicator = if is_playing { " " } else { "" };

    let lines = vec![
        Line::from(vec![
            Span::styled(format!("{:>2} ", index + 1), num_style),
            Span::styled(playing_indicator, Style::default().fg(theme::GREEN)),
            Span::styled(title_display, title_style),
        ]),
        Line::from(vec![
            Span::styled("   ", Style::default()),
            Span::styled(&track.artist, artist_style),
        ]),
        Line::from(vec![
            Span::styled("   ", Style::default()),
            Span::styled(track.duration_display(), Style::default().fg(theme::SURFACE2)),
        ]),
    ];

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, text_area);

    // Separator line at bottom of card
    let sep_y = area.y + area.height - 1;
    if sep_y < area.y + area.height {
        let sep_area = Rect {
            x: area.x + 1,
            y: sep_y,
            width: area.width.saturating_sub(2),
            height: 1,
        };
        let sep = Paragraph::new(Line::from(Span::styled(
            "─".repeat(sep_area.width as usize),
            Style::default().fg(theme::SURFACE0),
        )));
        f.render_widget(sep, sep_area);
    }
}
