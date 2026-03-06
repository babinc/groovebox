use rusqlite::{Connection, Row};

use crate::models::Track;

const COLUMNS: &[&str] = &[
    "id", "youtube_id", "title", "artist", "duration", "thumbnail_url", "youtube_url",
    "codec", "bitrate", "sample_rate", "channels", "filesize", "description",
];

pub fn track_columns(prefix: &str) -> String {
    COLUMNS.iter()
        .map(|c| if prefix.is_empty() { c.to_string() } else { format!("{prefix}.{c}") })
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn track_from_row(row: &Row) -> rusqlite::Result<Track> {
    Ok(Track {
        id: Some(row.get(0)?),
        youtube_id: row.get(1)?,
        title: row.get(2)?,
        artist: row.get(3)?,
        duration: row.get(4)?,
        thumbnail_url: row.get(5)?,
        youtube_url: row.get(6)?,
        codec: row.get(7)?,
        bitrate: row.get(8)?,
        sample_rate: row.get(9)?,
        channels: row.get(10)?,
        filesize: row.get(11)?,
        description: row.get(12)?,
    })
}

pub fn ensure_track(conn: &Connection, track: &Track) -> anyhow::Result<i64> {
    let insert_cols: Vec<_> = COLUMNS.iter().filter(|c| **c != "id").copied().collect();
    let placeholders: Vec<_> = (1..=insert_cols.len()).map(|i| format!("?{i}")).collect();

    conn.execute(
        &format!(
            "INSERT OR IGNORE INTO tracks ({}) VALUES ({})",
            insert_cols.join(", "),
            placeholders.join(", ")
        ),
        rusqlite::params![
            track.youtube_id, track.title, track.artist, track.duration,
            track.thumbnail_url, track.youtube_url, track.codec, track.bitrate,
            track.sample_rate, track.channels, track.filesize, track.description
        ],
    )?;

    let track_id: i64 = conn.query_row(
        "SELECT id FROM tracks WHERE youtube_id = ?1",
        [&track.youtube_id],
        |row| row.get(0),
    )?;

    Ok(track_id)
}
