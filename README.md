# Rinut

Rinut is a local-first, programmable bookmark manager.

The project is intentionally starting small. v0.1 focuses on a reliable local CLI backed by SQLite; tags, typed keys, queries, sync, clients, and AI integration come later.

## Current commands

```console
rinut init
rinut stash https://example.com
rinut stash https://example.com --title "Example" --note "demo"
rinut forage
rinut crack 1
rinut remove 1
```

Use `--db <PATH>` to override the database location for testing or scripting. `RINUT_DB` is also recognized.

## Development

```console
cargo run -- init
cargo run -- stash https://example.com
cargo run -- forage
cargo run -- crack 1
cargo run -- remove 1
```

## Direction

Rinut is designed around a few long-term principles:

- local-first and self-hostable
- programmable from the CLI and APIs
- user-owned, exportable data
- typed keys and tag/property values instead of a fixed folder model
- multiple clients built on the same core semantics

The next milestone after the bookmark CRUD foundation is the Key/Tag property model.
