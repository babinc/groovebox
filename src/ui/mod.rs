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

    // Nav panel (slim left)
    components::nav_panel::draw(f, app_layout.nav_panel, state);

    // Center panel (now playing — big thumbnail, info, nerd facts)
    components::now_playing::draw(f, app_layout.center_panel, state, thumb_protocol.as_mut());

    // Queue/search panel (right — YouTube-style cards)
    components::queue_panel::draw(f, app_layout.queue_panel, state, thumb_cache);

    // Loading bar (overlays equalizer area when active)
    if state.loading.active {
        components::loading_bar::draw(f, app_layout.equalizer, &state.loading);
    } else {
        // Equalizer
        components::equalizer::draw(f, app_layout.equalizer, &state.spectrum, &state.eq_peaks, state.eq_style);
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
