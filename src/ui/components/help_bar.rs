use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::app::state::{AppState, RepeatMode};
use crate::audio::types::PlayStatus;
use crate::ui::theme;

pub fn draw(f: &mut Frame, area: Rect, state: &AppState) {
    let key_style = Style::default().fg(theme::overlay1()).bg(theme::surface0());
    let hint_style = Style::default().fg(theme::overlay0());
    let active_style = Style::default().fg(theme::green()).bg(theme::surface0());
    let sep = Span::raw("  ");

    let is_paused = state.playback.status == PlayStatus::Paused;
    let pause_label = if is_paused { " paused" } else { " pause" };
    let pause_hint = if is_paused { active_style } else { hint_style };

    let shuffle_hint = if state.shuffle { active_style } else { hint_style };
    let shuffle_label = if state.shuffle { " shuffle on" } else { " shuffle" };

    let (repeat_hint, repeat_label) = match state.repeat {
        RepeatMode::Off => (hint_style, " repeat"),
        RepeatMode::One => (active_style, " repeat one"),
        RepeatMode::All => (active_style, " repeat all"),
    };

    let line = Line::from(vec![
        Span::styled(" / ", key_style), Span::styled(" search", hint_style), sep.clone(),
        Span::styled(" Enter ", key_style), Span::styled(" play", hint_style), sep.clone(),
        Span::styled(" Space ", key_style), Span::styled(pause_label, pause_hint), sep.clone(),
        Span::styled(" ←/→ ", key_style), Span::styled(" seek", hint_style), sep.clone(),
        Span::styled(" n ", key_style), Span::styled("/", hint_style), Span::styled(" p ", key_style), Span::styled(" next/prev", hint_style), sep.clone(),
        Span::styled(" s ", key_style), Span::styled(shuffle_label, shuffle_hint), sep.clone(),
        Span::styled(" r ", key_style), Span::styled(repeat_label, repeat_hint), sep.clone(),
        Span::styled(" -/+ ", key_style), Span::styled(" vol", hint_style), sep.clone(),
        Span::styled(" a ", key_style), Span::styled(" playlist", hint_style), sep.clone(),
        Span::styled(" e ", key_style), Span::styled(" eq", hint_style), sep.clone(),
        Span::styled(" t ", key_style), Span::styled(" theme", hint_style), sep.clone(),
        Span::styled(" q ", key_style), Span::styled(" quit", hint_style),
    ]);

    let bar = Paragraph::new(line).style(Style::default().bg(theme::mantle()));
    f.render_widget(bar, area);
}
