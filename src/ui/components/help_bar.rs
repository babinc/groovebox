use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::ui::theme;

pub fn draw(f: &mut Frame, area: Rect) {
    let key_style = Style::default().fg(theme::overlay1()).bg(theme::surface0());
    let hint_style = Style::default().fg(theme::overlay0());
    let sep = Span::raw("  ");

    let line = Line::from(vec![
        Span::styled(" / ", key_style), Span::styled(" search", hint_style), sep.clone(),
        Span::styled(" Enter ", key_style), Span::styled(" play", hint_style), sep.clone(),
        Span::styled(" Space ", key_style), Span::styled(" pause", hint_style), sep.clone(),
        Span::styled(" ←/→ ", key_style), Span::styled(" seek", hint_style), sep.clone(),
        Span::styled(" n ", key_style), Span::styled("/", hint_style), Span::styled(" p ", key_style), Span::styled(" next/prev", hint_style), sep.clone(),
        Span::styled(" s ", key_style), Span::styled(" shuffle", hint_style), sep.clone(),
        Span::styled(" r ", key_style), Span::styled(" repeat", hint_style), sep.clone(),
        Span::styled(" -/+ ", key_style), Span::styled(" vol", hint_style), sep.clone(),
        Span::styled(" a ", key_style), Span::styled(" playlist", hint_style), sep.clone(),
        Span::styled(" e ", key_style), Span::styled(" eq", hint_style), sep.clone(),
        Span::styled(" t ", key_style), Span::styled(" theme", hint_style), sep.clone(),
        Span::styled(" q ", key_style), Span::styled(" quit", hint_style),
    ]);

    let bar = Paragraph::new(line).style(Style::default().bg(theme::mantle()));
    f.render_widget(bar, area);
}
