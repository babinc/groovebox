use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use crate::app::state::AppState;
use crate::ui::theme;

pub fn draw_playlist_popup(f: &mut Frame, state: &AppState) {
    let area = centered_rect(50, 30, f.area());

    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" new playlist ")
        .title_style(Style::default().fg(theme::MAUVE))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::MAUVE))
        .style(Style::default().bg(theme::MANTLE));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

    let name_line = Paragraph::new(Line::from(vec![
        Span::styled("name ", Style::default().fg(theme::SURFACE2)),
        Span::styled(&state.popup_input, Style::default().fg(theme::TEXT)),
        Span::styled("_", Style::default().fg(theme::TEXT).add_modifier(Modifier::SLOW_BLINK)),
    ]));
    f.render_widget(name_line, chunks[0]);

    let desc_line = Paragraph::new(Line::from(vec![
        Span::styled("desc ", Style::default().fg(theme::SURFACE2)),
        Span::styled(&state.popup_description, Style::default().fg(theme::SUBTEXT0)),
    ]));
    f.render_widget(desc_line, chunks[1]);

    let help = Paragraph::new(Line::from(vec![
        Span::styled("enter", Style::default().fg(theme::GREEN)),
        Span::styled(" create  ", Style::default().fg(theme::OVERLAY0)),
        Span::styled("esc", Style::default().fg(theme::RED)),
        Span::styled(" cancel", Style::default().fg(theme::OVERLAY0)),
    ]));
    f.render_widget(help, chunks[2]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
