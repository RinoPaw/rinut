use rusqlite::{Connection, OptionalExtension, Result};

#[derive(Debug)]
pub struct Bookmark {
    pub id: i64,
    pub url: String,
    pub created_at: String,
    pub updated_at: String,
}

pub fn add(conn: &Connection, url: &str) -> Result<i64> {
    conn.execute("INSERT INTO bookmarks (url) VALUES (?1)", [url])?;
    Ok(conn.last_insert_rowid())
}

pub fn list(conn: &Connection) -> Result<Vec<Bookmark>> {
    let mut statement = conn.prepare(
        "SELECT id, url, created_at, updated_at
         FROM bookmarks
         ORDER BY id",
    )?;

    let rows = statement.query_map([], bookmark_from_row)?;
    rows.collect()
}

pub fn get(conn: &Connection, id: i64) -> Result<Option<Bookmark>> {
    conn.query_row(
        "SELECT id, url, created_at, updated_at
         FROM bookmarks
         WHERE id = ?1",
        [id],
        bookmark_from_row,
    )
    .optional()
}

pub fn delete(conn: &Connection, id: i64) -> Result<bool> {
    Ok(conn.execute("DELETE FROM bookmarks WHERE id = ?1", [id])? > 0)
}

pub fn touch(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE bookmarks
         SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE id = ?1",
        [id],
    )?;
    Ok(())
}

fn bookmark_from_row(row: &rusqlite::Row<'_>) -> Result<Bookmark> {
    Ok(Bookmark {
        id: row.get(0)?,
        url: row.get(1)?,
        created_at: row.get(2)?,
        updated_at: row.get(3)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bookmark_crud_round_trip() -> Result<()> {
        let connection = crate::db::memory()?;

        let id = add(&connection, "https://example.com")?;
        assert_eq!(id, 1);

        let bookmarks = list(&connection)?;
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].url, "https://example.com");

        let bookmark = get(&connection, id)?.expect("bookmark should exist");
        assert_eq!(bookmark.url, "https://example.com");

        assert!(delete(&connection, id)?);
        assert!(get(&connection, id)?.is_none());
        assert!(!delete(&connection, id)?);

        Ok(())
    }
}
