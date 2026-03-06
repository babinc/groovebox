use std::str::FromStr;

use anyhow::Result;
use rusqlite::Connection;

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
    let result = stmt.query_row([key], |row| row.get(0)).ok();
    Ok(result)
}

pub fn get_parsed<T: FromStr>(conn: &Connection, key: &str) -> Option<T> {
    get_setting(conn, key).ok().flatten().and_then(|v| v.parse().ok())
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )?;
    Ok(())
}
