use crate::audio::types::{PlaybackState, SpectrumData};
use crate::models::{Category, PlayHistoryEntry, Playlist, Track};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Navigation,
    Center,
    Queue,
    SearchInput,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NavSection {
    Search,
    Playlists,
    Categories,
    History,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContentView {
    SearchResults,
    PlaylistTracks(i64),
    CategoryPlaylists(i64),
    HistoryList,
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RepeatMode {
    Off,
    One,
    All,
}

pub struct AppState {
    pub running: bool,
    pub focus: Focus,
    pub nav_section: NavSection,
    pub nav_index: usize,
    pub content_view: ContentView,
    pub content_index: usize,

    // Search
    pub search_query: String,
    pub search_results: Vec<Track>,
    pub searching: bool,

    // Playback
    pub playback: PlaybackState,
    pub spectrum: SpectrumData,
    pub shuffle: bool,
    pub repeat: RepeatMode,

    // Play queue
    pub queue: Vec<Track>,
    pub queue_index: Option<usize>,

    // Data
    pub playlists: Vec<Playlist>,
    pub categories: Vec<Category>,
    pub history: Vec<PlayHistoryEntry>,

    // Preview (highlighted track in search results)
    pub preview_track: Option<Track>,
    pub last_preview_index: Option<usize>,

    // Popup
    pub show_playlist_popup: bool,
    pub popup_input: String,
    pub popup_description: String,

    // Toast
    pub toast_message: Option<String>,
    pub toast_timer: u8,

    // Queue scroll offset for YouTube-style cards
    pub queue_scroll: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            running: true,
            focus: Focus::Navigation,
            nav_section: NavSection::Search,
            nav_index: 0,
            content_view: ContentView::Empty,
            content_index: 0,
            search_query: String::new(),
            search_results: Vec::new(),
            searching: false,
            playback: PlaybackState::default(),
            spectrum: SpectrumData::default(),
            shuffle: false,
            repeat: RepeatMode::Off,
            preview_track: None,
            last_preview_index: None,
            queue: Vec::new(),
            queue_index: None,
            playlists: Vec::new(),
            categories: Vec::new(),
            history: Vec::new(),
            show_playlist_popup: false,
            popup_input: String::new(),
            popup_description: String::new(),
            toast_message: None,
            toast_timer: 0,
            queue_scroll: 0,
        }
    }
}
