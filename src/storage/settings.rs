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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::database;

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        database::migrate(&conn).unwrap();
        conn
    }

    #[test]
    fn get_missing_key_returns_none() {
        let conn = test_db();
        assert_eq!(get_setting(&conn, "nonexistent").unwrap(), None);
    }

    #[test]
    fn set_and_get_roundtrip() {
        let conn = test_db();
        set_setting(&conn, "volume", "75").unwrap();
        assert_eq!(get_setting(&conn, "volume").unwrap(), Some("75".into()));
    }

    #[test]
    fn set_overwrites_existing() {
        let conn = test_db();
        set_setting(&conn, "theme", "0").unwrap();
        set_setting(&conn, "theme", "3").unwrap();
        assert_eq!(get_setting(&conn, "theme").unwrap(), Some("3".into()));
    }

    #[test]
    fn get_parsed_returns_typed_value() {
        let conn = test_db();
        set_setting(&conn, "volume", "85.5").unwrap();
        assert_eq!(get_parsed::<f64>(&conn, "volume"), Some(85.5));
    }

    #[test]
    fn get_parsed_returns_none_for_bad_type() {
        let conn = test_db();
        set_setting(&conn, "name", "hello").unwrap();
        assert_eq!(get_parsed::<f64>(&conn, "name"), None);
    }

    #[test]
    fn get_parsed_returns_none_for_missing() {
        let conn = test_db();
        assert_eq!(get_parsed::<u32>(&conn, "nope"), None);
    }
}
