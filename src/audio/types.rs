use crate::models::Track;

#[derive(Debug, Clone, PartialEq)]
pub enum PlayStatus {
    Stopped,
    Playing,
    Paused,
    Buffering,
}

#[derive(Debug, Clone)]
pub struct PlaybackState {
    pub status: PlayStatus,
    pub position: f64,
    pub duration: f64,
    pub volume: f64,
    pub current_track: Option<Track>,
    pub codec: Option<String>,
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            status: PlayStatus::Stopped,
            position: 0.0,
            duration: 0.0,
            volume: 100.0,
            current_track: None,
            codec: None,
            bitrate: None,
            sample_rate: None,
            channels: None,
        }
    }
}

#[derive(Debug)]
pub enum PlayerCommand {
    Play(Track),
    Pause,
    Resume,
    Stop,
    Seek(f64),
    Volume(f64),
}

#[derive(Debug, Clone)]
pub struct SpectrumData {
    pub bins: Vec<f32>,
}

impl Default for SpectrumData {
    fn default() -> Self {
        Self {
            bins: vec![0.0; 32],
        }
    }
}
