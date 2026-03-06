use std::collections::HashMap;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::app::state::{AppState, ContentView, Focus, Preferences};
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
    let border_color = if focused { theme::blue() } else { theme::surface1() };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme::base()));

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
                        Style::default().fg(theme::mantle()).bg(theme::blue()),
                    ),
                    Span::styled(
                        format!(" {} results", state.search_results.len()),
                        Style::default().fg(theme::surface2()),
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
                        Style::default().fg(theme::mantle()).bg(theme::blue()),
                    ),
                    Span::styled(
                        format!(" {} tracks", state.search_results.len()),
                        Style::default().fg(theme::surface2()),
                    ),
                ])
            }
            ContentView::HistoryList => {
                Line::from(Span::styled(
                    " history ",
                    Style::default().fg(theme::mantle()).bg(theme::mauve()),
                ))
            }
            ContentView::Settings => {
                Line::from(Span::styled(
                    " settings ",
                    Style::default().fg(theme::mantle()).bg(theme::teal()),
                ))
            }
            _ => {
                Line::from(Span::styled(
                    " press / to search",
                    Style::default().fg(theme::overlay0()),
                ))
            }
        };
        f.render_widget(Paragraph::new(title), chunks[0]);
        (None, chunks[1])
    };

    if let Some(sa) = search_area {
        super::search_input::draw(f, sa, state);
    }

    // Settings view
    if state.content_view == ContentView::Settings {
        draw_settings(f, list_area, state, focused);
        return;
    }

    let tracks = &state.search_results;
    if tracks.is_empty() {
        if state.searching {
            let msg = Paragraph::new(Line::from(Span::styled(
                " searching...",
                Style::default().fg(theme::yellow()),
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
        accent_color = theme::green();
    } else if is_selected && focused {
        accent_char = "▌";
        accent_color = theme::blue();
    } else {
        accent_char = " ";
        accent_color = theme::base();
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
            Line::from(Span::styled("  ┌──────┐", Style::default().fg(theme::surface1()))),
            Line::from(Span::styled("  │      │", Style::default().fg(theme::surface1()))),
            Line::from(Span::styled("  └──────┘", Style::default().fg(theme::surface1()))),
        ]);
        f.render_widget(placeholder, chunks[0]);
    }

    // Text info
    let text_area = chunks[2];
    let max_title_len = text_area.width.saturating_sub(5) as usize;

    let title_display = if track.title.chars().count() > max_title_len && max_title_len > 3 {
        let truncated: String = track.title.chars().take(max_title_len.saturating_sub(3)).collect();
        format!("{truncated}...")
    } else {
        track.title.clone()
    };

    let num_style = if is_playing {
        Style::default().fg(theme::green()).add_modifier(Modifier::BOLD)
    } else if is_selected && focused {
        Style::default().fg(theme::blue()).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::surface2())
    };

    let title_style = if is_playing {
        Style::default().fg(theme::green()).add_modifier(Modifier::BOLD)
    } else if is_selected && focused {
        Style::default().fg(theme::text()).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::subtext0())
    };

    let artist_style = if is_selected && focused {
        Style::default().fg(theme::subtext1())
    } else {
        Style::default().fg(theme::overlay0())
    };

    let playing_indicator = if is_playing { " " } else { "" };

    let lines = vec![
        Line::from(vec![
            Span::styled(format!("{:>2} ", index + 1), num_style),
            Span::styled(playing_indicator, Style::default().fg(theme::green())),
            Span::styled(title_display, title_style),
        ]),
        Line::from(vec![
            Span::styled("   ", Style::default()),
            Span::styled(&track.artist, artist_style),
        ]),
        Line::from(vec![
            Span::styled("   ", Style::default()),
            Span::styled(track.duration_display(), Style::default().fg(theme::surface2())),
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
            Style::default().fg(theme::surface0()),
        )));
        f.render_widget(sep, sep_area);
    }
}

fn draw_settings(f: &mut Frame, area: Rect, state: &AppState, focused: bool) {
    let settings = Preferences::KEYS;

    for (i, &(key, label)) in settings.iter().enumerate() {
        if i as u16 * 2 >= area.height { break; }

        let row = Rect {
            x: area.x,
            y: area.y + i as u16 * 2,
            width: area.width,
            height: 2,
        };

        let is_selected = i == state.settings_index && focused;
        let enabled = state.preferences.get(key);

        let toggle = if enabled {
            Span::styled(" ON  ", Style::default().fg(theme::mantle()).bg(theme::green()))
        } else {
            Span::styled(" OFF ", Style::default().fg(theme::mantle()).bg(theme::surface2()))
        };

        let label_style = if is_selected {
            Style::default().fg(theme::text()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::subtext0())
        };

        let marker = if is_selected { "▌ " } else { "  " };
        let marker_style = if is_selected {
            Style::default().fg(theme::blue())
        } else {
            Style::default()
        };

        let line = Line::from(vec![
            Span::styled(marker, marker_style),
            toggle,
            Span::raw("  "),
            Span::styled(label, label_style),
        ]);

        f.render_widget(Paragraph::new(line), row);
    }

    // Help text at bottom
    if area.height > (settings.len() as u16 * 2) + 2 {
        let help_y = area.y + settings.len() as u16 * 2 + 1;
        let help = Paragraph::new(Line::from(vec![
            Span::styled(" Enter ", Style::default().fg(theme::overlay1()).bg(theme::surface0())),
            Span::styled(" toggle", Style::default().fg(theme::overlay0())),
        ]));
        f.render_widget(help, Rect { x: area.x + 1, y: help_y, width: area.width.saturating_sub(1), height: 1 });
    }
}
