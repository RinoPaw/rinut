# Rinut

Rinut is a local-first, programmable bookmark manager.

The current milestone is a small local CLI backed by SQLite. A bookmark has a URL and an unordered set of tags. Tags are global entities and form a forest for broader/narrower relationships.

## Commands

```console
rinut init [--db PATH]

rinut add URL

rinut list [+TAG]... [-TAG]...

rinut show ID

rinut edit ID \
    [--tag TAG]... \
    [--untag TAG]...

rinut delete ID

rinut tag add NAME
rinut tag list
rinut tag show NAME
rinut tag edit NAME --name NEW_NAME
rinut tag delete NAME

rinut tag link PARENT CHILD
rinut tag unlink PARENT CHILD
rinut tag tree
```

`+TAG` requires the bookmark to have that tag or one of its descendants. Multiple positive selectors use AND semantics. `-TAG` excludes bookmarks with that tag or one of its descendants.

Tags may be assigned whether or not they have children. `tag link` creates a broader -> narrower relationship and rejects cycles. Each tag can have at most one parent. To reparent a tag, unlink its current parent first.

`edit --tag` requires the tag to exist. Use `tag add` to extend the vocabulary explicitly; this keeps typos from silently creating tags.

Tag names should generally stay at 15 characters or fewer, counting separators such as `-`. Prefer familiar abbreviations such as `AI`, `ML`, `LLM`, `CS`, and `CG` when they keep names clear. This is a naming guideline rather than a CLI length restriction.

## Migration from the key model

Opening an older database automatically performs a one-time migration:

- scalar property values become tags;
- assigned taxonomy nodes become tags;
- taxonomy hierarchy edges become tag edges;
- key names, types, and cardinality are discarded;
- the legacy property tables are removed after a successful migration.

If the same value existed under more than one old key, it becomes one global tag.

Databases created by the earlier DAG-based tag model are also normalized automatically. If a tag already has multiple parents, Rinut keeps the earliest established parent edge and removes the others before enforcing the one-parent rule.

## Database location

`rinut init --db PATH` can initialize a specific path. Other commands use `RINUT_DB` when set, otherwise the platform default data path.

## Development

```console
cargo test
```
