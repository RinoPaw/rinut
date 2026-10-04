use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "rinut", version, about = "A local-first, programmable bookmark manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Initialize the local Rinut database.
    Init {
        /// Override the database path for initialization.
        #[arg(long, value_name = "PATH")]
        db: Option<PathBuf>,
    },

    /// Add a bookmark.
    Add {
        /// URL to store.
        url: String,
    },

    /// List bookmarks.
    List {
        /// Match bookmarks by property. Repeat for AND semantics.
        #[arg(short = 'm', long = "match", value_name = "KEY=VALUE")]
        matches: Vec<String>,
    },

    /// Show one bookmark.
    Show {
        /// Bookmark ID.
        id: i64,
    },

    /// Edit one bookmark's properties.
    Edit {
        /// Bookmark ID.
        id: i64,

        /// Ensure a property value exists. Repeatable.
        #[arg(long, value_name = "KEY=VALUE")]
        set: Vec<String>,

        /// Remove one value or all values for a key. Repeatable.
        #[arg(long, value_name = "KEY[=VALUE]")]
        unset: Vec<String>,
    },

    /// Permanently delete one bookmark.
    Delete {
        /// Bookmark ID.
        id: i64,
    },

    /// Manage property keys.
    Key {
        #[command(subcommand)]
        command: KeyCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum KeyCommand {
    /// Create a key.
    Add {
        name: String,

        #[arg(long = "type", value_enum)]
        key_type: KeyTypeArg,

        #[arg(long, value_enum)]
        cardinality: CardinalityArg,
    },

    /// List keys.
    List,

    /// Show one key.
    Show { name: String },

    /// Rename a key.
    Edit {
        name: String,

        #[arg(long = "name", value_name = "NEW_NAME")]
        new_name: String,
    },

    /// Delete a key.
    Delete { name: String },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum KeyTypeArg {
    Text,
    Integer,
    Number,
    Boolean,
    Taxonomy,
}

impl KeyTypeArg {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Integer => "integer",
            Self::Number => "number",
            Self::Boolean => "boolean",
            Self::Taxonomy => "taxonomy",
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CardinalityArg {
    Single,
    Multi,
}

impl CardinalityArg {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Multi => "multi",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_is_only_available_on_init() {
        assert!(Cli::try_parse_from(["rinut", "init", "--db", "test.db"]).is_ok());
        assert!(Cli::try_parse_from(["rinut", "add", "https://example.com", "--db", "test.db"]).is_err());
    }

    #[test]
    fn add_has_no_property_shortcuts() {
        assert!(Cli::try_parse_from(["rinut", "add", "https://example.com"]).is_ok());
        assert!(Cli::try_parse_from(["rinut", "add", "https://example.com", "--set", "topic=vulkan"]).is_err());
    }

    #[test]
    fn key_add_requires_schema() {
        assert!(Cli::try_parse_from(["rinut", "key", "add", "topic"]).is_err());
        assert!(Cli::try_parse_from([
            "rinut", "key", "add", "topic", "--type", "taxonomy", "--cardinality", "multi"
        ])
        .is_ok());
    }
}
