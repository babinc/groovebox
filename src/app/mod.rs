pub mod events;
pub mod state;

use std::collections::HashMap;
use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::{mpsc, watch};

use crate::audio::fft;
use crate::audio::mpv::MpvPlayer;
use crate::audio::types::{PlayStatus, PlaybackState, SpectrumData};
use crate::storage::{categories, database, history, playlists};
use crate::youtube::{metadata, search, thumbnail};
use events::AppAction;
use state::AppState;

pub struct App {
    state: AppState,
    mpv: Option<MpvPlayer>,
    playback_rx: Option<mpsc::Receiver<PlaybackState>>,
    spectrum_rx: watch::Receiver<SpectrumData>,
    db: rusqlite::Connection,
    thumb_protocol: Option<ratatui_image::protocol::StatefulProtocol>,
    thumb_cache: HashMap<String, ratatui_image::protocol::StatefulProtocol>,
    image_picker: Option<ratatui_image::picker::Picker>,
}

impl App {
    pub async fn new() -> Result<Self> {
        let db = database::open_database()?;

        let playlist_list = playlists::list_playlists(&db).unwrap_or_default();
        let category_list = categories::list_categories(&db).unwrap_or_default();

        let mut state = AppState::default();
        state.playlists = playlist_list;
        state.categories = category_list;

        let (spectrum_tx, spectrum_rx) = watch::channel(SpectrumData::default());
        fft::spawn_fft_task(spectrum_tx);

        let image_picker = ratatui_image::picker::Picker::from_query_stdio()
            .or_else(|_| {
                let mut picker = ratatui_image::picker::Picker::from_fontsize((8, 16));
                picker.set_protocol_type(ratatui_image::picker::ProtocolType::Halfblocks);
                Ok::<_, std::io::Error>(picker)
            })
            .ok();

        Ok(Self {
            state,
            mpv: None,
            playback_rx: None,
            spectrum_rx,
            db,
            thumb_protocol: None,
            thumb_cache: HashMap::new(),
            image_picker,
        })
    }

    async fn ensure_mpv(&mut self) -> Result<()> {
        if self.mpv.is_none() {
            let (mpv, rx) = MpvPlayer::spawn().await?;
            self.mpv = Some(mpv);
            self.playback_rx = Some(rx);
        }
        Ok(())
    }

    pub async fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        while self.state.running {
            // Drain playback state updates
            if let Some(ref mut rx) = self.playback_rx {
                while let Ok(pb_state) = rx.try_recv() {
                    let track = self.state.playback.current_track.clone();
                    self.state.playback = pb_state;
                    self.state.playback.current_track = track;
                }
            }

            // Update spectrum
            if self.spectrum_rx.has_changed().unwrap_or(false) {
                self.state.spectrum = self.spectrum_rx.borrow_and_update().clone();
            }

            // Update toast timer
            if self.state.toast_timer > 0 {
                self.state.toast_timer -= 1;
                if self.state.toast_timer == 0 {
                    self.state.toast_message = None;
                }
            }

            // Update center panel preview thumbnail when highlighted track changes
            let current_idx = if !self.state.search_results.is_empty() {
                Some(self.state.content_index)
            } else {
                None
            };
            if current_idx != self.state.last_preview_index {
                self.state.last_preview_index = current_idx;
                if let Some(idx) = current_idx {
                    if idx < self.state.search_results.len() {
                        let track = self.state.search_results[idx].clone();
                        self.state.preview_track = Some(track.clone());
                        // Load thumbnail for center panel
                        if self.state.playback.current_track.is_none() {
                            self.load_thumbnail(&track).await;
                        }
                    }
                } else {
                    self.state.preview_track = None;
                    self.thumb_protocol = None;
                }
            }

            // Detect end-of-track for auto-next
            if self.state.playback.status == PlayStatus::Stopped
                && self.state.queue_index.is_some()
            {
                self.handle_auto_next().await;
            }

            // Draw
            terminal.draw(|f| {
                crate::ui::draw(f, &self.state, &mut self.thumb_protocol, &mut self.thumb_cache);
            })?;

            // Handle input
            if event::poll(Duration::from_millis(16))? {
                if let Event::Key(key) = event::read()? {
                    let action = events::handle_key(&mut self.state, key);
                    self.handle_action(action).await;
                }
            }
        }

        self.record_current_play();
        fft::set_fft_active(false);

        Ok(())
    }

    async fn handle_action(&mut self, action: AppAction) {
        match action {
            AppAction::None => {}
            AppAction::Quit => {
                self.state.running = false;
            }
            AppAction::Search(query) => {
                self.state.searching = true;
                match search::search_youtube(&query, 10).await {
                    Ok(results) => {
                        self.state.search_results = results.clone();
                        self.state.queue = results;
                        self.state.content_index = 0;
                        self.state.last_preview_index = None;
                        // Load thumbnails for all search results
                        self.load_search_thumbnails().await;
                    }
                    Err(e) => {
                        self.state.toast_message = Some(format!("Search error: {e}"));
                        self.state.toast_timer = 60;
                    }
                }
                self.state.searching = false;
            }
            AppAction::PlayTrackIndex(idx) => {
                self.play_track_at_index(idx).await;
            }
            AppAction::TogglePause => {
                if let Some(ref mut mpv) = self.mpv {
                    match self.state.playback.status {
                        PlayStatus::Playing => { let _ = mpv.set_pause(true).await; }
                        PlayStatus::Paused => { let _ = mpv.set_pause(false).await; }
                        _ => {}
                    }
                }
            }
            AppAction::Pause => {
                if let Some(ref mut mpv) = self.mpv {
                    let _ = mpv.set_pause(true).await;
                }
            }
            AppAction::Resume => {
                if let Some(ref mut mpv) = self.mpv {
                    let _ = mpv.set_pause(false).await;
                }
            }
            AppAction::Seek(secs) => {
                if let Some(ref mut mpv) = self.mpv {
                    let _ = mpv.seek(secs).await;
                }
            }
            AppAction::VolumeUp => {
                let new_vol = (self.state.playback.volume + 5.0).min(150.0);
                if let Some(ref mut mpv) = self.mpv {
                    let _ = mpv.set_volume(new_vol).await;
                }
                self.state.playback.volume = new_vol;
            }
            AppAction::VolumeDown => {
                let new_vol = (self.state.playback.volume - 5.0).max(0.0);
                if let Some(ref mut mpv) = self.mpv {
                    let _ = mpv.set_volume(new_vol).await;
                }
                self.state.playback.volume = new_vol;
            }
            AppAction::NextTrack => { self.next_track().await; }
            AppAction::PrevTrack => { self.prev_track().await; }
            AppAction::AddToPlaylist => {
                if self.state.playlists.is_empty() {
                    self.state.toast_message = Some("Create a playlist first".into());
                    self.state.toast_timer = 40;
                }
            }
            AppAction::CreatePlaylist(name, desc) => {
                match playlists::create_playlist(&self.db, &name, &desc, None) {
                    Ok(_) => {
                        self.state.playlists = playlists::list_playlists(&self.db).unwrap_or_default();
                        self.state.toast_message = Some(format!("Created: {name}"));
                        self.state.toast_timer = 30;
                        if let Some(ref track) = self.state.playback.current_track {
                            if let Some(pl) = self.state.playlists.last() {
                                if let Some(id) = pl.id {
                                    let _ = playlists::add_track_to_playlist(&self.db, id, track);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        self.state.toast_message = Some(format!("Error: {e}"));
                        self.state.toast_timer = 40;
                    }
                }
            }
            AppAction::LoadPlaylists => {
                self.state.playlists = playlists::list_playlists(&self.db).unwrap_or_default();
            }
            AppAction::LoadHistory => {
                self.state.history = history::get_history(&self.db, 50).unwrap_or_default();
            }
            AppAction::LoadPlaylistTracks(id) => {
                if let Ok(tracks) = playlists::get_playlist_tracks(&self.db, id) {
                    self.state.search_results = tracks;
                    self.state.content_index = 0;
                }
            }
        }
    }

    /// Load thumbnails for all search results into the cache
    async fn load_search_thumbnails(&mut self) {
        let tracks: Vec<(String, String)> = self.state.search_results.iter()
            .filter(|t| !t.thumbnail_url.is_empty() && !self.thumb_cache.contains_key(&t.youtube_id))
            .map(|t| (t.youtube_id.clone(), t.thumbnail_url.clone()))
            .collect();

        for (youtube_id, thumb_url) in tracks {
            if let Some(ref mut picker) = self.image_picker {
                if let Ok(path) = thumbnail::download_thumbnail(&thumb_url, &youtube_id).await {
                    if let Ok(reader) = image::ImageReader::open(&path) {
                        if let Ok(reader) = reader.with_guessed_format() {
                            if let Ok(img) = reader.decode() {
                                let protocol = picker.new_resize_protocol(img);
                                self.thumb_cache.insert(youtube_id, protocol);
                            }
                        }
                    }
                }
            }
        }
    }

    async fn play_track_at_index(&mut self, idx: usize) {
        let track = if idx < self.state.queue.len() {
            self.state.queue[idx].clone()
        } else if idx < self.state.search_results.len() {
            self.state.search_results[idx].clone()
        } else {
            return;
        };

        self.record_current_play();

        self.state.queue_index = Some(idx);
        self.state.playback.current_track = Some(track.clone());
        self.state.playback.status = PlayStatus::Buffering;

        self.state.toast_message = Some(format!("Loading: {}", track.title));
        self.state.toast_timer = 60;

        match metadata::get_audio_url(&track.youtube_url).await {
            Ok(audio_url) => {
                if let Err(e) = self.ensure_mpv().await {
                    self.state.toast_message = Some(format!("mpv error: {e}"));
                    self.state.toast_timer = 60;
                    return;
                }

                if let Some(ref mut mpv) = self.mpv {
                    if let Err(e) = mpv.load_file(&audio_url).await {
                        self.state.toast_message = Some(format!("Play error: {e}"));
                        self.state.toast_timer = 60;
                        return;
                    }
                }

                self.state.toast_message = Some(format!("Playing: {}", track.title));
                self.state.toast_timer = 30;
                fft::set_fft_active(true);

                self.load_thumbnail(&track).await;

                if let Ok(full_track) = metadata::get_full_metadata(&track.youtube_url).await {
                    if let Some(ref mut current) = self.state.playback.current_track {
                        current.codec = full_track.codec.or(current.codec.clone());
                        current.bitrate = full_track.bitrate.or(current.bitrate);
                        current.sample_rate = full_track.sample_rate.or(current.sample_rate);
                        current.channels = full_track.channels.or(current.channels);
                        current.filesize = full_track.filesize.or(current.filesize);
                    }
                }
            }
            Err(e) => {
                self.state.toast_message = Some(format!("URL error: {e}"));
                self.state.toast_timer = 60;
                self.state.playback.status = PlayStatus::Stopped;
                fft::set_fft_active(false);
            }
        }
    }

    async fn load_thumbnail(&mut self, track: &crate::models::Track) {
        if track.thumbnail_url.is_empty() {
            self.thumb_protocol = None;
            return;
        }

        if let Some(ref mut picker) = self.image_picker {
            if let Ok(path) = thumbnail::download_thumbnail(&track.thumbnail_url, &track.youtube_id).await {
                if let Ok(reader) = image::ImageReader::open(&path) {
                    if let Ok(reader) = reader.with_guessed_format() {
                        if let Ok(img) = reader.decode() {
                            let protocol = picker.new_resize_protocol(img);
                            self.thumb_protocol = Some(protocol);
                            return;
                        }
                    }
                }
            }
        }
        self.thumb_protocol = None;
    }

    async fn next_track(&mut self) {
        if self.state.queue.is_empty() { return; }
        let next_idx = if self.state.shuffle {
            use std::time::{SystemTime, UNIX_EPOCH};
            let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos() as usize;
            seed % self.state.queue.len()
        } else if let Some(idx) = self.state.queue_index {
            let next = idx + 1;
            if next >= self.state.queue.len() {
                match self.state.repeat { state::RepeatMode::All => 0, _ => return }
            } else { next }
        } else { 0 };
        self.play_track_at_index(next_idx).await;
    }

    async fn prev_track(&mut self) {
        if self.state.queue.is_empty() { return; }
        let prev_idx = if let Some(idx) = self.state.queue_index {
            if idx == 0 {
                match self.state.repeat { state::RepeatMode::All => self.state.queue.len() - 1, _ => 0 }
            } else { idx - 1 }
        } else { 0 };
        self.play_track_at_index(prev_idx).await;
    }

    async fn handle_auto_next(&mut self) {
        match self.state.repeat {
            state::RepeatMode::One => {
                if let Some(idx) = self.state.queue_index { self.play_track_at_index(idx).await; }
            }
            state::RepeatMode::All => { self.next_track().await; }
            state::RepeatMode::Off => {
                if let Some(idx) = self.state.queue_index {
                    if idx + 1 < self.state.queue.len() { self.next_track().await; }
                }
            }
        }
    }

    fn record_current_play(&self) {
        if let Some(ref track) = self.state.playback.current_track {
            let duration_listened = self.state.playback.position;
            let completed = self.state.playback.duration > 0.0
                && duration_listened >= self.state.playback.duration * 0.9;
            let _ = history::record_play(&self.db, track, duration_listened, completed);
        }
    }
}
