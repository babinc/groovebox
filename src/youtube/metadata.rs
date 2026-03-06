use anyhow::Result;
use tokio::process::Command;

use crate::models::Track;
use super::types::YtDlpResult;

pub async fn get_audio_url(youtube_url: &str) -> Result<String> {
    let output = Command::new("yt-dlp")
        .args([
            "-f", "bestaudio",
            "--get-url",
            "--no-warnings",
            youtube_url,
        ])
        .output()
        .await?;

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if url.is_empty() {
        anyhow::bail!("Failed to get audio URL for {youtube_url}");
    }
    Ok(url)
}

pub async fn get_full_metadata(youtube_url: &str) -> Result<Track> {
    let output = Command::new("yt-dlp")
        .args([
            "-f", "bestaudio",
            "-j",
            "--no-warnings",
            youtube_url,
        ])
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: YtDlpResult = serde_json::from_str(stdout.trim())?;

    let thumbnail_url = result.best_thumbnail();
    let youtube_id = result.id.unwrap_or_default();
    Ok(Track {
        id: None,
        youtube_id: youtube_id.clone(),
        title: result.title.unwrap_or_else(|| "Unknown".into()),
        artist: result.uploader.or(result.channel).unwrap_or_else(|| "Unknown".into()),
        duration: result.duration.unwrap_or(0.0),
        thumbnail_url,
        youtube_url: result.webpage_url
            .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={youtube_id}")),
        codec: result.acodec,
        bitrate: result.abr.map(|b| b as u32),
        sample_rate: result.asr.map(|s| s as u32),
        channels: result.audio_channels,
        filesize: result.filesize.or(result.filesize_approx),
        description: result.description,
    })
}
