use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::app::state::{AppState, Focus};
use crate::ui::theme;

pub fn draw(f: &mut Frame, area: Rect, state: &AppState) {
    let focused = state.focus == Focus::SearchInput;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if focused { theme::yellow() } else { theme::surface1() }))
        .style(Style::default().bg(theme::mantle()));

    let cursor = if focused { "_" } else { "" };
    let text = Line::from(vec![
        Span::styled(" / ", Style::default().fg(theme::mantle()).bg(theme::yellow())),
        Span::raw(" "),
        Span::styled(&state.search_query, Style::default().fg(theme::text())),
        Span::styled(cursor, Style::default().fg(theme::text()).add_modifier(Modifier::SLOW_BLINK)),
    ]);

    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}
