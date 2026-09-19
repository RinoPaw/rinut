use rusqlite::{params, Connection, OptionalExtension, Result};

#[derive(Debug)]
pub struct Bookmark {
    pub id: i64,
    pub url: String,
    pub title: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
}

pub fn stash(
    conn: &Connection,
    url: &str,
    title: Option<&str>,
    note: Option<&str>,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO bookmarks (url, title, note) VALUES (?1, ?2, ?3)",
        params![url, title, note],
    )?;

    Ok(conn.last_insert_rowid())
}

pub fn list(conn: &Connection) -> Result<Vec<Bookmark>> {
    let mut statement = conn.prepare(
        "SELECT id, url, title, note, created_at
         FROM bookmarks
         ORDER BY id",
    )?;

    let rows = statement.query_map([], bookmark_from_row)?;
    rows.collect()
}

pub fn get(conn: &Connection, id: i64) -> Result<Option<Bookmark>> {
    conn.query_row(
        "SELECT id, url, title, note, created_at
         FROM bookmarks
         WHERE id = ?1",
        [id],
        bookmark_from_row,
    )
    .optional()
}

pub fn remove(conn: &Connection, id: i64) -> Result<bool> {
    Ok(conn.execute("DELETE FROM bookmarks WHERE id = ?1", [id])? > 0)
}

fn bookmark_from_row(row: &rusqlite::Row<'_>) -> Result<Bookmark> {
    Ok(Bookmark {
        id: row.get(0)?,
        url: row.get(1)?,
        title: row.get(2)?,
        note: row.get(3)?,
        created_at: row.get(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connection() -> Result<Connection> {
        let connection = Connection::open_in_memory()?;
        connection.execute_batch(
            "CREATE TABLE bookmarks (
                id         INTEGER PRIMARY KEY,
                url        TEXT NOT NULL,
                title      TEXT,
                note       TEXT,
                created_at TEXT NOT NULL DEFAULT 'test'
            );",
        )?;
        Ok(connection)
    }

    #[test]
    fn bookmark_crud_round_trip() -> Result<()> {
        let connection = connection()?;

        let id = stash(
            &connection,
            "https://example.com",
            Some("Example"),
            Some("demo"),
        )?;
        assert_eq!(id, 1);

        let bookmarks = list(&connection)?;
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].url, "https://example.com");

        let bookmark = get(&connection, id)?.expect("bookmark should exist");
        assert_eq!(bookmark.title.as_deref(), Some("Example"));
        assert_eq!(bookmark.note.as_deref(), Some("demo"));

        assert!(remove(&connection, id)?);
        assert!(get(&connection, id)?.is_none());
        assert!(!remove(&connection, id)?);

        Ok(())
    }
}
