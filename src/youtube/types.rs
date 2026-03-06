use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ThumbnailEntry {
    pub url: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct YtDlpResult {
    pub id: Option<String>,
    pub title: Option<String>,
    pub uploader: Option<String>,
    pub channel: Option<String>,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
    pub thumbnails: Option<Vec<ThumbnailEntry>>,
    pub url: Option<String>,
    pub webpage_url: Option<String>,
    pub acodec: Option<String>,
    pub abr: Option<f64>,
    pub asr: Option<f64>,
    pub audio_channels: Option<u32>,
    pub filesize: Option<u64>,
    pub filesize_approx: Option<u64>,
    pub description: Option<String>,
}

impl YtDlpResult {
    /// Get the best thumbnail URL — prefer `thumbnail`, fall back to last entry in `thumbnails`
    pub fn best_thumbnail(&self) -> String {
        if let Some(ref url) = self.thumbnail {
            if !url.is_empty() {
                return url.clone();
            }
        }
        if let Some(ref thumbs) = self.thumbnails {
            // Last entry is typically highest resolution
            if let Some(entry) = thumbs.last() {
                if let Some(ref url) = entry.url {
                    return url.clone();
                }
            }
        }
        String::new()
    }
}
