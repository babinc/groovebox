use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: Option<i64>,
    pub youtube_id: String,
    pub title: String,
    pub artist: String,
    pub duration: f64,
    pub thumbnail_url: String,
    pub youtube_url: String,
    pub codec: Option<String>,
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub filesize: Option<u64>,
    pub description: Option<String>,
}

impl Track {
    pub fn duration_display(&self) -> String {
        let mins = self.duration as u64 / 60;
        let secs = self.duration as u64 % 60;
        format!("{mins}:{secs:02}")
    }
}
