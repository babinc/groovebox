use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub top_bar: Rect,
    pub nav_panel: Option<Rect>,
    pub center_panel: Option<Rect>,
    pub queue_panel: Rect,
    pub equalizer: Option<Rect>,
    pub progress_bar: Rect,
    pub help_bar: Rect,
}

pub fn build_layout(area: Rect) -> AppLayout {
    let w = area.width;
    let h = area.height;

    // Adaptive vertical: hide EQ on short terminals, shrink top bar
    let show_eq = h >= 20;
    let eq_height = if h >= 30 { 8 } else if h >= 24 { 5 } else { 3 };
    let top_bar_height = if h >= 15 { 3 } else { 1 };

    let mut v_constraints = vec![
        Constraint::Length(top_bar_height),   // top bar
        Constraint::Min(6),                   // middle panels
    ];
    if show_eq {
        v_constraints.push(Constraint::Length(eq_height));
    }
    v_constraints.push(Constraint::Length(1)); // progress bar
    v_constraints.push(Constraint::Length(1)); // help bar

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(v_constraints)
        .split(area);

    // Adaptive horizontal: collapse panels based on width
    let show_nav = w >= 40;
    let show_center = w >= 100;
    let nav_width = if w >= 80 { 16 } else { 12 };

    let middle = main_chunks[1];

    let queue_width = if w >= 200 { 60 } else if w >= 140 { 50 } else { 40 };

    let (nav_panel, center_panel, queue_panel) = if show_center && show_nav {
        // Full 3-panel layout: nav fixed, queue capped, center gets the rest
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(nav_width),
                Constraint::Min(30),
                Constraint::Length(queue_width),
            ])
            .split(middle);
        (Some(chunks[0]), Some(chunks[1]), chunks[2])
    } else if show_nav {
        // 2-panel: nav + queue (no center/now-playing)
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(nav_width),
                Constraint::Min(20),
            ])
            .split(middle);
        (Some(chunks[0]), None, chunks[1])
    } else {
        // Tiny: queue only
        (None, None, middle)
    };

    let eq_idx = 2;
    let progress_idx = if show_eq { 3 } else { 2 };
    let help_idx = if show_eq { 4 } else { 3 };

    AppLayout {
        top_bar: main_chunks[0],
        nav_panel,
        center_panel,
        queue_panel,
        equalizer: if show_eq { Some(main_chunks[eq_idx]) } else { None },
        progress_bar: main_chunks[progress_idx],
        help_bar: main_chunks[help_idx],
    }
}
