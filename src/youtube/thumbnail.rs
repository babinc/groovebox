use anyhow::Result;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

fn cache_dir() -> PathBuf {
    let base = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join("groovebox").join("thumbs")
}

pub fn thumbnail_path(youtube_id: &str) -> PathBuf {
    cache_dir().join(format!("{youtube_id}.jpg"))
}

pub fn hires_thumbnail_path(youtube_id: &str) -> PathBuf {
    cache_dir().join(format!("{youtube_id}_hires.jpg"))
}

pub async fn download_thumbnail(url: &str, youtube_id: &str) -> Result<PathBuf> {
    let path = thumbnail_path(youtube_id);
    if path.exists() {
        return Ok(path);
    }

    std::fs::create_dir_all(cache_dir())?;

    let bytes = reqwest::get(url).await?.bytes().await?;
    std::fs::write(&path, &bytes)?;

    Ok(path)
}

/// Download the highest resolution thumbnail available for a video.
/// Tries maxresdefault (1280x720), falls back to sddefault (640x480),
/// then hqdefault (480x360).
pub async fn download_hires_thumbnail(youtube_id: &str) -> Result<PathBuf> {
    let path = hires_thumbnail_path(youtube_id);
    if path.exists() {
        return Ok(path);
    }

    std::fs::create_dir_all(cache_dir())?;

    let candidates = [
        format!("https://i.ytimg.com/vi/{youtube_id}/maxresdefault.jpg"),
        format!("https://i.ytimg.com/vi/{youtube_id}/sddefault.jpg"),
        format!("https://i.ytimg.com/vi/{youtube_id}/hqdefault.jpg"),
    ];

    for url in &candidates {
        if let Ok(resp) = reqwest::get(url).await {
            if resp.status().is_success() {
                let bytes = resp.bytes().await?;
                // YouTube returns a tiny placeholder for missing resolutions
                if bytes.len() > 2000 {
                    std::fs::write(&path, &bytes)?;
                    return Ok(path);
                }
            }
        }
    }

    anyhow::bail!("no hi-res thumbnail available")
}

/// Delete cached thumbnails older than `max_age`. Called once on startup.
pub fn cleanup_cache(max_age: Duration) {
    let dir = cache_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else { return };
    let cutoff = SystemTime::now() - max_age;

    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else { continue };
        let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        if modified < cutoff {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}
