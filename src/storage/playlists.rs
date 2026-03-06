use anyhow::Result;
use rusqlite::Connection;

use crate::models::{Playlist, Track};
use super::tracks::{self, track_from_row};

pub fn list_playlists(conn: &Connection) -> Result<Vec<Playlist>> {
    let mut stmt = conn.prepare("SELECT id, name, description, category_id FROM playlists ORDER BY name")?;
    let rows = stmt.query_map([], |row| {
        Ok(Playlist {
            id: Some(row.get(0)?),
            name: row.get(1)?,
            description: row.get(2)?,
            category_id: row.get(3)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn create_playlist(conn: &Connection, name: &str, description: &str, category_id: Option<i64>) -> Result<i64> {
    conn.execute(
        "INSERT INTO playlists (name, description, category_id) VALUES (?1, ?2, ?3)",
        rusqlite::params![name, description, category_id],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_playlist(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM playlists WHERE id = ?1", [id])?;
    Ok(())
}

pub fn add_track_to_playlist(conn: &Connection, playlist_id: i64, track: &Track) -> Result<()> {
    let track_id = tracks::ensure_track(conn, track)?;

    let position: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position), -1) + 1 FROM playlist_tracks WHERE playlist_id = ?1",
        [playlist_id],
        |row| row.get(0),
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO playlist_tracks (playlist_id, track_id, position) VALUES (?1, ?2, ?3)",
        rusqlite::params![playlist_id, track_id, position],
    )?;

    Ok(())
}

pub fn get_playlist_tracks(conn: &Connection, playlist_id: i64) -> Result<Vec<Track>> {
    let mut stmt = conn.prepare(
        &format!(
            "SELECT {} FROM tracks t \
             JOIN playlist_tracks pt ON pt.track_id = t.id \
             WHERE pt.playlist_id = ?1 \
             ORDER BY pt.position",
            tracks::track_columns("t")
        ),
    )?;
    let rows = stmt.query_map([playlist_id], |row| track_from_row(row))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn remove_track_from_playlist(conn: &Connection, playlist_id: i64, track_id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND track_id = ?2",
        rusqlite::params![playlist_id, track_id],
    )?;
    Ok(())
}
