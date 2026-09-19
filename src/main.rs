mod bookmark;
mod cli;
mod db;

use std::io;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let db_path = cli.db.unwrap_or_else(db::default_db_path);

    match cli.command {
        Command::Init => {
            db::initialize(&db_path)?;
            println!("Initialized Rinut at {}", db_path.display());
        }
        Command::Stash { url, title, note } => {
            let connection = db::open(&db_path)?;
            let id = bookmark::stash(
                &connection,
                &url,
                title.as_deref(),
                note.as_deref(),
            )?;
            println!("Stashed [{id}] {url}");
        }
        Command::Forage => {
            let connection = db::open(&db_path)?;
            let bookmarks = bookmark::list(&connection)?;

            if bookmarks.is_empty() {
                println!("No bookmarks.");
            } else {
                for bookmark in bookmarks {
                    let title = bookmark.title.as_deref().unwrap_or("(untitled)");
                    println!("[{}] {}\n    {}", bookmark.id, title, bookmark.url);
                }
            }
        }
        Command::Crack { id } => {
            let connection = db::open(&db_path)?;
            let bookmark = bookmark::get(&connection, id)?.ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, format!("bookmark {id} not found"))
            })?;

            println!("ID:      {}", bookmark.id);
            println!("Title:   {}", bookmark.title.as_deref().unwrap_or("(untitled)"));
            println!("URL:     {}", bookmark.url);
            if let Some(note) = bookmark.note {
                println!("Note:    {note}");
            }
            println!("Created: {}", bookmark.created_at);
        }
        Command::Remove { id } => {
            let connection = db::open(&db_path)?;
            if bookmark::remove(&connection, id)? {
                println!("Removed bookmark {id}");
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("bookmark {id} not found"),
                )
                .into());
            }
        }
    }

    Ok(())
}
