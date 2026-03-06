pub mod events;
pub mod state;

use std::collections::HashMap;
use std::io;
use std::io::Write as _;
use std::path::PathBuf;
use std::time::Duration;

#[cfg(debug_assertions)]
fn log(msg: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true).append(true)
        .open("/tmp/groovebox.log")
    {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let _ = writeln!(f, "[{:.3}] {msg}", now.as_secs_f64());
    }
}

#[cfg(not(debug_assertions))]
fn log(_msg: &str) {}

use anyhow::Result;
use crossterm::event::{self, Event};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::{mpsc, watch};

use crate::audio::fft;
use crate::audio::mpv::MpvPlayer;
use crate::audio::types::{PlayStatus, PlaybackState, SpectrumData};
use crate::models::Track;
use crate::storage::{database, history, playlists, settings, tracks};
use crate::ui::theme;
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
    RelatedReady(Result<Vec<Track>>),
    HiResThumbnailReady(String), // youtube_id
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
    auto_resume: Option<(Track, f64)>, // track + position to resume on startup
    resume_seek: Option<f64>,         // seek to this position once playback starts
}

impl App {
    pub async fn new() -> Result<Self> {
        // Clean up old cached thumbnails (older than 7 days)
        thumbnail::cleanup_cache(std::time::Duration::from_secs(7 * 24 * 3600));

        let db = database::open_database()?;

        let playlist_list = playlists::list_playlists(&db).unwrap_or_default();

        // Load saved theme
        if let Some(theme_idx) = settings::get_parsed::<usize>(&db, "theme_index") {
            theme::set_theme(theme_idx);
        }

        // Load saved EQ style
        let eq_style = settings::get_parsed::<u8>(&db, "eq_style")
            .and_then(|i| state::EqStyle::ALL.get(i as usize).copied())
            .unwrap_or(state::EqStyle::Bars);

        // Load preferences
        let mut prefs = state::Preferences::default();
        for &(key, _) in state::Preferences::KEYS {
            if let Some(val) = settings::get_setting(&db, key).ok().flatten() {
                match val.as_str() {
                    "0" => { if prefs.get(key) { prefs.toggle(key); } }
                    "1" => { if !prefs.get(key) { prefs.toggle(key); } }
                    _ => {}
                }
            }
        }

        // Restore last session track
        let last_track_id_raw = settings::get_setting(&db, "last_track_id").ok().flatten();
        log(&format!("SESSION: last_track_id from DB = {:?}", last_track_id_raw));
        let last_track = last_track_id_raw
            .and_then(|yt_id| {
                let result = tracks::get_track_by_youtube_id(&db, &yt_id);
                log(&format!("SESSION: DB lookup for '{yt_id}' = {:?}", result.as_ref().map(|t| &t.title)));
                result.ok()
            });
        let last_position = settings::get_parsed::<f64>(&db, "last_position").unwrap_or(0.0);
        log(&format!("SESSION: last_position = {last_position:.1}s, has_track = {}", last_track.is_some()));

        // Load saved playback preferences
        let last_volume = settings::get_parsed::<f64>(&db, "volume").unwrap_or(100.0);
        let last_shuffle = settings::get_parsed::<u8>(&db, "shuffle").unwrap_or(0) == 1;
        let last_repeat = match settings::get_parsed::<u8>(&db, "repeat").unwrap_or(0) {
            1 => state::RepeatMode::One,
            2 => state::RepeatMode::All,
            _ => state::RepeatMode::Off,
        };

        let mut state = AppState::default();
        state.playlists = playlist_list;
        state.eq_style = eq_style;
        state.preferences = prefs;
        state.playback.volume = last_volume;
        state.shuffle = last_shuffle;
        state.repeat = last_repeat;

        // If we have a last track, put it in the queue ready to play
        if let Some(ref track) = last_track {
            log(&format!("SESSION: restoring '{}' by {} ({})", track.title, track.artist, track.youtube_id));
            state.search_results = vec![track.clone()];
            state.queue = vec![track.clone()];
            state.preview_track = Some(track.clone());
            state.content_view = state::ContentView::SearchResults;
        }

        let (spectrum_tx, spectrum_rx) = watch::channel(SpectrumData::default());
        fft::spawn_fft_task(spectrum_tx);

        let image_picker = ratatui_image::picker::Picker::from_query_stdio()
            .or_else(|_| {
                let mut picker = ratatui_image::picker::Picker::from_fontsize((8, 16));
                picker.set_protocol_type(ratatui_image::picker::ProtocolType::Halfblocks);
                Ok::<_, std::io::Error>(picker)
            })
            .ok();
        log(&format!("IMAGE_PICKER: {:?}", image_picker.as_ref().map(|p| p.protocol_type())));

        let (bg_tx, bg_rx) = mpsc::channel(64);

        let auto_resume = if state.preferences.auto_resume {
            last_track.map(|t| (t, last_position))
        } else {
            None
        };

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
            resume_seek: None,
            auto_resume,
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
        // Load thumbnails for any restored session tracks
        if !self.state.search_results.is_empty() {
            self.spawn_thumbnail_batch();
        }

        // Auto-resume playback from last session
        if let Some((ref track, position)) = self.auto_resume.take() {
            log(&format!("RESUME: playing '{}' at queue[0], seeking to {position:.1}s", track.title));
            // Pre-fetch related tracks using artist+title search (more relevant than YouTube Mix)
            // Strip emojis and special chars so yt-dlp search actually finds results
            let query: String = format!("{} {}", track.artist, track.title)
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-' || *c == '&')
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<&str>>()
                .join(" ");
            let tx = self.bg_tx.clone();
            let url = track.youtube_url.clone();
            self.state.fetching_related = true;
            tokio::spawn(async move {
                // Try search first, fall back to YouTube Mix if no results
                let result = search::search_youtube(&query, 20).await;
                if result.as_ref().map_or(true, |t| t.is_empty()) {
                    let fallback = search::fetch_related(&url, 20).await;
                    let _ = tx.send(BgResult::RelatedReady(fallback)).await;
                } else {
                    let _ = tx.send(BgResult::RelatedReady(result)).await;
                }
            });
            self.play_track_at_index(0);
            self.resume_seek = if position > 5.0 { Some(position - 2.0) } else { None };
        }

        let mut preview_debounce: u8 = 0;

        while self.state.running {
            // 1. Handle ALL pending input first (drain the queue so navigation feels instant)
            let mut had_input = false;
            while event::poll(Duration::from_millis(0))? {
                if let Event::Key(key) = event::read()? {
                    let action = events::handle_key(&mut self.state, key);
                    match &action {
                        AppAction::None => {}
                        other => log(&format!("ACTION: {other:?} content_index={} queue_index={:?}", self.state.content_index, self.state.queue_index)),
                    }
                    self.handle_action(action).await;
                    had_input = true;
                }
            }

            // 1b. If more input is already pending, skip expensive draw and keep draining
            if had_input && event::poll(Duration::from_millis(0))? {
                continue;
            }

            // 2. Drain channel updates (non-blocking)
            if let Some(ref mut rx) = self.playback_rx {
                let is_loading = self.state.loading.active || self.pending_play.is_some();
                while let Ok(pb_state) = rx.try_recv() {
                    let track = self.state.playback.current_track.clone();
                    let volume = self.state.playback.volume;
                    let prev_status = self.state.playback.status;
                    // Don't let mpv's end-file event overwrite Buffering with Stopped
                    // during loading — that would trigger a false auto-next
                    let ignore_stopped = is_loading
                        && pb_state.status == PlayStatus::Stopped;
                    self.state.playback = pb_state;
                    self.state.playback.current_track = track;
                    if ignore_stopped {
                        self.state.playback.status = PlayStatus::Buffering;
                    }
                    // Only log status transitions, not every frame
                    if self.state.playback.status != prev_status {
                        if ignore_stopped {
                            log("MPV: ignoring Stopped (is_loading=true), keeping Buffering");
                        } else {
                            log(&format!("MPV: status -> {:?}", self.state.playback.status));
                        }
                    }
                    if (self.state.playback.volume - volume).abs() > 0.5 {
                        self.state.playback.volume = volume;
                    }
                }
            }

            // Clear loading state once mpv actually starts playing
            if self.state.playback.status == PlayStatus::Playing
                && self.state.loading.active
                && self.state.loading.kind == state::LoadingKind::Buffering
            {
                self.state.loading.active = false;
                // Apply resume seek if pending
                if let Some(pos) = self.resume_seek.take() {
                    if let Some(ref mut mpv) = self.mpv {
                        let _ = mpv.seek_absolute(pos).await;
                    }
                }
            }

            while let Ok(result) = self.bg_rx.try_recv() {
                self.handle_bg_result(result);
            }

            self.process_pending_play().await;

            // 3. Update spectrum + peak decay
            if self.spectrum_rx.has_changed().unwrap_or(false) {
                self.state.spectrum = self.spectrum_rx.borrow_and_update().clone();
                // Update peak hold values (for Peaks style)
                for (i, &val) in self.state.spectrum.bins.iter().enumerate() {
                    if i < self.state.eq_peaks.len() {
                        if val > self.state.eq_peaks[i] {
                            self.state.eq_peaks[i] = val;
                        } else {
                            // Fast decay, snap to zero when tiny
                            let decayed = self.state.eq_peaks[i] * 0.92;
                            self.state.eq_peaks[i] = if decayed < 0.05 { 0.0 } else { decayed };
                        }
                    }
                }
            }

            // 4. Toast timer
            if self.state.toast_timer > 0 {
                self.state.toast_timer -= 1;
                if self.state.toast_timer == 0 {
                    self.state.toast_message = None;
                }
            }

            // Theme selector timer
            if self.state.theme_selector_timer > 0 {
                self.state.theme_selector_timer -= 1;
            }
            if self.state.eq_selector_timer > 0 {
                self.state.eq_selector_timer -= 1;
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
            //    Only when truly idle — not during loading or pending play
            if self.state.playback.status == PlayStatus::Stopped
                && self.state.queue_index.is_some()
                && !self.state.loading.active
                && self.pending_play.is_none()
            {
                log(&format!("AUTO-NEXT: triggered, queue_index={:?}, queue_len={}, current_track={:?}",
                    self.state.queue_index, self.state.queue.len(),
                    self.state.playback.current_track.as_ref().map(|t| &t.title)));
                self.handle_auto_next().await;
            }

            // 8. Draw
            self.state.frame_count = self.state.frame_count.wrapping_add(1);
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
        // Save last track + position for session restore
        if let Some(ref track) = self.state.playback.current_track {
            log(&format!("SESSION SAVE: '{}' ({}) at {:.1}s", track.title, track.youtube_id, self.state.playback.position));
            let _ = settings::set_setting(&self.db, "last_track_id", &track.youtube_id);
            let _ = settings::set_setting(&self.db, "last_position", &self.state.playback.position.to_string());
        } else {
            log("SESSION SAVE: no current track to save");
        }
        let _ = settings::set_setting(&self.db, "volume", &self.state.playback.volume.to_string());
        let _ = settings::set_setting(&self.db, "shuffle", if self.state.shuffle { "1" } else { "0" });
        let _ = settings::set_setting(&self.db, "repeat", &(self.state.repeat as u8).to_string());
        fft::set_fft_active(false);

        // Kill mpv process
        if let Some(ref mut mpv) = self.mpv {
            let _ = mpv.quit().await;
        }

        Ok(())
    }

    fn handle_bg_result(&mut self, result: BgResult) {
        match result {
            BgResult::SearchDone(res) => {
                self.state.searching = false;
                self.state.loading.active = false;
                match res {
                    Ok(results) => {
                        self.replace_queue(results);
                    }
                    Err(e) => {
                        let msg = format!("Search error: {e}");
                        log(&format!("TOAST: {msg}"));
                        self.state.toast_message = Some(msg);
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
                    && self.state.loading.kind == state::LoadingKind::Thumbnails
                {
                    self.state.loading.active = false;
                }
            }
            BgResult::AudioUrlReady(idx, track, res) => {
                log(&format!("AUDIO_URL: idx={idx} track='{}' success={}", track.title, res.is_ok()));
                match res {
                    Ok(audio_url) => {
                        self.state.queue_index = Some(idx);
                        self.state.playback.current_track = Some(track.clone());
                        self.state.playback.status = PlayStatus::Buffering;
                        self.state.loading.kind = state::LoadingKind::Buffering;
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
                        log(&format!("AUDIO_URL: FAILED for '{}': {e}", track.title));
                        self.state.loading.active = false;
                        self.state.playback.status = PlayStatus::Stopped;
                        self.state.queue_index = None; // prevent auto-next from advancing
                        let msg = format!("URL error: {e}");
                        log(&format!("TOAST: {msg}"));
                        self.state.toast_message = Some(msg);
                        self.state.toast_timer = 60;
                        fft::set_fft_active(false);
                    }
                }
            }
            BgResult::MetadataReady(res) => {
                if let Ok(full_track) = res {
                    if let Some(ref mut current) = self.state.playback.current_track {
                        if full_track.codec.is_some() { current.codec = full_track.codec; }
                        if full_track.bitrate.is_some() { current.bitrate = full_track.bitrate; }
                        if full_track.sample_rate.is_some() { current.sample_rate = full_track.sample_rate; }
                        if full_track.channels.is_some() { current.channels = full_track.channels; }
                        if full_track.filesize.is_some() { current.filesize = full_track.filesize; }
                        if full_track.description.is_some() {
                            // Cache parsed chapters when description arrives
                            self.state.cached_chapters = full_track.description.as_deref()
                                .map(crate::youtube::chapters::parse_chapters)
                                .unwrap_or_default();
                            self.state.cached_chapters_track_id = current.youtube_id.clone();
                            current.description = full_track.description;
                        }
                    }
                }
            }
            BgResult::PlaylistTracksReady(res) => {
                self.state.loading.active = false;
                if let Ok(tracks) = res {
                    self.replace_queue(tracks);
                }
            }
            BgResult::RelatedReady(res) => {
                self.state.fetching_related = false;
                log(&format!("RELATED: received {} tracks", res.as_ref().map(|t| t.len()).unwrap_or(0)));
                if let Ok(tracks) = res {
                    if !tracks.is_empty() {
                        let mut existing_ids: std::collections::HashSet<String> = self.state.queue
                            .iter().map(|t| t.youtube_id.clone()).collect();
                        existing_ids.extend(self.state.search_results.iter().map(|t| t.youtube_id.clone()));
                        let new_tracks: Vec<Track> = tracks.into_iter()
                            .filter(|t| !existing_ids.contains(&t.youtube_id))
                            .collect();
                        self.state.search_results.extend(new_tracks.clone());
                        self.state.queue.extend(new_tracks);
                        self.spawn_thumbnail_batch();
                    }
                }
            }
            BgResult::HiResThumbnailReady(yt_id) => {
                log(&format!("THUMB: hi-res ready for {yt_id}, current_protocol_id={}", self.thumb_protocol_id));
                if self.thumb_protocol_id == yt_id {
                    if let Some(ref current) = self.state.playback.current_track {
                        let track = current.clone();
                        self.thumb_protocol_id.clear(); // force reload
                        self.load_thumbnail_sync(&track);
                    }
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
                self.state.loading.kind = state::LoadingKind::Search;
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
                log(&format!("TOGGLE_PAUSE: status={:?} has_mpv={}", self.state.playback.status, self.mpv.is_some()));
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
                        log(&format!("TOAST: Playlist error: {e}"));
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
                // Single JOIN query for full Track objects (no N+1)
                let tracks = history::get_history_tracks(&self.db, 50).unwrap_or_default();
                self.replace_queue(tracks);
            }
            AppAction::CycleEq => {
                self.state.eq_style = self.state.eq_style.next();
                self.state.toast_message = Some(format!("EQ: {}", self.state.eq_style.label()));
                self.state.toast_timer = 30;
                self.state.eq_selector_timer = 40;
                let _ = settings::set_setting(&self.db, "eq_style", &(self.state.eq_style as u8).to_string());
            }
            AppAction::CycleTheme => {
                theme::cycle_theme();
                self.state.theme_selector_timer = 40;
                let _ = settings::set_setting(&self.db, "theme_index", &theme::current_index().to_string());
            }
            AppAction::ToggleSetting(idx) => {
                if let Some(&(key, label)) = state::Preferences::KEYS.get(idx) {
                    self.state.preferences.toggle(key);
                    let val = self.state.preferences.get(key);
                    let _ = settings::set_setting(&self.db, key, if val { "1" } else { "0" });
                    self.state.toast_message = Some(format!("{}: {}", label, if val { "On" } else { "Off" }));
                    self.state.toast_timer = 30;
                }
            }
            AppAction::LoadPlaylistTracks(id) => {
                self.state.loading.active = true;
                self.state.loading.kind = state::LoadingKind::Playlist;
                self.state.loading.message = "Loading playlist...".into();
                self.state.loading.progress = -1.0;
                self.state.loading.completed = 0;

                // DB access is sync, do it here, then spawn thumbnail loading
                if let Ok(tracks) = playlists::get_playlist_tracks(&self.db, id) {
                    self.state.loading.active = false;
                    self.replace_queue(tracks);
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
        self.state.loading.kind = state::LoadingKind::Thumbnails;
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
        } else {
            return;
        };

        self.record_current_play();

        self.state.queue_index = Some(idx);
        self.state.playback.current_track = Some(track.clone());
        self.state.playback.status = PlayStatus::Buffering;

        self.state.loading.active = true;
        self.state.loading.kind = state::LoadingKind::Buffering;
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

        // Auto-fetch YouTube recommendations when queue is small
        if self.state.queue.len() <= 1 && !self.state.fetching_related {
            self.state.fetching_related = true;
            let tx = self.bg_tx.clone();
            let url = self.state.queue[idx].youtube_url.clone();
            tokio::spawn(async move {
                let result = search::fetch_related(&url, 20).await;
                let _ = tx.send(BgResult::RelatedReady(result)).await;
            });
        }
    }

    async fn process_pending_play(&mut self) {
        if let Some((track, audio_url)) = self.pending_play.take() {
            if let Err(e) = self.ensure_mpv().await {
                let msg = format!("mpv error: {e}");
                log(&format!("TOAST: {msg}"));
                self.state.toast_message = Some(msg);
                self.state.toast_timer = 60;
                self.state.loading.active = false;
                self.state.playback.status = PlayStatus::Stopped;
                return;
            }

            if let Some(ref mut mpv) = self.mpv {
                if let Err(e) = mpv.load_file(&audio_url).await {
                    let msg = format!("Play error: {e}");
                    log(&format!("TOAST: {msg}"));
                    self.state.toast_message = Some(msg);
                    self.state.toast_timer = 60;
                    self.state.loading.active = false;
                    self.state.playback.status = PlayStatus::Stopped;
                    return;
                }
            }

            // Keep loading active until mpv reports Playing — this prevents
            // stale end-file events from triggering auto-next
            self.state.loading.active = true;
            self.state.loading.kind = state::LoadingKind::Buffering;
            fft::set_fft_active(true);
            self.load_thumbnail_sync(&track);

            // Spawn metadata fetch in background
            let tx = self.bg_tx.clone();
            let url = track.youtube_url.clone();
            tokio::spawn(async move {
                let result = metadata::get_full_metadata(&url).await;
                let _ = tx.send(BgResult::MetadataReady(result)).await;
            });

            // Fetch hi-res thumbnail for now-playing panel
            let hires_path = thumbnail::hires_thumbnail_path(&track.youtube_id);
            if !hires_path.exists() {
                let tx = self.bg_tx.clone();
                let yt_id = track.youtube_id.clone();
                tokio::spawn(async move {
                    if thumbnail::download_hires_thumbnail(&yt_id).await.is_ok() {
                        let _ = tx.send(BgResult::HiResThumbnailReady(yt_id)).await;
                    }
                });
            }
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

        // Prefer hi-res thumbnail, fall back to standard
        let hires_path = thumbnail::hires_thumbnail_path(&track.youtube_id);
        let std_path = thumbnail::thumbnail_path(&track.youtube_id);
        let path = if hires_path.exists() { hires_path } else { std_path };
        log(&format!("THUMB: loading {} using={}", track.youtube_id, path.display()));
        if path.exists() {
            if let Some(ref mut picker) = self.image_picker {
                match image::ImageReader::open(&path)
                    .and_then(|r| r.with_guessed_format())
                    .map_err(|e| e.to_string())
                    .and_then(|r| r.decode().map_err(|e| e.to_string()))
                {
                    Ok(img) => {
                        log(&format!("THUMB: decoded {}x{} for {}", img.width(), img.height(), track.youtube_id));
                        // Downscale large images for terminal protocol compatibility
                        let img = if img.width() > 640 || img.height() > 360 {
                            img.resize(640, 360, image::imageops::FilterType::Triangle)
                        } else {
                            img
                        };
                        let protocol = picker.new_resize_protocol(img);
                        self.thumb_protocol = Some(protocol);
                        self.thumb_protocol_id = track.youtube_id.clone();
                        return;
                    }
                    Err(e) => {
                        log(&format!("THUMB: decode FAILED for {}: {e}", track.youtube_id));
                    }
                }
            } else {
                log("THUMB: no image_picker available");
            }
        } else {
            log(&format!("THUMB: no file found for {}", track.youtube_id));
        }
        self.thumb_protocol = None;
        self.thumb_protocol_id.clear();
    }

    /// Replace the queue and search results with new tracks, resetting navigation state.
    fn replace_queue(&mut self, tracks: Vec<Track>) {
        self.state.search_results = tracks.clone();
        self.state.queue = tracks;
        self.state.content_index = 0;
        self.state.last_preview_index = None;
        if self.state.playback.status == PlayStatus::Stopped {
            self.state.queue_index = None;
        }
        self.spawn_thumbnail_batch();
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
