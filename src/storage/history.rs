use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;

use crate::models::{PlayHistoryEntry, Track};
use super::tracks::{self, track_from_row};

pub fn record_play(conn: &Connection, track: &Track, duration_listened: f64, completed: bool) -> Result<()> {
    let track_id = tracks::ensure_track(conn, track)?;

    conn.execute(
        "INSERT INTO play_history (track_id, played_at, duration_listened, completed)
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![track_id, Utc::now().to_rfc3339(), duration_listened, completed as i32],
    )?;

    Ok(())
}

pub fn get_history(conn: &Connection, limit: usize) -> Result<Vec<PlayHistoryEntry>> {
    let mut stmt = conn.prepare(
        "SELECT h.id, h.track_id, h.played_at, h.duration_listened, h.completed, t.title, t.artist
         FROM play_history h
         JOIN tracks t ON t.id = h.track_id
         ORDER BY h.played_at DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit as i64], |row| {
        let played_at_str: String = row.get(2)?;
        let played_at = chrono::DateTime::parse_from_rfc3339(&played_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        Ok(PlayHistoryEntry {
            id: Some(row.get(0)?),
            track_id: row.get(1)?,
            played_at,
            duration_listened: row.get(3)?,
            completed: row.get::<_, i32>(4)? != 0,
            track_title: Some(row.get(5)?),
            track_artist: Some(row.get(6)?),
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_track_by_id(conn: &Connection, track_id: i64) -> Result<Track> {
    let track = conn.query_row(
        &format!("SELECT {} FROM tracks WHERE id = ?1", tracks::track_columns("")),
        [track_id],
        |row| track_from_row(row),
    )?;
    Ok(track)
}

/// Get full Track objects for recent history in a single query (no N+1).
pub fn get_history_tracks(conn: &Connection, limit: usize) -> Result<Vec<Track>> {
    let mut stmt = conn.prepare(
        &format!(
            "SELECT {} FROM tracks t
             JOIN play_history h ON t.id = h.track_id
             ORDER BY h.played_at DESC
             LIMIT ?1",
            tracks::track_columns("t")
        ),
    )?;
    let rows = stmt.query_map([limit as i64], |row| track_from_row(row))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}
