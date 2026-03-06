use anyhow::Result;
use std::path::PathBuf;

fn cache_dir() -> PathBuf {
    let base = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join("groovebox").join("thumbs")
}

pub fn thumbnail_path(youtube_id: &str) -> PathBuf {
    cache_dir().join(format!("{youtube_id}.jpg"))
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
