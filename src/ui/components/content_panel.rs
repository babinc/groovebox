use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table},
};

use crate::app::state::{AppState, ContentView, Focus};
use crate::models::Track;
use crate::ui::theme;

pub fn draw(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    thumb_protocol: &mut Option<ratatui_image::protocol::StatefulProtocol>,
) {
    let focused = state.focus == Focus::Queue;
    let border_color = if focused { theme::BLUE } else { theme::SURFACE0 };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme::BASE));

    match &state.content_view {
        ContentView::SearchResults => {
            draw_search_view(f, area, state, block, focused, thumb_protocol);
        }
        ContentView::PlaylistTracks(_id) => {
            draw_track_list(f, area, &state.search_results, state.content_index, block, focused, "playlist", thumb_protocol);
        }
        ContentView::HistoryList => {
            draw_history_view(f, area, state, block);
        }
        _ => {
            let inner = block.inner(area);
            f.render_widget(block, area);
            let msg = Paragraph::new(Line::from(vec![
                Span::styled(" press ", Style::default().fg(theme::OVERLAY0)),
                Span::styled("/", Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)),
                Span::styled(" to search", Style::default().fg(theme::OVERLAY0)),
            ]));
            f.render_widget(msg, inner);
        }
    }
}

fn draw_search_view(
    f: &mut Frame,
    area: Rect,
    state: &AppState,
    block: Block,
    focused: bool,
    thumb_protocol: &mut Option<ratatui_image::protocol::StatefulProtocol>,
) {
    if state.searching {
        let inner = block.inner(area);
        f.render_widget(block, area);
        let msg = Paragraph::new(Line::from(Span::styled(
            " searching...",
            Style::default().fg(theme::YELLOW),
        )));
        f.render_widget(msg, inner);
        return;
    }

    if state.search_results.is_empty() {
        let inner = block.inner(area);
        f.render_widget(block, area);
        let msg = if state.search_query.is_empty() {
            " press / to search"
        } else {
            " no results"
        };
        let paragraph = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(theme::OVERLAY0),
        )));
        f.render_widget(paragraph, inner);
        return;
    }

    draw_track_list(
        f, area, &state.search_results, state.content_index,
        block, focused,
        &state.search_query,
        thumb_protocol,
    );
}

fn draw_track_list(
    f: &mut Frame,
    area: Rect,
    tracks: &[Track],
    selected: usize,
    block: Block,
    focused: bool,
    query: &str,
    thumb_protocol: &mut Option<ratatui_image::protocol::StatefulProtocol>,
) {
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Split: thumbnail preview on top, track list below
    let has_thumb = thumb_protocol.is_some();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if has_thumb {
            vec![Constraint::Length(8), Constraint::Length(1), Constraint::Min(1)]
        } else {
            vec![Constraint::Length(0), Constraint::Length(1), Constraint::Min(1)]
        })
        .split(inner);

    // Thumbnail + info row
    if has_thumb {
        let thumb_row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(16), Constraint::Min(1)])
            .split(chunks[0]);

        if let Some(protocol) = thumb_protocol.as_mut() {
            let image = ratatui_image::StatefulImage::default();
            f.render_stateful_widget(image, thumb_row[0], protocol);
        }

        if selected < tracks.len() {
            let track = &tracks[selected];
            let info = Paragraph::new(vec![
                Line::from(Span::styled(
                    &track.title,
                    Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    &track.artist,
                    Style::default().fg(theme::SUBTEXT0),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("dur ", Style::default().fg(theme::SURFACE2)),
                    Span::styled(track.duration_display(), Style::default().fg(theme::SUBTEXT1)),
                ]),
            ]);
            f.render_widget(info, thumb_row[1]);
        }
    }

    // Query/title bar
    let title_line = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {query} "),
            Style::default().fg(theme::SURFACE0).bg(theme::BLUE),
        ),
        Span::styled(
            format!("  {} tracks", tracks.len()),
            Style::default().fg(theme::OVERLAY0),
        ),
    ]));
    f.render_widget(title_line, chunks[1]);

    // Track list
    let table_area = chunks[2];
    let table_height = table_area.height as usize;
    let scroll_offset = if selected >= table_height {
        selected - table_height + 1
    } else {
        0
    };

    let rows: Vec<Row> = tracks
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(table_height)
        .map(|(i, track)| {
            let is_selected = i == selected;
            let num_style;
            let title_style;
            let artist_style;
            let dur_style;

            if is_selected && focused {
                num_style = Style::default().fg(theme::BLUE).add_modifier(Modifier::BOLD);
                title_style = Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD);
                artist_style = Style::default().fg(theme::SUBTEXT1);
                dur_style = Style::default().fg(theme::SUBTEXT0);
            } else if is_selected {
                num_style = Style::default().fg(theme::OVERLAY1);
                title_style = Style::default().fg(theme::SUBTEXT1);
                artist_style = Style::default().fg(theme::OVERLAY0);
                dur_style = Style::default().fg(theme::SURFACE2);
            } else {
                num_style = Style::default().fg(theme::SURFACE2);
                title_style = Style::default().fg(theme::SUBTEXT0);
                artist_style = Style::default().fg(theme::OVERLAY0);
                dur_style = Style::default().fg(theme::SURFACE2);
            }

            let indicator = if is_selected && focused { ">" } else { " " };

            Row::new(vec![
                Cell::from(format!("{indicator}{:>2}", i + 1)).style(num_style),
                Cell::from(track.title.clone()).style(title_style),
                Cell::from(track.artist.clone()).style(artist_style),
                Cell::from(track.duration_display()).style(dur_style),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Percentage(50),
            Constraint::Percentage(30),
            Constraint::Length(8),
        ],
    );

    f.render_widget(table, table_area);
}

fn draw_history_view(f: &mut Frame, area: Rect, state: &AppState, block: Block) {
    let inner = block.inner(area);
    f.render_widget(block, area);

    if state.history.is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            " no history",
            Style::default().fg(theme::OVERLAY0),
        )));
        f.render_widget(msg, inner);
        return;
    }

    let rows: Vec<Row> = state
        .history
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let is_selected = i == state.content_index && state.focus == Focus::Queue;

            let title = entry.track_title.as_deref().unwrap_or("?");
            let artist = entry.track_artist.as_deref().unwrap_or("?");
            let ago = format_relative_time(entry.played_at);

            if is_selected {
                Row::new(vec![
                    Cell::from(title.to_string()).style(Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD)),
                    Cell::from(artist.to_string()).style(Style::default().fg(theme::SUBTEXT1)),
                    Cell::from(ago).style(Style::default().fg(theme::OVERLAY0)),
                ])
            } else {
                Row::new(vec![
                    Cell::from(title.to_string()).style(Style::default().fg(theme::SUBTEXT0)),
                    Cell::from(artist.to_string()).style(Style::default().fg(theme::OVERLAY0)),
                    Cell::from(ago).style(Style::default().fg(theme::SURFACE2)),
                ])
            }
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(45),
            Constraint::Percentage(30),
            Constraint::Percentage(25),
        ],
    );

    f.render_widget(table, inner);
}

fn format_relative_time(dt: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let diff = now - dt;

    if diff.num_minutes() < 1 {
        "now".to_string()
    } else if diff.num_minutes() < 60 {
        format!("{}m", diff.num_minutes())
    } else if diff.num_hours() < 24 {
        format!("{}h", diff.num_hours())
    } else {
        format!("{}d", diff.num_days())
    }
}
