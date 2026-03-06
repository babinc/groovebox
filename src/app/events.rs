use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::state::{AppState, ContentView, Focus, NavSection, Preferences, RepeatMode};

#[derive(Debug)]
pub enum AppAction {
    None,
    Quit,
    Search(String),
    PlayTrackIndex(usize),
    Pause,
    Resume,
    TogglePause,
    Seek(f64),
    VolumeUp,
    VolumeDown,
    NextTrack,
    PrevTrack,
    AddToPlaylist,
    CreatePlaylist(String, String),
    LoadPlaylists,
    LoadHistory,
    LoadPlaylistTracks(i64),
    CycleTheme,
    CycleEq,
    ToggleSetting(usize),
}

pub fn handle_key(state: &mut AppState, key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return AppAction::Quit;
        }
        _ => {}
    }

    if state.show_playlist_popup {
        return handle_popup_key(state, key);
    }

    if state.focus == Focus::SearchInput {
        return handle_search_input(state, key);
    }

    match key.code {
        KeyCode::Char('q') => AppAction::Quit,
        KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') if state.focus == Focus::Navigation => {
            state.focus = Focus::Queue;
            AppAction::None
        }
        KeyCode::Tab => {
            state.focus = match state.focus {
                Focus::Navigation => Focus::Queue,
                Focus::Queue => Focus::Navigation,
                Focus::SearchInput => Focus::Queue,
                _ => Focus::Navigation,
            };
            AppAction::None
        }
        KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') if state.focus == Focus::Queue => {
            state.focus = Focus::Navigation;
            AppAction::None
        }
        KeyCode::BackTab => {
            state.focus = match state.focus {
                Focus::Navigation => Focus::Queue,
                Focus::Queue => Focus::Navigation,
                Focus::SearchInput => Focus::Navigation,
                _ => Focus::Navigation,
            };
            AppAction::None
        }
        KeyCode::Char('/') => {
            state.focus = Focus::SearchInput;
            state.content_view = ContentView::SearchResults;
            AppAction::None
        }
        KeyCode::Char(' ') => AppAction::TogglePause,
        KeyCode::Left if state.playback.status != crate::audio::types::PlayStatus::Stopped => seek_with_accel(state, -1.0),
        KeyCode::Right if state.playback.status != crate::audio::types::PlayStatus::Stopped => seek_with_accel(state, 1.0),
        KeyCode::Char('+') | KeyCode::Char('=') => AppAction::VolumeUp,
        KeyCode::Char('-') => AppAction::VolumeDown,
        KeyCode::Char('n') => AppAction::NextTrack,
        KeyCode::Char('p') => AppAction::PrevTrack,
        KeyCode::Char('a') => {
            state.show_playlist_popup = true;
            state.popup_input.clear();
            state.popup_description.clear();
            AppAction::AddToPlaylist
        }
        KeyCode::Char('s') => {
            state.shuffle = !state.shuffle;
            state.toast_message = Some(format!("Shuffle: {}", if state.shuffle { "On" } else { "Off" }));
            state.toast_timer = 30;
            AppAction::None
        }
        KeyCode::Char('t') => AppAction::CycleTheme,
        KeyCode::Char('e') => AppAction::CycleEq,
        KeyCode::Char('r') => {
            state.repeat = match state.repeat {
                RepeatMode::Off => RepeatMode::One,
                RepeatMode::One => RepeatMode::All,
                RepeatMode::All => RepeatMode::Off,
            };
            let label = match state.repeat {
                RepeatMode::Off => "Off",
                RepeatMode::One => "One",
                RepeatMode::All => "All",
            };
            state.toast_message = Some(format!("Repeat: {label}"));
            state.toast_timer = 30;
            AppAction::None
        }
        _ => handle_panel_key(state, key),
    }
}

fn handle_panel_key(state: &mut AppState, key: KeyEvent) -> AppAction {
    match state.focus {
        Focus::Navigation => handle_nav_key(state, key),
        Focus::Queue => handle_queue_key(state, key),
        _ => AppAction::None,
    }
}

fn handle_nav_key(state: &mut AppState, key: KeyEvent) -> AppAction {
    let sections = [NavSection::Search, NavSection::Playlists, NavSection::History, NavSection::Settings];
    let section_idx = sections.iter().position(|s| *s == state.nav_section).unwrap_or(0);

    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            // If on Playlists and there are sub-items, navigate within them first
            if state.nav_section == NavSection::Playlists && !state.playlists.is_empty() {
                if state.nav_sub_index < state.playlists.len().saturating_sub(1) {
                    state.nav_sub_index += 1;
                    return AppAction::None;
                }
            }
            // Move to next section
            let next = (section_idx + 1) % sections.len();
            state.nav_section = sections[next];
            state.nav_sub_index = 0;
            AppAction::None
        }
        KeyCode::Char('k') | KeyCode::Up => {
            // If on Playlists and sub-index > 0, navigate up within playlists
            if state.nav_section == NavSection::Playlists && state.nav_sub_index > 0 {
                state.nav_sub_index -= 1;
                return AppAction::None;
            }
            // Move to previous section
            let prev = if section_idx == 0 { sections.len() - 1 } else { section_idx - 1 };
            state.nav_section = sections[prev];
            // If landing on Playlists, start at last item
            if state.nav_section == NavSection::Playlists && !state.playlists.is_empty() {
                state.nav_sub_index = state.playlists.len() - 1;
            } else {
                state.nav_sub_index = 0;
            }
            AppAction::None
        }
        KeyCode::Enter => {
            match state.nav_section {
                NavSection::Search => {
                    state.focus = Focus::SearchInput;
                    state.content_view = ContentView::SearchResults;
                }
                NavSection::Playlists => {
                    if let Some(pl) = state.playlists.get(state.nav_sub_index) {
                        if let Some(id) = pl.id {
                            state.content_view = ContentView::PlaylistTracks(id);
                            state.content_index = 0;
                            state.focus = Focus::Queue;
                            return AppAction::LoadPlaylistTracks(id);
                        }
                    }
                    // No playlists yet — show empty playlist view
                    state.content_view = ContentView::SearchResults;
                    state.search_results.clear();
                    state.queue.clear();
                    state.content_index = 0;
                    state.toast_message = Some("No playlists yet — play a track and press 'a' to create one".into());
                    state.toast_timer = 60;
                    return AppAction::LoadPlaylists;
                }
                NavSection::History => {
                    state.content_view = ContentView::HistoryList;
                    state.content_index = 0;
                    return AppAction::LoadHistory;
                }
                NavSection::Settings => {
                    state.content_view = ContentView::Settings;
                    state.settings_index = 0;
                    state.focus = Focus::Queue;
                }
            }
            AppAction::None
        }
        _ => AppAction::None,
    }
}

fn handle_queue_key(state: &mut AppState, key: KeyEvent) -> AppAction {
    // Settings view has its own navigation
    if state.content_view == ContentView::Settings {
        let num_settings = Preferences::KEYS.len();
        return match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if num_settings > 0 {
                    state.settings_index = (state.settings_index + 1).min(num_settings - 1);
                }
                AppAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if state.settings_index > 0 {
                    state.settings_index -= 1;
                }
                AppAction::None
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                AppAction::ToggleSetting(state.settings_index)
            }
            _ => AppAction::None,
        };
    }

    let list_len = match &state.content_view {
        ContentView::SearchResults => state.search_results.len(),
        ContentView::PlaylistTracks(_) => state.search_results.len(),
        ContentView::HistoryList => state.search_results.len(),
        _ => 0,
    };

    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if list_len > 0 {
                state.content_index = (state.content_index + 1).min(list_len - 1);
            }
            AppAction::None
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if state.content_index > 0 {
                state.content_index -= 1;
            }
            AppAction::None
        }
        KeyCode::Enter => {
            if list_len > 0 && state.content_index < list_len {
                AppAction::PlayTrackIndex(state.content_index)
            } else {
                AppAction::None
            }
        }
        _ => AppAction::None,
    }
}

fn handle_search_input(state: &mut AppState, key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Enter => {
            let query = state.search_query.clone();
            if !query.is_empty() {
                state.searching = true;
                state.focus = Focus::Queue;
                return AppAction::Search(query);
            }
            AppAction::None
        }
        KeyCode::Esc => {
            state.focus = Focus::Queue;
            AppAction::None
        }
        KeyCode::Backspace => {
            state.search_query.pop();
            AppAction::None
        }
        KeyCode::Char(c) => {
            state.search_query.push(c);
            AppAction::None
        }
        _ => AppAction::None,
    }
}

fn seek_with_accel(state: &mut AppState, direction: f64) -> AppAction {
    // If seeking within ~10 frames of last seek, increase streak
    let gap = state.frame_count.saturating_sub(state.last_seek_frame);
    if gap < 15 {
        state.seek_streak += 1;
    } else {
        state.seek_streak = 1;
    }
    state.last_seek_frame = state.frame_count;

    // Accelerate: 5s -> 10s -> 15s -> 30s -> 60s
    let secs = match state.seek_streak {
        0..=3 => 5.0,
        4..=8 => 10.0,
        9..=15 => 15.0,
        16..=25 => 30.0,
        _ => 60.0,
    };

    let label = if secs < 60.0 { format!("{:.0}s", secs) } else { format!("{:.0}m", secs / 60.0) };
    let arrow = if direction > 0.0 { ">>" } else { "<<" };
    state.toast_message = Some(format!("{arrow} {label}"));
    state.toast_timer = 15;

    AppAction::Seek(direction * secs)
}

fn handle_popup_key(state: &mut AppState, key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Esc => {
            state.show_playlist_popup = false;
            AppAction::None
        }
        KeyCode::Enter => {
            let name = state.popup_input.clone();
            let desc = state.popup_description.clone();
            state.show_playlist_popup = false;
            if !name.is_empty() {
                AppAction::CreatePlaylist(name, desc)
            } else {
                AppAction::None
            }
        }
        KeyCode::Backspace => {
            state.popup_input.pop();
            AppAction::None
        }
        KeyCode::Char(c) => {
            state.popup_input.push(c);
            AppAction::None
        }
        _ => AppAction::None,
    }
}
