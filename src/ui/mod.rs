pub mod components;
pub mod layout;
pub mod theme;

use std::collections::HashMap;

use ratatui::Frame;

use crate::app::state::AppState;
use crate::audio::types::PlayStatus;

pub fn draw(
    f: &mut Frame,
    state: &AppState,
    thumb_protocol: &mut Option<ratatui_image::protocol::StatefulProtocol>,
    thumb_cache: &mut HashMap<String, ratatui_image::protocol::StatefulProtocol>,
) {
    let app_layout = layout::build_layout(f.area());
    let is_playing = matches!(state.playback.status, PlayStatus::Playing | PlayStatus::Paused | PlayStatus::Buffering);

    // Background
    let bg = ratatui::widgets::Block::default()
        .style(ratatui::style::Style::default().bg(theme::BASE));
    f.render_widget(bg, f.area());

    // Top bar
    components::top_bar::draw(f, app_layout.top_bar, state);

    // Nav panel (slim left)
    components::nav_panel::draw(f, app_layout.nav_panel, state);

    // Center panel (now playing — big thumbnail, info, nerd facts)
    if is_playing {
        components::now_playing::draw(f, app_layout.center_panel, state, thumb_protocol.as_mut());
    } else {
        components::now_playing::draw(f, app_layout.center_panel, state, thumb_protocol.as_mut());
    }

    // Queue/search panel (right — YouTube-style cards)
    components::queue_panel::draw(f, app_layout.queue_panel, state, thumb_cache);

    // Equalizer
    components::equalizer::draw(f, app_layout.equalizer, &state.spectrum);

    // Progress bar
    components::progress_bar::draw(f, app_layout.progress_bar, &state.playback);

    // Popups
    if state.show_playlist_popup {
        components::popup::draw_playlist_popup(f, state);
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
            .fg(theme::MANTLE)
            .bg(theme::SURFACE2),
    ));
    f.render_widget(toast, toast_area);
}
