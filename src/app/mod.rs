pub mod events;
pub mod state;

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::{mpsc, watch};

use crate::audio::fft;
use crate::audio::mpv::MpvPlayer;
use crate::audio::types::{PlayStatus, PlaybackState, SpectrumData};
use crate::models::Track;
use crate::storage::{categories, database, history, playlists};
use crate::youtube::{metadata, search, thumbnail};
use events::AppAction;
use state::AppState;

/// Results from background tasks sent back to the main loop.
enum BgResult {
    SearchDone(Result<Vec<Track>>),
    ThumbnailReady(String, PathBuf), // youtube_id, path
    ThumbnailBatchDone,
    AudioUrlReady(usize, Track, Result<String>), // idx, track, url
    MetadataReady(Result<Track>),
    PlaylistTracksReady(Result<Vec<Track>>),
}

pub struct App {
    state: AppState,
    mpv: Option<MpvPlayer>,
    playback_rx: Option<mpsc::Receiver<PlaybackState>>,
    bg_tx: mpsc::Sender<BgResult>,
    bg_rx: mpsc::Receiver<BgResult>,
    spectrum_rx: watch::Receiver<SpectrumData>,
    db: rusqlite::Connection,
    thumb_protocol: Option<ratatui_image::protocol::StatefulProtocol>,
    thumb_protocol_id: String, // youtube_id of the currently loaded thumb_protocol
    thumb_cache: HashMap<String, ratatui_image::protocol::StatefulProtocol>,
    image_picker: Option<ratatui_image::picker::Picker>,
    pending_play: Option<(Track, String)>,
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

        let (bg_tx, bg_rx) = mpsc::channel(64);

        Ok(Self {
            state,
            mpv: None,
            playback_rx: None,
            bg_tx,
            bg_rx,
            spectrum_rx,
            db,
            thumb_protocol: None,
            thumb_protocol_id: String::new(),
            thumb_cache: HashMap::new(),
            image_picker,
            pending_play: None,
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
        let mut preview_debounce: u8 = 0;

        while self.state.running {
            // 1. Handle ALL pending input first (drain the queue so navigation feels instant)
            let mut had_input = false;
            while event::poll(Duration::from_millis(0))? {
                if let Event::Key(key) = event::read()? {
                    let action = events::handle_key(&mut self.state, key);
                    self.handle_action(action).await;
                    had_input = true;
                }
            }

            // 2. Drain channel updates (non-blocking)
            if let Some(ref mut rx) = self.playback_rx {
                while let Ok(pb_state) = rx.try_recv() {
                    let track = self.state.playback.current_track.clone();
                    let volume = self.state.playback.volume;
                    self.state.playback = pb_state;
                    self.state.playback.current_track = track;
                    if (self.state.playback.volume - volume).abs() > 0.5 {
                        self.state.playback.volume = volume;
                    }
                }
            }

            while let Ok(result) = self.bg_rx.try_recv() {
                self.handle_bg_result(result);
            }

            self.process_pending_play().await;

            // 3. Update spectrum
            if self.spectrum_rx.has_changed().unwrap_or(false) {
                self.state.spectrum = self.spectrum_rx.borrow_and_update().clone();
            }

            // 4. Toast timer
            if self.state.toast_timer > 0 {
                self.state.toast_timer -= 1;
                if self.state.toast_timer == 0 {
                    self.state.toast_message = None;
                }
            }

            // 5. Loading animation tick
            if self.state.loading.active && self.state.loading.progress < 0.0 {
                self.state.loading.completed = self.state.loading.completed.wrapping_add(1);
            }

            // 6. Debounced preview thumbnail — only decode after cursor settles
            let current_idx = if !self.state.search_results.is_empty() {
                Some(self.state.content_index)
            } else {
                None
            };
            if current_idx != self.state.last_preview_index {
                // Cursor moved — update preview track immediately (lightweight)
                if let Some(idx) = current_idx {
                    if idx < self.state.search_results.len() {
                        self.state.preview_track = Some(self.state.search_results[idx].clone());
                    }
                } else {
                    self.state.preview_track = None;
                    self.thumb_protocol = None;
                }
                self.state.last_preview_index = current_idx;
                preview_debounce = 3; // wait 3 frames (~50ms) before decoding thumbnail
            } else if preview_debounce > 0 {
                preview_debounce -= 1;
                if preview_debounce == 0 {
                    // Cursor settled — now do the expensive thumbnail decode
                    if self.state.playback.current_track.is_none() {
                        if let Some(track) = self.state.preview_track.clone() {
                            self.load_thumbnail_sync(&track);
                        }
                    }
                }
            }

            // 7. Detect end-of-track for auto-next
            if self.state.playback.status == PlayStatus::Stopped
                && self.state.queue_index.is_some()
            {
                self.handle_auto_next().await;
            }

            // 8. Draw
            terminal.draw(|f| {
                crate::ui::draw(f, &self.state, &mut self.thumb_protocol, &mut self.thumb_cache);
            })?;

            // 9. If no input this frame, sleep briefly to avoid busy-loop
            if !had_input {
                if event::poll(Duration::from_millis(16))? {
                    // Will be read at top of next iteration
                }
            }
        }

        self.record_current_play();
        fft::set_fft_active(false);

        Ok(())
    }

    fn handle_bg_result(&mut self, result: BgResult) {
        match result {
            BgResult::SearchDone(res) => {
                self.state.searching = false;
                self.state.loading.active = false;
                match res {
                    Ok(results) => {
                        self.state.search_results = results.clone();
                        self.state.queue = results;
                        self.state.content_index = 0;
                        self.state.last_preview_index = None;
                        // Spawn thumbnail loading in background
                        self.spawn_thumbnail_batch();
                    }
                    Err(e) => {
                        self.state.toast_message = Some(format!("Search error: {e}"));
                        self.state.toast_timer = 60;
                    }
                }
            }
            BgResult::ThumbnailReady(youtube_id, path) => {
                if let Some(ref mut picker) = self.image_picker {
                    if let Ok(reader) = image::ImageReader::open(&path) {
                        if let Ok(reader) = reader.with_guessed_format() {
                            if let Ok(img) = reader.decode() {
                                let protocol = picker.new_resize_protocol(img);
                                self.thumb_cache.insert(youtube_id, protocol);
                            }
                        }
                    }
                }
                // Update loading progress
                if self.state.loading.active && self.state.loading.total > 0 {
                    self.state.loading.completed += 1;
                    self.state.loading.progress =
                        self.state.loading.completed as f64 / self.state.loading.total as f64;
                }
            }
            BgResult::ThumbnailBatchDone => {
                if self.state.loading.active
                    && self.state.loading.message.contains("thumbnails")
                {
                    self.state.loading.active = false;
                }
            }
            BgResult::AudioUrlReady(idx, track, res) => {
                match res {
                    Ok(audio_url) => {
                        self.state.queue_index = Some(idx);
                        self.state.playback.current_track = Some(track.clone());
                        self.state.playback.status = PlayStatus::Buffering;
                        self.state.loading.message = "Buffering audio...".into();

                        // Store url for the run loop to pick up
                        // We need to load file into mpv - store pending play
                        self.state.toast_message = Some(format!("Playing: {}", track.title));
                        self.state.toast_timer = 30;

                        // We can't call async ensure_mpv here, so store pending
                        // Instead, use a sync field
                        self.pending_play = Some((track, audio_url));
                    }
                    Err(e) => {
                        self.state.loading.active = false;
                        self.state.playback.status = PlayStatus::Stopped;
                        self.state.toast_message = Some(format!("URL error: {e}"));
                        self.state.toast_timer = 60;
                        fft::set_fft_active(false);
                    }
                }
            }
            BgResult::MetadataReady(res) => {
                if let Ok(full_track) = res {
                    if let Some(ref mut current) = self.state.playback.current_track {
                        current.codec = full_track.codec.or(current.codec.clone());
                        current.bitrate = full_track.bitrate.or(current.bitrate);
                        current.sample_rate = full_track.sample_rate.or(current.sample_rate);
                        current.channels = full_track.channels.or(current.channels);
                        current.filesize = full_track.filesize.or(current.filesize);
                    }
                }
            }
            BgResult::PlaylistTracksReady(res) => {
                self.state.loading.active = false;
                if let Ok(tracks) = res {
                    self.state.search_results = tracks.clone();
                    self.state.queue = tracks;
                    self.state.content_index = 0;
                    self.state.last_preview_index = None;
                    self.spawn_thumbnail_batch();
                }
            }
        }
    }

    async fn handle_action(&mut self, action: AppAction) {
        match action {
            AppAction::None => {}
            AppAction::Quit => {
                self.state.running = false;
            }
            AppAction::Search(query) => {
                self.state.searching = true;
                self.state.loading.active = true;
                self.state.loading.message = format!("Searching \"{query}\"...");
                self.state.loading.progress = -1.0; // indeterminate
                self.state.loading.completed = 0;

                let tx = self.bg_tx.clone();
                tokio::spawn(async move {
                    let result = search::search_youtube(&query, 10).await;
                    let _ = tx.send(BgResult::SearchDone(result)).await;
                });
            }
            AppAction::PlayTrackIndex(idx) => {
                self.play_track_at_index(idx);
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
            AppAction::NextTrack => { self.next_track(); }
            AppAction::PrevTrack => { self.prev_track(); }
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
                self.state.loading.active = true;
                self.state.loading.message = "Loading playlist...".into();
                self.state.loading.progress = -1.0;
                self.state.loading.completed = 0;

                // DB access is sync, do it here, then spawn thumbnail loading
                if let Ok(tracks) = playlists::get_playlist_tracks(&self.db, id) {
                    self.state.search_results = tracks.clone();
                    self.state.queue = tracks;
                    self.state.content_index = 0;
                    self.state.last_preview_index = None;
                    self.state.loading.active = false;
                    self.spawn_thumbnail_batch();
                } else {
                    self.state.loading.active = false;
                }
            }
        }

        // Handle pending play (from AudioUrlReady)
        self.process_pending_play().await;
    }

    /// Spawn thumbnail downloads for current search results in background.
    fn spawn_thumbnail_batch(&mut self) {
        let tracks: Vec<(String, String)> = self.state.search_results.iter()
            .filter(|t| !t.thumbnail_url.is_empty() && !self.thumb_cache.contains_key(&t.youtube_id))
            .map(|t| (t.youtube_id.clone(), t.thumbnail_url.clone()))
            .collect();

        if tracks.is_empty() {
            return;
        }

        self.state.loading.active = true;
        self.state.loading.message = format!("Loading {} thumbnails...", tracks.len());
        self.state.loading.progress = 0.0;
        self.state.loading.total = tracks.len();
        self.state.loading.completed = 0;

        let tx = self.bg_tx.clone();
        tokio::spawn(async move {
            for (youtube_id, thumb_url) in tracks {
                if let Ok(path) = thumbnail::download_thumbnail(&thumb_url, &youtube_id).await {
                    let _ = tx.send(BgResult::ThumbnailReady(youtube_id, path)).await;
                }
            }
            let _ = tx.send(BgResult::ThumbnailBatchDone).await;
        });
    }

    fn play_track_at_index(&mut self, idx: usize) {
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

        self.state.loading.active = true;
        self.state.loading.message = format!("Loading: {}...", track.title);
        self.state.loading.progress = -1.0;
        self.state.loading.completed = 0;

        let tx = self.bg_tx.clone();
        let url = track.youtube_url.clone();
        let track_clone = track;
        tokio::spawn(async move {
            let result = metadata::get_audio_url(&url).await;
            let _ = tx.send(BgResult::AudioUrlReady(idx, track_clone, result)).await;
        });
    }

    async fn process_pending_play(&mut self) {
        if let Some((track, audio_url)) = self.pending_play.take() {
            if let Err(e) = self.ensure_mpv().await {
                self.state.toast_message = Some(format!("mpv error: {e}"));
                self.state.toast_timer = 60;
                self.state.loading.active = false;
                self.state.playback.status = PlayStatus::Stopped;
                return;
            }

            if let Some(ref mut mpv) = self.mpv {
                if let Err(e) = mpv.load_file(&audio_url).await {
                    self.state.toast_message = Some(format!("Play error: {e}"));
                    self.state.toast_timer = 60;
                    self.state.loading.active = false;
                    self.state.playback.status = PlayStatus::Stopped;
                    return;
                }
            }

            self.state.loading.active = false;
            fft::set_fft_active(true);
            self.load_thumbnail_sync(&track);

            // Spawn metadata fetch in background
            let tx = self.bg_tx.clone();
            let url = track.youtube_url.clone();
            tokio::spawn(async move {
                let result = metadata::get_full_metadata(&url).await;
                let _ = tx.send(BgResult::MetadataReady(result)).await;
            });
        }
    }

    fn load_thumbnail_sync(&mut self, track: &Track) {
        // Skip if already decoded for this track
        if self.thumb_protocol_id == track.youtube_id && self.thumb_protocol.is_some() {
            return;
        }

        if track.thumbnail_url.is_empty() {
            self.thumb_protocol = None;
            self.thumb_protocol_id.clear();
            return;
        }

        let path = thumbnail::thumbnail_path(&track.youtube_id);
        if path.exists() {
            if let Some(ref mut picker) = self.image_picker {
                if let Ok(reader) = image::ImageReader::open(&path) {
                    if let Ok(reader) = reader.with_guessed_format() {
                        if let Ok(img) = reader.decode() {
                            let protocol = picker.new_resize_protocol(img);
                            self.thumb_protocol = Some(protocol);
                            self.thumb_protocol_id = track.youtube_id.clone();
                            return;
                        }
                    }
                }
            }
        }
        self.thumb_protocol = None;
        self.thumb_protocol_id.clear();
    }

    fn next_track(&mut self) {
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
        self.play_track_at_index(next_idx);
    }

    fn prev_track(&mut self) {
        if self.state.queue.is_empty() { return; }
        let prev_idx = if let Some(idx) = self.state.queue_index {
            if idx == 0 {
                match self.state.repeat { state::RepeatMode::All => self.state.queue.len() - 1, _ => 0 }
            } else { idx - 1 }
        } else { 0 };
        self.play_track_at_index(prev_idx);
    }

    async fn handle_auto_next(&mut self) {
        match self.state.repeat {
            state::RepeatMode::One => {
                if let Some(idx) = self.state.queue_index { self.play_track_at_index(idx); }
            }
            state::RepeatMode::All => { self.next_track(); }
            state::RepeatMode::Off => {
                if let Some(idx) = self.state.queue_index {
                    if idx + 1 < self.state.queue.len() { self.next_track(); }
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
