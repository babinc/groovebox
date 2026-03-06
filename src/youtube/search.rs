use anyhow::Result;
use tokio::process::Command;

use crate::models::Track;
use super::types::YtDlpResult;

pub async fn search_youtube(query: &str, max_results: usize) -> Result<Vec<Track>> {
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        Command::new("yt-dlp")
            .args([
                "--flat-playlist",
                "--dump-json",
                "--no-warnings",
                "--default-search", "ytsearch",
                &format!("ytsearch{max_results}:{query}"),
            ])
            .output()
    ).await
        .map_err(|_| anyhow::anyhow!("Search timed out"))??;

    Ok(parse_ytdlp_output(&String::from_utf8_lossy(&output.stdout), None))
}

/// Fetch YouTube's recommended/related videos for a given video URL.
pub async fn fetch_related(youtube_url: &str, max_results: usize) -> Result<Vec<Track>> {
    let video_id = youtube_url
        .split("v=").nth(1)
        .and_then(|s| s.split('&').next())
        .unwrap_or("");

    if video_id.is_empty() {
        return Ok(Vec::new());
    }

    let mix_url = format!("https://www.youtube.com/watch?v={video_id}&list=RD{video_id}");

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        Command::new("yt-dlp")
            .args([
                "--flat-playlist",
                "--dump-json",
                "--no-warnings",
                "--playlist-end", &max_results.to_string(),
                &mix_url,
            ])
            .output()
    ).await
        .map_err(|_| anyhow::anyhow!("Related tracks fetch timed out"))??;

    Ok(parse_ytdlp_output(&String::from_utf8_lossy(&output.stdout), Some(video_id)))
}

fn parse_ytdlp_output(stdout: &str, skip_id: Option<&str>) -> Vec<Track> {
    let mut tracks = Vec::new();

    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(result) = serde_json::from_str::<YtDlpResult>(line) {
            let thumbnail_url = result.best_thumbnail();
            let youtube_id = result.id.unwrap_or_default();
            if skip_id.is_some_and(|id| id == youtube_id) {
                continue;
            }
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

    tracks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_json_line() {
        let json = r#"{"id":"abc123","title":"Test Song","uploader":"Artist","duration":180.0,"webpage_url":"https://www.youtube.com/watch?v=abc123","thumbnails":[{"url":"https://example.com/thumb.jpg","height":360,"width":480}]}"#;
        let tracks = parse_ytdlp_output(json, None);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].youtube_id, "abc123");
        assert_eq!(tracks[0].title, "Test Song");
        assert_eq!(tracks[0].artist, "Artist");
        assert_eq!(tracks[0].duration, 180.0);
    }

    #[test]
    fn parse_skips_invalid_json() {
        let input = "not json\n{\"id\":\"x\",\"title\":\"T\",\"uploader\":\"A\",\"duration\":1.0,\"thumbnails\":[]}\ngarbage";
        let tracks = parse_ytdlp_output(input, None);
        assert_eq!(tracks.len(), 1);
    }

    #[test]
    fn parse_skips_matching_id() {
        let json = r#"{"id":"skip_me","title":"T","uploader":"A","duration":1.0,"thumbnails":[]}"#;
        let tracks = parse_ytdlp_output(json, Some("skip_me"));
        assert!(tracks.is_empty());
    }

    #[test]
    fn parse_missing_fields_use_defaults() {
        let json = r#"{"thumbnails":[]}"#;
        let tracks = parse_ytdlp_output(json, None);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].title, "Unknown");
        assert_eq!(tracks[0].artist, "Unknown");
        assert_eq!(tracks[0].duration, 0.0);
    }

    #[test]
    fn parse_prefers_uploader_over_channel() {
        let json = r#"{"id":"x","title":"T","uploader":"Uploader","channel":"Channel","duration":1.0,"thumbnails":[]}"#;
        let tracks = parse_ytdlp_output(json, None);
        assert_eq!(tracks[0].artist, "Uploader");
    }

    #[test]
    fn parse_falls_back_to_channel() {
        let json = r#"{"id":"x","title":"T","channel":"Channel","duration":1.0,"thumbnails":[]}"#;
        let tracks = parse_ytdlp_output(json, None);
        assert_eq!(tracks[0].artist, "Channel");
    }

    #[test]
    fn parse_empty_input() {
        let tracks = parse_ytdlp_output("", None);
        assert!(tracks.is_empty());
    }

    #[test]
    fn parse_multiple_lines() {
        let input = "{\"id\":\"a\",\"title\":\"A\",\"uploader\":\"X\",\"duration\":1.0,\"thumbnails\":[]}\n\
                     {\"id\":\"b\",\"title\":\"B\",\"uploader\":\"Y\",\"duration\":2.0,\"thumbnails\":[]}";
        let tracks = parse_ytdlp_output(input, None);
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].youtube_id, "a");
        assert_eq!(tracks[1].youtube_id, "b");
    }
}
