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
        .border_style(Style::default().fg(if focused { theme::YELLOW } else { theme::SURFACE0 }))
        .style(Style::default().bg(theme::MANTLE));

    let cursor = if focused { "_" } else { "" };
    let text = Line::from(vec![
        Span::styled(" / ", Style::default().fg(theme::MANTLE).bg(theme::YELLOW)),
        Span::raw(" "),
        Span::styled(&state.search_query, Style::default().fg(theme::TEXT)),
        Span::styled(cursor, Style::default().fg(theme::TEXT).add_modifier(Modifier::SLOW_BLINK)),
    ]);

    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}
