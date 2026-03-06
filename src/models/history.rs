use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayHistoryEntry {
    pub id: Option<i64>,
    pub track_id: i64,
    pub played_at: DateTime<Utc>,
    pub duration_listened: f64,
    pub completed: bool,
    // Denormalized for display
    pub track_title: Option<String>,
    pub track_artist: Option<String>,
}
