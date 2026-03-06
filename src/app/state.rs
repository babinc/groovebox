use crate::audio::types::{PlaybackState, SpectrumData};
use crate::models::{PlayHistoryEntry, Playlist, Track};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoadingKind {
    Search,
    Buffering,
    Thumbnails,
    Playlist,
}

#[derive(Debug, Clone)]
pub struct LoadingProgress {
    pub active: bool,
    pub kind: LoadingKind,
    pub message: String,
    pub progress: f64, // 0.0..1.0, negative means indeterminate
    pub total: usize,
    pub completed: usize,
}

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
    History,
    Settings,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContentView {
    SearchResults,
    PlaylistTracks(i64),
    CategoryPlaylists(i64),
    HistoryList,
    Settings,
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RepeatMode {
    Off,
    One,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EqStyle {
    Bars,
    BarsSpread,
    Blocks,
    BlocksSpread,
    Peaks,
    PeaksSpread,
    Mirror,
    MirrorSpread,
    Wave,
    WaveSpread,
    Haze,
    HazeSpread,
}

impl EqStyle {
    pub const ALL: [EqStyle; 12] = [
        EqStyle::Bars,
        EqStyle::BarsSpread,
        EqStyle::Blocks,
        EqStyle::BlocksSpread,
        EqStyle::Peaks,
        EqStyle::PeaksSpread,
        EqStyle::Mirror,
        EqStyle::MirrorSpread,
        EqStyle::Wave,
        EqStyle::WaveSpread,
        EqStyle::Haze,
        EqStyle::HazeSpread,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EqStyle::Bars => "Bars",
            EqStyle::BarsSpread => "Bars Spread",
            EqStyle::Blocks => "Blocks",
            EqStyle::BlocksSpread => "Blocks Spread",
            EqStyle::Peaks => "Peaks",
            EqStyle::PeaksSpread => "Peaks Spread",
            EqStyle::Mirror => "Mirror",
            EqStyle::MirrorSpread => "Mirror Spread",
            EqStyle::Wave => "Wave",
            EqStyle::WaveSpread => "Wave Spread",
            EqStyle::Haze => "Haze",
            EqStyle::HazeSpread => "Haze Spread",
        }
    }

    pub fn next(self) -> Self {
        let idx = EqStyle::ALL.iter().position(|&s| s == self).unwrap_or(0);
        EqStyle::ALL[(idx + 1) % EqStyle::ALL.len()]
    }
}

#[derive(Debug, Clone)]
pub struct Preferences {
    pub auto_resume: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            auto_resume: true,
        }
    }
}

impl Preferences {
    pub const KEYS: &'static [(&'static str, &'static str)] = &[
        ("auto_resume", "Resume playback on startup"),
    ];

    pub fn get(&self, key: &str) -> bool {
        match key {
            "auto_resume" => self.auto_resume,
            _ => false,
        }
    }

    pub fn toggle(&mut self, key: &str) {
        match key {
            "auto_resume" => self.auto_resume = !self.auto_resume,
            _ => {}
        }
    }
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
    pub history: Vec<PlayHistoryEntry>,

    // Nav panel sub-item index (e.g., which playlist is selected)
    pub nav_sub_index: usize,

    // Preview (highlighted track in search results)
    pub preview_track: Option<Track>,
    pub last_preview_index: Option<usize>,

    // Popup
    pub show_playlist_popup: bool,
    pub popup_input: String,
    pub popup_description: String,

    // Loading progress
    pub loading: LoadingProgress,

    // Toast
    pub toast_message: Option<String>,
    pub toast_timer: u8,

    // Queue scroll offset for YouTube-style cards
    pub queue_scroll: usize,

    // Theme selector (timer > 0 means visible)
    pub theme_selector_timer: u8,

    // EQ visualizer
    pub eq_style: EqStyle,
    pub eq_peaks: [f32; 64],
    pub eq_selector_timer: u8,

    // Preferences
    pub preferences: Preferences,
    pub settings_index: usize,

    // Frame counter for animations
    pub frame_count: usize,

    // Seek acceleration
    pub seek_streak: u32,
    pub last_seek_frame: usize,

    // Guard against duplicate fetch_related spawns
    pub fetching_related: bool,
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
            nav_sub_index: 0,
            preview_track: None,
            last_preview_index: None,
            queue: Vec::new(),
            queue_index: None,
            playlists: Vec::new(),
            history: Vec::new(),
            loading: LoadingProgress {
                active: false,
                kind: LoadingKind::Search,
                message: String::new(),
                progress: -1.0,
                total: 0,
                completed: 0,
            },
            show_playlist_popup: false,
            popup_input: String::new(),
            popup_description: String::new(),
            toast_message: None,
            toast_timer: 0,
            queue_scroll: 0,
            theme_selector_timer: 0,
            eq_style: EqStyle::Bars,
            eq_peaks: [0.0; 64],
            eq_selector_timer: 0,
            preferences: Preferences::default(),
            settings_index: 0,
            frame_count: 0,
            seek_streak: 0,
            last_seek_frame: 0,
            fetching_related: false,
        }
    }
}
