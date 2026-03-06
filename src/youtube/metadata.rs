use anyhow::Result;
use tokio::process::Command;

use crate::models::Track;
use super::types::YtDlpResult;

#[cfg(debug_assertions)]
fn log_to_file(msg: &str) {
    use std::io::Write;
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
fn log_to_file(_msg: &str) {}

pub async fn get_audio_url(youtube_url: &str) -> Result<String> {
    // Retry once on failure — yt-dlp can transiently fail (rate limits, network)
    for attempt in 0..2 {
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
        if !url.is_empty() {
            return Ok(url);
        }

        if attempt == 0 {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log_to_file(&format!("yt-dlp attempt 1 failed (status={}): {}", output.status, stderr.trim()));
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("yt-dlp failed: {}", stderr.trim());
        }
    }
    unreachable!()
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
