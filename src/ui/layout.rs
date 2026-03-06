use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub top_bar: Rect,
    pub nav_panel: Rect,
    pub center_panel: Rect,
    pub queue_panel: Rect,
    pub equalizer: Rect,
    pub progress_bar: Rect,
}

pub fn build_layout(area: Rect) -> AppLayout {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),         // top bar
            Constraint::Min(12),           // middle panels
            Constraint::Length(5),          // equalizer
            Constraint::Length(1),          // progress bar
        ])
        .split(area);

    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(16),         // nav (slim)
            Constraint::Percentage(55),    // center (now playing + info)
            Constraint::Min(30),           // queue/search (right)
        ])
        .split(main_chunks[1]);

    AppLayout {
        top_bar: main_chunks[0],
        nav_panel: middle_chunks[0],
        center_panel: middle_chunks[1],
        queue_panel: middle_chunks[2],
        equalizer: main_chunks[2],
        progress_bar: main_chunks[3],
    }
}
