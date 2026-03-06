use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::app::state::LoadingProgress;
use crate::ui::theme;

pub fn draw(f: &mut Frame, area: Rect, loading: &LoadingProgress) {
    if !loading.active {
        return;
    }

    // Render in a centered overlay near the bottom of the given area
    let bar_width = area.width.min(50);
    let x = area.x + (area.width.saturating_sub(bar_width)) / 2;
    let bar_area = Rect {
        x,
        y: area.y,
        width: bar_width,
        height: 2,
    };

    // Message line
    let msg_area = Rect { height: 1, ..bar_area };
    let msg = Paragraph::new(Line::from(Span::styled(
        &loading.message,
        Style::default().fg(theme::subtext0()),
    )))
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(msg, msg_area);

    // Progress bar line
    let prog_area = Rect {
        y: bar_area.y + 1,
        height: 1,
        ..bar_area
    };
    let inner_width = prog_area.width as usize;

    if loading.progress < 0.0 {
        // Indeterminate: bouncing animation
        let tick = loading.completed % (inner_width * 2);
        let pos = if tick < inner_width { tick } else { inner_width * 2 - tick };
        let highlight_len = 4.min(inner_width);

        let mut spans: Vec<Span> = Vec::new();
        let start = pos.saturating_sub(highlight_len / 2);
        let end = (start + highlight_len).min(inner_width);

        if start > 0 {
            spans.push(Span::styled(
                "─".repeat(start),
                Style::default().fg(theme::surface1()),
            ));
        }
        spans.push(Span::styled(
            "━".repeat(end - start),
            Style::default().fg(theme::mauve()),
        ));
        if end < inner_width {
            spans.push(Span::styled(
                "─".repeat(inner_width - end),
                Style::default().fg(theme::surface1()),
            ));
        }

        let bar = Paragraph::new(Line::from(spans));
        f.render_widget(bar, prog_area);
    } else {
        // Determinate: fill bar
        let ratio = loading.progress.clamp(0.0, 1.0);
        let filled = (ratio * inner_width as f64) as usize;

        let mut spans: Vec<Span> = Vec::new();
        if filled > 0 {
            spans.push(Span::styled(
                "━".repeat(filled),
                Style::default().fg(theme::green()),
            ));
        }
        if filled < inner_width {
            spans.push(Span::styled(
                "─".repeat(inner_width - filled),
                Style::default().fg(theme::surface1()),
            ));
        }

        let bar = Paragraph::new(Line::from(spans));
        f.render_widget(bar, prog_area);
    }
}
