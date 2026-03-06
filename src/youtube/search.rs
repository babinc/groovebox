use anyhow::Result;
use tokio::process::Command;

use crate::models::Track;
use super::types::YtDlpResult;

pub async fn search_youtube(query: &str, max_results: usize) -> Result<Vec<Track>> {
    let output = Command::new("yt-dlp")
        .args([
            "--flat-playlist",
            "--dump-json",
            "--no-warnings",
            "--default-search", "ytsearch",
            &format!("ytsearch{max_results}:{query}"),
        ])
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut tracks = Vec::new();

    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(result) = serde_json::from_str::<YtDlpResult>(line) {
            let thumbnail_url = result.best_thumbnail();
            let youtube_id = result.id.unwrap_or_default();
            let title = result.title.unwrap_or_else(|| "Unknown".into());
            let artist = result.uploader
                .or(result.channel)
                .unwrap_or_else(|| "Unknown".into());
            let duration = result.duration.unwrap_or(0.0);
            let youtube_url = result.webpage_url
                .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={youtube_id}"));

            tracks.push(Track {
                id: None,
                youtube_id,
                title,
                artist,
                duration,
                thumbnail_url,
                youtube_url,
                codec: None,
                bitrate: None,
                sample_rate: None,
                channels: None,
                filesize: None,
                description: result.description,
            });
        }
    }

    Ok(tracks)
}
