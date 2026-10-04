mod bookmark;
mod cli;
mod db;
mod key;
mod property;

use std::io;

use clap::Parser;
use cli::{CardinalityArg, Cli, Command, KeyCommand, KeyTypeArg};
use key::{Cardinality, KeyType};

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
        Command::List { matches } => {
            let connection = db::open_default()?;
            let conditions = property::resolve_matches(&connection, &matches)?;
            let bookmarks = bookmark::list(&connection)?;

            for bookmark in bookmarks {
                let mut matched = true;
                for condition in &conditions {
                    if !property::bookmark_matches(&connection, bookmark.id, condition)? {
                        matched = false;
                        break;
                    }
                }
                if matched {
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
            let properties = property::list_for_bookmark(&connection, id)?;
            if !properties.is_empty() {
                println!("Properties:");
                for (key, value) in properties {
                    println!("  {key}={value}");
                }
            }
        }
        Command::Edit { id, set, unset } => {
            let mut connection = db::open_default()?;
            require_bookmark(&connection, id)?;

            let transaction = connection.transaction()?;
            for value in set {
                property::set(&transaction, id, &value)?;
            }
            for value in unset {
                property::unset(&transaction, id, &value)?;
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
        Command::Key { command } => {
            let connection = db::open_default()?;
            run_key_command(&connection, command)?;
        }
    }

    Ok(())
}

fn run_key_command(
    connection: &rusqlite::Connection,
    command: KeyCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        KeyCommand::Add {
            name,
            key_type,
            cardinality,
        } => {
            key::add(
                connection,
                &name,
                key_type_from_arg(key_type),
                cardinality_from_arg(cardinality),
            )?;
            println!("Added key {name}");
        }
        KeyCommand::List => {
            for key in key::list(connection)? {
                println!(
                    "{}\t{}\t{}",
                    key.name,
                    key.key_type.as_str(),
                    key.cardinality.as_str()
                );
            }
        }
        KeyCommand::Show { name } => {
            let key = key::get(connection, &name)?
                .ok_or_else(|| not_found(format!("key '{name}' not found")))?;
            println!("Name:        {}", key.name);
            println!("Type:        {}", key.key_type.as_str());
            println!("Cardinality: {}", key.cardinality.as_str());
        }
        KeyCommand::Edit { name, new_name } => {
            if key::rename(connection, &name, &new_name)? {
                println!("Renamed key {name} to {new_name}");
            } else {
                return Err(not_found(format!("key '{name}' not found")));
            }
        }
        KeyCommand::Delete { name } => {
            if key::delete(connection, &name)? {
                println!("Deleted key {name}");
            } else {
                return Err(not_found(format!("key '{name}' not found")));
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

fn key_type_from_arg(value: KeyTypeArg) -> KeyType {
    KeyType::parse(value.as_str()).expect("CLI key type must be valid")
}

fn cardinality_from_arg(value: CardinalityArg) -> Cardinality {
    Cardinality::parse(value.as_str()).expect("CLI cardinality must be valid")
}
