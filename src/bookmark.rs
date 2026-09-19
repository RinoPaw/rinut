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
