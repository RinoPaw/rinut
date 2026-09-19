use std::{env, fs, io, path::{Path, PathBuf}};

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

pub fn open(path: &Path) -> Result<Connection, Box<dyn std::error::Error>> {
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

fn migrate(connection: &Connection) -> SqlResult<()> {
    connection.execute_batch(
        "PRAGMA foreign_keys = ON;

         CREATE TABLE IF NOT EXISTS bookmarks (
             id         INTEGER PRIMARY KEY,
             url        TEXT NOT NULL,
             title      TEXT,
             note       TEXT,
             created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
             updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
         );",
    )
}
