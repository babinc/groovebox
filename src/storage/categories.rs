use anyhow::Result;
use rusqlite::Connection;

use crate::models::Category;

pub fn list_categories(conn: &Connection) -> Result<Vec<Category>> {
    let mut stmt = conn.prepare("SELECT id, name, color, icon FROM categories ORDER BY name")?;
    let rows = stmt.query_map([], |row| {
        Ok(Category {
            id: Some(row.get(0)?),
            name: row.get(1)?,
            color: row.get(2)?,
            icon: row.get(3)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn create_category(conn: &Connection, name: &str, color: &str, icon: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO categories (name, color, icon) VALUES (?1, ?2, ?3)",
        rusqlite::params![name, color, icon],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_category(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM categories WHERE id = ?1", [id])?;
    Ok(())
}
