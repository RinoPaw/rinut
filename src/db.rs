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
    connection.execute_batch(
        "PRAGMA foreign_keys = ON;

         CREATE TABLE IF NOT EXISTS bookmarks (
             id         INTEGER PRIMARY KEY,
             url        TEXT NOT NULL,
             created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
             updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         );

         CREATE TABLE IF NOT EXISTS property_keys (
             id          INTEGER PRIMARY KEY,
             name        TEXT NOT NULL UNIQUE,
             type        TEXT NOT NULL CHECK (type IN ('text', 'integer', 'number', 'boolean', 'taxonomy')),
             cardinality TEXT NOT NULL CHECK (cardinality IN ('single', 'multi')),
             CHECK (type <> 'boolean' OR cardinality = 'single')
         );

         CREATE TABLE IF NOT EXISTS scalar_properties (
             bookmark_id INTEGER NOT NULL REFERENCES bookmarks(id) ON DELETE CASCADE,
             key_id      INTEGER NOT NULL REFERENCES property_keys(id) ON DELETE CASCADE,
             value       TEXT NOT NULL,
             PRIMARY KEY (bookmark_id, key_id, value)
         );

         CREATE TABLE IF NOT EXISTS taxonomy_nodes (
             id     INTEGER PRIMARY KEY,
             key_id INTEGER NOT NULL REFERENCES property_keys(id) ON DELETE CASCADE,
             name   TEXT NOT NULL,
             kind   TEXT NOT NULL CHECK (kind IN ('group', 'value')),
             UNIQUE (key_id, name)
         );

         CREATE TABLE IF NOT EXISTS taxonomy_edges (
             parent_id INTEGER NOT NULL REFERENCES taxonomy_nodes(id) ON DELETE CASCADE,
             child_id  INTEGER NOT NULL REFERENCES taxonomy_nodes(id) ON DELETE CASCADE,
             PRIMARY KEY (parent_id, child_id),
             CHECK (parent_id <> child_id)
         );

         CREATE TABLE IF NOT EXISTS taxonomy_properties (
             bookmark_id INTEGER NOT NULL REFERENCES bookmarks(id) ON DELETE CASCADE,
             key_id      INTEGER NOT NULL REFERENCES property_keys(id) ON DELETE CASCADE,
             value_id    INTEGER NOT NULL REFERENCES taxonomy_nodes(id) ON DELETE CASCADE,
             PRIMARY KEY (bookmark_id, key_id, value_id)
         );

         CREATE TRIGGER IF NOT EXISTS taxonomy_parent_must_be_group
         BEFORE INSERT ON taxonomy_edges
         WHEN (SELECT kind FROM taxonomy_nodes WHERE id = NEW.parent_id) <> 'group'
         BEGIN
             SELECT RAISE(ABORT, 'taxonomy parent must be a group');
         END;

         CREATE TRIGGER IF NOT EXISTS taxonomy_edge_same_key
         BEFORE INSERT ON taxonomy_edges
         WHEN (SELECT key_id FROM taxonomy_nodes WHERE id = NEW.parent_id)
              <> (SELECT key_id FROM taxonomy_nodes WHERE id = NEW.child_id)
         BEGIN
             SELECT RAISE(ABORT, 'taxonomy nodes must belong to the same key');
         END;",
    )
}

#[cfg(test)]
pub fn memory() -> SqlResult<Connection> {
    let connection = Connection::open_in_memory()?;
    migrate(&connection)?;
    Ok(connection)
}
