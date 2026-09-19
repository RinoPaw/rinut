use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "rinut", version, about = "A local-first, programmable bookmark manager")]
pub struct Cli {
    /// Override the database path.
    #[arg(long, global = true, value_name = "PATH")]
    pub db: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Initialize the local Rinut database.
    Init,

    /// Store a new bookmark.
    Stash {
        /// URL to store.
        url: String,

        /// Optional title.
        #[arg(short, long)]
        title: Option<String>,

        /// Optional note.
        #[arg(short, long)]
        note: Option<String>,
    },

    /// List stored bookmarks.
    Forage,

    /// Show one bookmark in detail.
    Crack {
        /// Bookmark ID.
        id: i64,
    },

    /// Delete one bookmark.
    Remove {
        /// Bookmark ID.
        id: i64,
    },
}
