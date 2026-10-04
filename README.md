# Rinut

Rinut is a local-first, programmable bookmark manager.

The current milestone is a small local CLI backed by SQLite, with typed property keys and exact property matching.

## Commands

```console
rinut init [--db PATH]

rinut add URL

rinut list [-m KEY=VALUE | --match KEY=VALUE]...

rinut show ID

rinut edit ID \
    [--set KEY=VALUE]... \
    [--unset KEY[=VALUE]]...

rinut delete ID

rinut key add NAME \
    --type text|integer|number|boolean|taxonomy \
    --cardinality single|multi

rinut key list
rinut key show NAME
rinut key edit NAME --name NEW_NAME
rinut key delete NAME
```

`boolean` keys must use `single` cardinality and accept only `true` or `false`.

For `single` keys, `--set` replaces the previous value. For `multi` keys, repeated `--set` values form an unordered set. `--unset KEY=VALUE` removes one value and `--unset KEY` removes all values for that key.

`taxonomy` values have stable identities. Assigning a new taxonomy value creates a leaf value entity. The schema reserves group nodes and DAG edges for taxonomy organization; management commands for that hierarchy are intentionally not exposed yet.

## Database location

`rinut init --db PATH` can initialize a specific path. Other commands use `RINUT_DB` when set, otherwise the platform default data path.

## Development

```console
cargo test
```
