use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem},
};

use crate::app::state::{AppState, Focus, NavSection};
use crate::ui::theme;

pub fn draw(f: &mut Frame, area: Rect, state: &AppState) {
    let focused = state.focus == Focus::Navigation;
    let border_color = if focused { theme::MAUVE } else { theme::SURFACE0 };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme::BASE));

    let sections = [
        (NavSection::Search, "search", theme::YELLOW),
        (NavSection::Playlists, "playlists", theme::BLUE),
        (NavSection::Categories, "categories", theme::GREEN),
        (NavSection::History, "history", theme::MAUVE),
    ];

    let mut items: Vec<ListItem> = Vec::new();

    for (section, label, accent) in &sections {
        let is_selected = state.nav_section == *section && focused;
        let is_active = state.nav_section == *section;

        let line = if is_selected {
            Line::from(vec![
                Span::styled(" ", Style::default().fg(*accent)),
                Span::styled(
                    format!(" {label} "),
                    Style::default().fg(theme::MANTLE).bg(*accent).add_modifier(Modifier::BOLD),
                ),
            ])
        } else if is_active {
            Line::from(vec![
                Span::styled(" ", Style::default().fg(*accent)),
                Span::styled(format!(" {label}"), Style::default().fg(*accent)),
            ])
        } else {
            Line::from(Span::styled(
                format!("   {label}"),
                Style::default().fg(theme::OVERLAY1),
            ))
        };

        items.push(ListItem::new(line));
    }

    // Separator
    items.push(ListItem::new(Line::from(Span::styled(
        " ────────────",
        Style::default().fg(theme::SURFACE1),
    ))));

    // Playlist items
    if state.nav_section == NavSection::Playlists || !state.playlists.is_empty() {
        for (i, playlist) in state.playlists.iter().enumerate() {
            let pl_selected = state.nav_section == NavSection::Playlists
                && focused
                && state.nav_sub_index == i;

            let line = if pl_selected {
                Line::from(vec![
                    Span::styled(" ", Style::default().fg(theme::BLUE)),
                    Span::styled(
                        format!(" {} ", playlist.name),
                        Style::default().fg(theme::MANTLE).bg(theme::BLUE),
                    ),
                ])
            } else {
                Line::from(Span::styled(
                    format!("   {}", playlist.name),
                    Style::default().fg(theme::SUBTEXT0),
                ))
            };

            items.push(ListItem::new(line));
        }
    }

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}
