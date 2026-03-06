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

    let block = popup_block(" new playlist ");

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
        Span::styled("name ", Style::default().fg(theme::surface2())),
        Span::styled(&state.popup_input, Style::default().fg(theme::text())),
        Span::styled("_", Style::default().fg(theme::text()).add_modifier(Modifier::SLOW_BLINK)),
    ]));
    f.render_widget(name_line, chunks[0]);

    let desc_line = Paragraph::new(Line::from(vec![
        Span::styled("desc ", Style::default().fg(theme::surface2())),
        Span::styled(&state.popup_description, Style::default().fg(theme::subtext0())),
    ]));
    f.render_widget(desc_line, chunks[1]);

    let help = Paragraph::new(Line::from(vec![
        Span::styled("enter", Style::default().fg(theme::green())),
        Span::styled(" create  ", Style::default().fg(theme::overlay0())),
        Span::styled("esc", Style::default().fg(theme::red())),
        Span::styled(" cancel", Style::default().fg(theme::overlay0())),
    ]));
    f.render_widget(help, chunks[2]);
}

pub(crate) fn popup_block(title: &str) -> Block<'_> {
    Block::default()
        .title(title)
        .title_style(Style::default().fg(theme::mauve()).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::mauve()))
        .style(Style::default().bg(theme::mantle()))
}

pub(crate) fn centered_fixed(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.width.saturating_sub(width) / 2;
    let y = r.height.saturating_sub(height) / 2;
    Rect {
        x: r.x + x,
        y: r.y + y,
        width: width.min(r.width),
        height: height.min(r.height),
    }
}

pub(crate) fn draw_selector(f: &mut Frame, title: &str, items: &[&str], selected: usize) {
    let popup_area = centered_fixed(30, items.len() as u16 + 4, f.area());
    f.render_widget(Clear, popup_area);

    let block = popup_block(title);
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let constraints: Vec<Constraint> = (0..items.len()).map(|_| Constraint::Length(1)).collect();
    let rows = Layout::vertical(constraints).split(inner);

    for (i, name) in items.iter().enumerate() {
        let is_selected = i == selected;
        let style = if is_selected {
            Style::default().fg(theme::mantle()).bg(theme::mauve()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::subtext0())
        };
        let marker = if is_selected { " > " } else { "   " };
        let line = Line::from(vec![Span::styled(marker, style), Span::styled(*name, style)]);
        if i < rows.len() {
            f.render_widget(Paragraph::new(line), rows[i]);
        }
    }
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
