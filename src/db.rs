use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use rusqlite::{Connection, Result as SqlResult};

pub fn default_db_path() -> PathBuf {
    if let Some(path) = env::var_os("RINUT_DB") {
        return PathBuf::from(path);
    }

    if cfg!(target_os = "windows") {
        if let Some(path) = env::var_os("LOCALAPPDATA") {
            return PathBuf::from(path).join("rinut").join("rinut.db");
        }
    }

    if cfg!(target_os = "macos") {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("rinut")
                .join("rinut.db");
        }
    }

    if let Some(path) = env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(path).join("rinut").join("rinut.db");
    }

    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("rinut")
            .join("rinut.db");
    }

    PathBuf::from("rinut.db")
}

pub fn initialize(path: &Path) -> Result<Connection, Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }

    let connection = Connection::open(path)?;
    migrate(&connection)?;
    Ok(connection)
}

pub fn open_default() -> Result<Connection, Box<dyn std::error::Error>> {
    open(&default_db_path())
}

fn open(path: &Path) -> Result<Connection, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "Rinut database not found at {}. Run `rinut init` first.",
                path.display()
            ),
        )
        .into());
    }

    let connection = Connection::open(path)?;
    migrate(&connection)?;
    Ok(connection)
}

pub(crate) fn migrate(connection: &Connection) -> SqlResult<()> {
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    let legacy_schema = table_exists(connection, "property_keys")?;

    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS bookmarks (
             id         INTEGER PRIMARY KEY,
             url        TEXT NOT NULL,
             created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
             updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         );

         CREATE TABLE IF NOT EXISTS tags (
             id   INTEGER PRIMARY KEY,
             name TEXT NOT NULL UNIQUE
         );

         CREATE TABLE IF NOT EXISTS tag_edges (
             parent_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
             child_id  INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
             PRIMARY KEY (parent_id, child_id),
             CHECK (parent_id <> child_id)
         );

         CREATE TABLE IF NOT EXISTS bookmark_tags (
             bookmark_id INTEGER NOT NULL REFERENCES bookmarks(id) ON DELETE CASCADE,
             tag_id      INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
             PRIMARY KEY (bookmark_id, tag_id)
         );

         CREATE INDEX IF NOT EXISTS bookmark_tags_tag_id ON bookmark_tags(tag_id);

         CREATE TRIGGER IF NOT EXISTS tag_edge_no_cycle
         BEFORE INSERT ON tag_edges
         WHEN EXISTS (
             WITH RECURSIVE descendants(id) AS (
                 SELECT NEW.child_id
                 UNION
                 SELECT e.child_id
                 FROM tag_edges e
                 JOIN descendants d ON e.parent_id = d.id
             )
             SELECT 1 FROM descendants WHERE id = NEW.parent_id
         )
         BEGIN
             SELECT RAISE(ABORT, 'tag hierarchy cannot contain a cycle');
         END;",
    )?;

    if legacy_schema {
        migrate_legacy_schema(connection)?;
    }

    connection.execute_batch(
        "DELETE FROM tag_edges
         WHERE rowid NOT IN (
             SELECT MIN(rowid)
             FROM tag_edges
             GROUP BY child_id
         );

         DROP INDEX IF EXISTS tag_edges_child_id;
         CREATE UNIQUE INDEX IF NOT EXISTS tag_edges_one_parent ON tag_edges(child_id);",
    )?;

    Ok(())
}

fn migrate_legacy_schema(connection: &Connection) -> SqlResult<()> {
    let result = connection.execute_batch(
        "BEGIN IMMEDIATE;

         INSERT OR IGNORE INTO tags (name)
         SELECT value FROM scalar_properties;

         INSERT OR IGNORE INTO tags (name)
         SELECT DISTINCT n.name
         FROM taxonomy_nodes n
         WHERE EXISTS (SELECT 1 FROM taxonomy_properties p WHERE p.value_id = n.id)
            OR EXISTS (SELECT 1 FROM taxonomy_edges e WHERE e.parent_id = n.id OR e.child_id = n.id);

         INSERT OR IGNORE INTO bookmark_tags (bookmark_id, tag_id)
         SELECT p.bookmark_id, t.id
         FROM scalar_properties p
         JOIN tags t ON t.name = p.value;

         INSERT OR IGNORE INTO bookmark_tags (bookmark_id, tag_id)
         SELECT p.bookmark_id, t.id
         FROM taxonomy_properties p
         JOIN taxonomy_nodes n ON n.id = p.value_id
         JOIN tags t ON t.name = n.name;

         INSERT OR IGNORE INTO tag_edges (parent_id, child_id)
         SELECT parent_tag.id, child_tag.id
         FROM taxonomy_edges e
         JOIN taxonomy_nodes parent_node ON parent_node.id = e.parent_id
         JOIN taxonomy_nodes child_node ON child_node.id = e.child_id
         JOIN tags parent_tag ON parent_tag.name = parent_node.name
         JOIN tags child_tag ON child_tag.name = child_node.name;

         DROP TRIGGER IF EXISTS taxonomy_parent_must_be_group;
         DROP TRIGGER IF EXISTS taxonomy_edge_same_key;
         DROP TABLE IF EXISTS taxonomy_properties;
         DROP TABLE IF EXISTS taxonomy_edges;
         DROP TABLE IF EXISTS taxonomy_nodes;
         DROP TABLE IF EXISTS scalar_properties;
         DROP TABLE IF EXISTS property_keys;

         COMMIT;",
    );

    if result.is_err() {
        let _ = connection.execute_batch("ROLLBACK;");
    }
    result
}

fn table_exists(connection: &Connection, name: &str) -> SqlResult<bool> {
    connection.query_row(
        "SELECT EXISTS(
             SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1
         )",
        [name],
        |row| row.get(0),
    )
}

#[cfg(test)]
pub fn memory() -> SqlResult<Connection> {
    let connection = Connection::open_in_memory()?;
    migrate(&connection)?;
    Ok(connection)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_properties_are_flattened_into_tags_once() -> SqlResult<()> {
        let connection = Connection::open_in_memory()?;
        connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE bookmarks (
                 id INTEGER PRIMARY KEY,
                 url TEXT NOT NULL,
                 created_at TEXT NOT NULL DEFAULT '',
                 updated_at TEXT NOT NULL DEFAULT ''
             );
             CREATE TABLE property_keys (
                 id INTEGER PRIMARY KEY,
                 name TEXT NOT NULL UNIQUE,
                 type TEXT NOT NULL,
                 cardinality TEXT NOT NULL
             );
             CREATE TABLE scalar_properties (
                 bookmark_id INTEGER NOT NULL,
                 key_id INTEGER NOT NULL,
                 value TEXT NOT NULL,
                 PRIMARY KEY (bookmark_id, key_id, value)
             );
             CREATE TABLE taxonomy_nodes (
                 id INTEGER PRIMARY KEY,
                 key_id INTEGER NOT NULL,
                 name TEXT NOT NULL,
                 kind TEXT NOT NULL,
                 UNIQUE (key_id, name)
             );
             CREATE TABLE taxonomy_edges (
                 parent_id INTEGER NOT NULL,
                 child_id INTEGER NOT NULL,
                 PRIMARY KEY (parent_id, child_id)
             );
             CREATE TABLE taxonomy_properties (
                 bookmark_id INTEGER NOT NULL,
                 key_id INTEGER NOT NULL,
                 value_id INTEGER NOT NULL,
                 PRIMARY KEY (bookmark_id, key_id, value_id)
             );
             INSERT INTO bookmarks (id, url) VALUES (1, 'https://example.com');
             INSERT INTO property_keys VALUES (1, 'domain', 'taxonomy', 'multi');
             INSERT INTO taxonomy_nodes VALUES (1, 1, 'computer-science', 'group');
             INSERT INTO taxonomy_nodes VALUES (2, 1, 'computer-graphics', 'value');
             INSERT INTO taxonomy_edges VALUES (1, 2);
             INSERT INTO taxonomy_properties VALUES (1, 1, 2);",
        )?;

        migrate(&connection)?;
        assert!(!table_exists(&connection, "property_keys")?);
        let assigned: i64 = connection.query_row(
            "SELECT COUNT(*) FROM bookmark_tags bt JOIN tags t ON t.id = bt.tag_id
             WHERE bt.bookmark_id = 1 AND t.name = 'computer-graphics'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(assigned, 1);
        let edge: i64 = connection.query_row(
            "SELECT COUNT(*) FROM tag_edges e
             JOIN tags p ON p.id = e.parent_id
             JOIN tags c ON c.id = e.child_id
             WHERE p.name = 'computer-science' AND c.name = 'computer-graphics'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(edge, 1);

        migrate(&connection)?;
        let assigned_after_second_migration: i64 = connection.query_row(
            "SELECT COUNT(*) FROM bookmark_tags bt JOIN tags t ON t.id = bt.tag_id
             WHERE bt.bookmark_id = 1 AND t.name = 'computer-graphics'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(assigned_after_second_migration, 1);
        Ok(())
    }

    #[test]
    fn existing_multiple_parents_keep_the_earliest_edge() -> SqlResult<()> {
        let connection = Connection::open_in_memory()?;
        connection.execute_batch(
            "CREATE TABLE tags (
                 id INTEGER PRIMARY KEY,
                 name TEXT NOT NULL UNIQUE
             );
             CREATE TABLE tag_edges (
                 parent_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
                 child_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
                 PRIMARY KEY (parent_id, child_id),
                 CHECK (parent_id <> child_id)
             );
             INSERT INTO tags (id, name) VALUES
                 (1, 'first-parent'),
                 (2, 'second-parent'),
                 (3, 'child');
             INSERT INTO tag_edges (parent_id, child_id) VALUES (1, 3);
             INSERT INTO tag_edges (parent_id, child_id) VALUES (2, 3);",
        )?;

        migrate(&connection)?;

        let parent_id: i64 = connection.query_row(
            "SELECT parent_id FROM tag_edges WHERE child_id = 3",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(parent_id, 1);
        assert!(connection
            .execute(
                "INSERT INTO tag_edges (parent_id, child_id) VALUES (2, 3)",
                [],
            )
            .is_err());
        Ok(())
    }
}
