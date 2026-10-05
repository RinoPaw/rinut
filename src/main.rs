mod bookmark;
mod cli;
mod db;
mod tag;

use std::io;

use clap::Parser;
use cli::{Cli, Command, TagCommand};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init { db: path } => {
            let path = path.unwrap_or_else(db::default_db_path);
            db::initialize(&path)?;
            println!("Initialized Rinut at {}", path.display());
        }
        Command::Add { url } => {
            let connection = db::open_default()?;
            let id = bookmark::add(&connection, &url)?;
            println!("Added [{id}] {url}");
        }
        Command::List { selectors } => {
            let connection = db::open_default()?;
            let selectors = tag::resolve_selectors(&connection, &selectors)?;
            for bookmark in bookmark::list(&connection)? {
                if tag::bookmark_matches(&connection, bookmark.id, &selectors)? {
                    println!("[{}] {}", bookmark.id, bookmark.url);
                }
            }
        }
        Command::Show { id } => {
            let connection = db::open_default()?;
            let bookmark = require_bookmark(&connection, id)?;
            println!("ID:      {}", bookmark.id);
            println!("URL:     {}", bookmark.url);
            println!("Created: {}", bookmark.created_at);
            println!("Updated: {}", bookmark.updated_at);
            let tags = tag::list_for_bookmark(&connection, id)?;
            if !tags.is_empty() {
                println!("Tags:");
                for tag in tags {
                    println!("  {tag}");
                }
            }
        }
        Command::Edit { id, tags, untags } => {
            let mut connection = db::open_default()?;
            require_bookmark(&connection, id)?;
            let changed = !tags.is_empty() || !untags.is_empty();

            let transaction = connection.transaction()?;
            for name in tags {
                tag::assign(&transaction, id, &name)?;
            }
            for name in untags {
                tag::unassign(&transaction, id, &name)?;
            }
            if changed {
                bookmark::touch(&transaction, id)?;
            }
            transaction.commit()?;
            println!("Edited bookmark {id}");
        }
        Command::Delete { id } => {
            let connection = db::open_default()?;
            if bookmark::delete(&connection, id)? {
                println!("Deleted bookmark {id}");
            } else {
                return Err(not_found(format!("bookmark {id} not found")));
            }
        }
        Command::Tag { command } => {
            let connection = db::open_default()?;
            run_tag_command(&connection, command)?;
        }
    }

    Ok(())
}

fn run_tag_command(
    connection: &rusqlite::Connection,
    command: TagCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        TagCommand::Add { name } => {
            tag::add(connection, &name)?;
            println!("Added tag {name}");
        }
        TagCommand::List => {
            for tag in tag::list(connection)? {
                println!("{}", tag.name);
            }
        }
        TagCommand::Show { name } => {
            let item = tag::get(connection, &name)?
                .ok_or_else(|| not_found(format!("tag '{name}' not found")))?;
            println!("Name: {}", item.name);
            if let Some(parent) = tag::parent(connection, item.id)? {
                println!("Parent: {parent}");
            }
            let children = tag::children(connection, item.id)?;
            if !children.is_empty() {
                println!("Children:");
                for child in children {
                    println!("  {child}");
                }
            }
        }
        TagCommand::Edit { name, new_name } => {
            if tag::rename(connection, &name, &new_name)? {
                println!("Renamed tag {name} to {new_name}");
            } else {
                return Err(not_found(format!("tag '{name}' not found")));
            }
        }
        TagCommand::Delete { name } => {
            if tag::delete(connection, &name)? {
                println!("Deleted tag {name}");
            } else {
                return Err(not_found(format!("tag '{name}' not found")));
            }
        }
        TagCommand::Link { parent, child } => {
            tag::link(connection, &parent, &child)?;
            println!("Linked {parent} -> {child}");
        }
        TagCommand::Unlink { parent, child } => {
            if tag::unlink(connection, &parent, &child)? {
                println!("Unlinked {parent} -> {child}");
            } else {
                return Err(not_found(format!("tag link '{parent} -> {child}' not found")));
            }
        }
        TagCommand::Tree => {
            for line in tag::tree_lines(connection)? {
                println!("{line}");
            }
        }
    }
    Ok(())
}

fn require_bookmark(
    connection: &rusqlite::Connection,
    id: i64,
) -> Result<bookmark::Bookmark, Box<dyn std::error::Error>> {
    bookmark::get(connection, id)?.ok_or_else(|| not_found(format!("bookmark {id} not found")))
}

fn not_found(message: String) -> Box<dyn std::error::Error> {
    io::Error::new(io::ErrorKind::NotFound, message).into()
}
