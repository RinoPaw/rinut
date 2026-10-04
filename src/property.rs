use std::io;

use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    bookmark,
    key::{self, Cardinality, Key, KeyType},
};

#[derive(Debug)]
pub struct Match {
    key: Key,
    value: String,
}

pub fn list_for_bookmark(
    conn: &Connection,
    bookmark_id: i64,
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    let mut statement = conn.prepare(
        "SELECT k.name, p.value
         FROM scalar_properties p
         JOIN property_keys k ON k.id = p.key_id
         WHERE p.bookmark_id = ?1
         UNION ALL
         SELECT k.name, n.name
         FROM taxonomy_properties p
         JOIN property_keys k ON k.id = p.key_id
         JOIN taxonomy_nodes n ON n.id = p.value_id
         WHERE p.bookmark_id = ?1
         ORDER BY 1, 2",
    )?;

    let rows = statement.query_map([bookmark_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn resolve_matches(
    conn: &Connection,
    values: &[String],
) -> Result<Vec<Match>, Box<dyn std::error::Error>> {
    values
        .iter()
        .map(|value| {
            let (name, raw) = split_set(value)?;
            let key = require_key(conn, name)?;
            let value = normalize_value(&key, raw)?;
            Ok(Match { key, value })
        })
        .collect()
}

pub fn bookmark_matches(
    conn: &Connection,
    bookmark_id: i64,
    condition: &Match,
) -> Result<bool, Box<dyn std::error::Error>> {
    match condition.key.key_type {
        KeyType::Taxonomy => taxonomy_matches(conn, bookmark_id, &condition.key, &condition.value),
        _ => Ok(conn.query_row(
            "SELECT EXISTS(
                 SELECT 1
                 FROM scalar_properties
                 WHERE bookmark_id = ?1 AND key_id = ?2 AND value = ?3
             )",
            params![bookmark_id, condition.key.id, condition.value],
            |row| row.get::<_, bool>(0),
        )?),
    }
}

pub fn set(
    conn: &Connection,
    bookmark_id: i64,
    input: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let (name, raw) = split_set(input)?;
    let key = require_key(conn, name)?;
    let value = normalize_value(&key, raw)?;

    match key.key_type {
        KeyType::Taxonomy => set_taxonomy(conn, bookmark_id, &key, &value)?,
        _ => set_scalar(conn, bookmark_id, &key, &value)?,
    }
    bookmark::touch(conn, bookmark_id)?;
    Ok(())
}

pub fn unset(
    conn: &Connection,
    bookmark_id: i64,
    input: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let (name, raw) = split_unset(input)?;
    let key = require_key(conn, name)?;

    match (key.key_type, raw) {
        (KeyType::Taxonomy, Some(raw)) => {
            let value = normalize_value(&key, raw)?;
            unset_taxonomy_value(conn, bookmark_id, &key, &value)?;
        }
        (KeyType::Taxonomy, None) => {
            conn.execute(
                "DELETE FROM taxonomy_properties WHERE bookmark_id = ?1 AND key_id = ?2",
                params![bookmark_id, key.id],
            )?;
        }
        (_, Some(raw)) => {
            let value = normalize_value(&key, raw)?;
            conn.execute(
                "DELETE FROM scalar_properties
                 WHERE bookmark_id = ?1 AND key_id = ?2 AND value = ?3",
                params![bookmark_id, key.id, value],
            )?;
        }
        (_, None) => {
            conn.execute(
                "DELETE FROM scalar_properties WHERE bookmark_id = ?1 AND key_id = ?2",
                params![bookmark_id, key.id],
            )?;
        }
    }

    bookmark::touch(conn, bookmark_id)?;
    Ok(())
}

fn set_scalar(
    conn: &Connection,
    bookmark_id: i64,
    key: &Key,
    value: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if key.cardinality == Cardinality::Single {
        conn.execute(
            "DELETE FROM scalar_properties WHERE bookmark_id = ?1 AND key_id = ?2",
            params![bookmark_id, key.id],
        )?;
    }

    conn.execute(
        "INSERT OR IGNORE INTO scalar_properties (bookmark_id, key_id, value)
         VALUES (?1, ?2, ?3)",
        params![bookmark_id, key.id, value],
    )?;
    Ok(())
}

fn set_taxonomy(
    conn: &Connection,
    bookmark_id: i64,
    key: &Key,
    value: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    conn.execute(
        "INSERT OR IGNORE INTO taxonomy_nodes (key_id, name, kind)
         VALUES (?1, ?2, 'value')",
        params![key.id, value],
    )?;

    let (value_id, kind): (i64, String) = conn.query_row(
        "SELECT id, kind FROM taxonomy_nodes WHERE key_id = ?1 AND name = ?2",
        params![key.id, value],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    if kind != "value" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("taxonomy group '{value}' cannot be assigned as a value"),
        )
        .into());
    }

    if key.cardinality == Cardinality::Single {
        conn.execute(
            "DELETE FROM taxonomy_properties WHERE bookmark_id = ?1 AND key_id = ?2",
            params![bookmark_id, key.id],
        )?;
    }

    conn.execute(
        "INSERT OR IGNORE INTO taxonomy_properties (bookmark_id, key_id, value_id)
         VALUES (?1, ?2, ?3)",
        params![bookmark_id, key.id, value_id],
    )?;
    Ok(())
}

fn unset_taxonomy_value(
    conn: &Connection,
    bookmark_id: i64,
    key: &Key,
    value: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let node = conn
        .query_row(
            "SELECT id, kind FROM taxonomy_nodes WHERE key_id = ?1 AND name = ?2",
            params![key.id, value],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;

    let Some((value_id, kind)) = node else {
        return Ok(());
    };
    if kind != "value" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("taxonomy group '{value}' is not an assignable value"),
        )
        .into());
    }

    conn.execute(
        "DELETE FROM taxonomy_properties
         WHERE bookmark_id = ?1 AND key_id = ?2 AND value_id = ?3",
        params![bookmark_id, key.id, value_id],
    )?;
    Ok(())
}

fn taxonomy_matches(
    conn: &Connection,
    bookmark_id: i64,
    key: &Key,
    value: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let node_id = conn
        .query_row(
            "SELECT id FROM taxonomy_nodes WHERE key_id = ?1 AND name = ?2",
            params![key.id, value],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;

    let Some(node_id) = node_id else {
        return Ok(false);
    };

    Ok(conn.query_row(
        "WITH RECURSIVE descendants(id) AS (
             SELECT ?3
             UNION
             SELECT e.child_id
             FROM taxonomy_edges e
             JOIN descendants d ON e.parent_id = d.id
         )
         SELECT EXISTS(
             SELECT 1
             FROM taxonomy_properties p
             WHERE p.bookmark_id = ?1
               AND p.key_id = ?2
               AND p.value_id IN (SELECT id FROM descendants)
         )",
        params![bookmark_id, key.id, node_id],
        |row| row.get::<_, bool>(0),
    )?)
}

fn require_key(conn: &Connection, name: &str) -> Result<Key, Box<dyn std::error::Error>> {
    key::get(conn, name)?.ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, format!("key '{name}' not found")).into()
    })
}

fn split_set(input: &str) -> Result<(&str, &str), Box<dyn std::error::Error>> {
    let (key, value) = input.split_once('=').ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("expected KEY=VALUE, got '{input}'"),
        )
    })?;
    if key.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "key cannot be empty").into());
    }
    Ok((key, value))
}

fn split_unset(input: &str) -> Result<(&str, Option<&str>), Box<dyn std::error::Error>> {
    let (key, value) = match input.split_once('=') {
        Some((key, value)) => (key, Some(value)),
        None => (input, None),
    };
    if key.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "key cannot be empty").into());
    }
    Ok((key, value))
}

fn normalize_value(key: &Key, raw: &str) -> Result<String, Box<dyn std::error::Error>> {
    match key.key_type {
        KeyType::Text | KeyType::Taxonomy => Ok(raw.to_owned()),
        KeyType::Integer => raw.parse::<i64>().map(|value| value.to_string()).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("'{raw}' is not a valid integer for key '{}'", key.name),
            )
            .into()
        }),
        KeyType::Number => {
            let value = raw.parse::<f64>().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("'{raw}' is not a valid number for key '{}'", key.name),
                )
            })?;
            if !value.is_finite() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("'{raw}' is not a finite number for key '{}'", key.name),
                )
                .into());
            }
            Ok(value.to_string())
        }
        KeyType::Boolean => match raw {
            "true" | "false" => Ok(raw.to_owned()),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("boolean key '{}' accepts only true or false", key.name),
            )
            .into()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::{self, Cardinality, KeyType};

    #[test]
    fn scalar_properties_follow_cardinality() -> Result<(), Box<dyn std::error::Error>> {
        let connection = crate::db::memory()?;
        let bookmark_id = crate::bookmark::add(&connection, "https://example.com")?;
        key::add(&connection, "state", KeyType::Text, Cardinality::Single)?;
        key::add(&connection, "topic", KeyType::Text, Cardinality::Multi)?;

        set(&connection, bookmark_id, "state=reading")?;
        set(&connection, bookmark_id, "state=finished")?;
        set(&connection, bookmark_id, "topic=vulkan")?;
        set(&connection, bookmark_id, "topic=graphics")?;

        let state = resolve_matches(&connection, &["state=finished".into()])?;
        let old_state = resolve_matches(&connection, &["state=reading".into()])?;
        let vulkan = resolve_matches(&connection, &["topic=vulkan".into()])?;
        let graphics = resolve_matches(&connection, &["topic=graphics".into()])?;

        assert!(bookmark_matches(&connection, bookmark_id, &state[0])?);
        assert!(!bookmark_matches(&connection, bookmark_id, &old_state[0])?);
        assert!(bookmark_matches(&connection, bookmark_id, &vulkan[0])?);
        assert!(bookmark_matches(&connection, bookmark_id, &graphics[0])?);

        unset(&connection, bookmark_id, "topic=vulkan")?;
        assert!(!bookmark_matches(&connection, bookmark_id, &vulkan[0])?);
        assert!(bookmark_matches(&connection, bookmark_id, &graphics[0])?);

        unset(&connection, bookmark_id, "topic")?;
        assert!(!bookmark_matches(&connection, bookmark_id, &graphics[0])?);
        Ok(())
    }

    #[test]
    fn boolean_accepts_only_true_or_false() -> Result<(), Box<dyn std::error::Error>> {
        let connection = crate::db::memory()?;
        let bookmark_id = crate::bookmark::add(&connection, "https://example.com")?;
        key::add(&connection, "favorite", KeyType::Boolean, Cardinality::Single)?;

        assert!(set(&connection, bookmark_id, "favorite=true").is_ok());
        assert!(set(&connection, bookmark_id, "favorite=1").is_err());
        assert!(set(&connection, bookmark_id, "favorite=True").is_err());
        Ok(())
    }

    #[test]
    fn taxonomy_values_are_entities() -> Result<(), Box<dyn std::error::Error>> {
        let connection = crate::db::memory()?;
        let bookmark_id = crate::bookmark::add(&connection, "https://example.com")?;
        key::add(&connection, "topic", KeyType::Taxonomy, Cardinality::Multi)?;

        set(&connection, bookmark_id, "topic=vulkan")?;
        set(&connection, bookmark_id, "topic=vulkan")?;

        let node_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM taxonomy_nodes WHERE name = 'vulkan' AND kind = 'value'",
            [],
            |row| row.get(0),
        )?;
        let property_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM taxonomy_properties WHERE bookmark_id = ?1",
            [bookmark_id],
            |row| row.get(0),
        )?;

        assert_eq!(node_count, 1);
        assert_eq!(property_count, 1);
        Ok(())
    }
}
