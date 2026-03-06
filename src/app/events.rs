use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::state::{AppState, ContentView, Focus, NavSection, RepeatMode};

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
        KeyCode::Tab => {
            state.focus = match state.focus {
                Focus::Navigation => Focus::Queue,
                Focus::Queue => Focus::Center,
                Focus::Center => Focus::Navigation,
                Focus::SearchInput => Focus::Queue,
            };
            AppAction::None
        }
        KeyCode::BackTab => {
            state.focus = match state.focus {
                Focus::Navigation => Focus::Center,
                Focus::Queue => Focus::Navigation,
                Focus::Center => Focus::Queue,
                Focus::SearchInput => Focus::Navigation,
            };
            AppAction::None
        }
        KeyCode::Char('/') => {
            state.focus = Focus::SearchInput;
            state.content_view = ContentView::SearchResults;
            AppAction::None
        }
        KeyCode::Char(' ') => AppAction::TogglePause,
        KeyCode::Left => AppAction::Seek(-5.0),
        KeyCode::Right => AppAction::Seek(5.0),
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
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            state.nav_section = match state.nav_section {
                NavSection::Search => NavSection::Playlists,
                NavSection::Playlists => NavSection::Categories,
                NavSection::Categories => NavSection::History,
                NavSection::History => NavSection::Search,
            };
            AppAction::None
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.nav_section = match state.nav_section {
                NavSection::Search => NavSection::History,
                NavSection::Playlists => NavSection::Search,
                NavSection::Categories => NavSection::Playlists,
                NavSection::History => NavSection::Categories,
            };
            AppAction::None
        }
        KeyCode::Enter => {
            match state.nav_section {
                NavSection::Search => {
                    state.focus = Focus::SearchInput;
                    state.content_view = ContentView::SearchResults;
                }
                NavSection::Playlists => {
                    state.content_view = ContentView::SearchResults;
                    return AppAction::LoadPlaylists;
                }
                NavSection::History => {
                    state.content_view = ContentView::HistoryList;
                    return AppAction::LoadHistory;
                }
                NavSection::Categories => {}
            }
            AppAction::None
        }
        _ => AppAction::None,
    }
}

fn handle_queue_key(state: &mut AppState, key: KeyEvent) -> AppAction {
    let list_len = match &state.content_view {
        ContentView::SearchResults => state.search_results.len(),
        ContentView::HistoryList => state.history.len(),
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
