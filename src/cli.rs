use std::path::PathBuf;

use clap::{Parser, Subcommand};

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

    /// List bookmarks, optionally filtering by tags.
    List {
        /// Tag selectors. +TAG requires a tag (or descendant); -TAG excludes it.
        #[arg(value_name = "SELECTOR", allow_hyphen_values = true)]
        selectors: Vec<String>,
    },

    /// Show one bookmark.
    Show {
        /// Bookmark ID.
        id: i64,
    },

    /// Edit one bookmark's tags.
    Edit {
        /// Bookmark ID.
        id: i64,

        /// Add a tag to the bookmark. Repeatable.
        #[arg(long = "tag", value_name = "TAG")]
        tags: Vec<String>,

        /// Remove a tag from the bookmark. Repeatable.
        #[arg(long = "untag", value_name = "TAG")]
        untags: Vec<String>,
    },

    /// Permanently delete one bookmark.
    Delete {
        /// Bookmark ID.
        id: i64,
    },

    /// Manage tags and their hierarchy.
    Tag {
        #[command(subcommand)]
        command: TagCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum TagCommand {
    /// Create a tag.
    Add { name: String },

    /// List tags.
    List,

    /// Show one tag and its direct relationships.
    Show { name: String },

    /// Rename a tag.
    Edit {
        name: String,

        #[arg(long = "name", value_name = "NEW_NAME")]
        new_name: String,
    },

    /// Delete a tag and remove it from bookmarks.
    Delete { name: String },

    /// Add a broader -> narrower relationship.
    Link { parent: String, child: String },

    /// Remove a broader -> narrower relationship.
    Unlink { parent: String, child: String },

    /// Show the tag DAG as an indented tree (multi-parent tags may appear more than once).
    Tree,
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
    fn list_accepts_positive_and_negative_tag_selectors() {
        assert!(Cli::try_parse_from(["rinut", "list", "+graphics", "-tutorial"]).is_ok());
    }

    #[test]
    fn edit_uses_tags_instead_of_properties() {
        assert!(Cli::try_parse_from([
            "rinut", "edit", "1", "--tag", "vulkan", "--untag", "tutorial"
        ])
        .is_ok());
        assert!(Cli::try_parse_from(["rinut", "edit", "1", "--set", "topic=vulkan"]).is_err());
    }

    #[test]
    fn tag_hierarchy_commands_parse() {
        assert!(Cli::try_parse_from(["rinut", "tag", "add", "computer-science"]).is_ok());
        assert!(Cli::try_parse_from([
            "rinut", "tag", "link", "computer-science", "computer-graphics"
        ])
        .is_ok());
    }
}
