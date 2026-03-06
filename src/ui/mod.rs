pub mod components;
pub mod layout;
pub mod theme;

use std::collections::HashMap;

use ratatui::Frame;

use crate::app::state::AppState;

pub fn draw(
    f: &mut Frame,
    state: &AppState,
    thumb_protocol: &mut Option<ratatui_image::protocol::StatefulProtocol>,
    thumb_cache: &mut HashMap<String, ratatui_image::protocol::StatefulProtocol>,
) {
    let app_layout = layout::build_layout(f.area());

    // Background
    let bg = ratatui::widgets::Block::default()
        .style(ratatui::style::Style::default().bg(theme::base()));
    f.render_widget(bg, f.area());

    // Top bar
    components::top_bar::draw(f, app_layout.top_bar, state);

    // Nav panel (hidden on narrow terminals)
    if let Some(nav_area) = app_layout.nav_panel {
        components::nav_panel::draw(f, nav_area, state);
    }

    // Center panel (hidden on medium/narrow terminals)
    if let Some(center_area) = app_layout.center_panel {
        components::now_playing::draw(f, center_area, state, thumb_protocol.as_mut());
    }

    // Queue/search panel (always visible)
    components::queue_panel::draw(f, app_layout.queue_panel, state, thumb_cache);

    // Equalizer / loading bar (hidden on short terminals)
    if let Some(eq_area) = app_layout.equalizer {
        if state.loading.active {
            components::loading_bar::draw(f, eq_area, &state.loading);
        } else {
            components::equalizer::draw(f, eq_area, &state.spectrum, &state.eq_peaks, state.eq_style);
        }
    }

    // Progress bar
    components::progress_bar::draw(f, app_layout.progress_bar, &state.playback, state.frame_count);

    // Help bar
    components::help_bar::draw(f, app_layout.help_bar);

    // Popups
    if state.show_playlist_popup {
        components::popup::draw_playlist_popup(f, state);
    }

    // Theme selector
    if state.theme_selector_timer > 0 {
        components::theme_selector::draw(f);
    }

    // EQ style selector
    if state.eq_selector_timer > 0 {
        components::eq_selector::draw(f, state.eq_style);
    }

    // Toast
    if let Some(ref msg) = state.toast_message {
        draw_toast(f, msg);
    }
}

fn draw_toast(f: &mut Frame, message: &str) {
    let area = f.area();
    let toast_width = (message.len() as u16 + 4).min(area.width);
    let toast_area = ratatui::layout::Rect {
        x: area.width.saturating_sub(toast_width).saturating_sub(1),
        y: 0,
        width: toast_width,
        height: 1,
    };

    let toast = ratatui::widgets::Paragraph::new(ratatui::text::Span::styled(
        format!(" {message} "),
        ratatui::style::Style::default()
            .fg(theme::mantle())
            .bg(theme::surface2()),
    ));
    f.render_widget(toast, toast_area);
}
