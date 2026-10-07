use std::io;

use rusqlite::{params, Connection, OptionalExtension};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    pub id: i64,
    pub name: String,
    pub cardinality: String,
}

pub fn add(
    conn: &Connection,
    name: &str,
    cardinality: &str,
) -> Result<i64, Box<dyn std::error::Error>> {
    validate_name(name)?;
    validate_cardinality(cardinality)?;
    conn.execute(
        "INSERT INTO keys (name, cardinality) VALUES (?1, ?2)",
        params![name, cardinality],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get(conn: &Connection, name: &str) -> rusqlite::Result<Option<Key>> {
    conn.query_row(
        "SELECT id, name, cardinality FROM keys WHERE name = ?1",
        [name],
        |row| {
            Ok(Key {
                id: row.get(0)?,
                name: row.get(1)?,
                cardinality: row.get(2)?,
            })
        },
    )
    .optional()
}

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<Key>> {
    let mut statement =
        conn.prepare("SELECT id, name, cardinality FROM keys ORDER BY name")?;
    let rows = statement.query_map([], |row| {
        Ok(Key {
            id: row.get(0)?,
            name: row.get(1)?,
            cardinality: row.get(2)?,
        })
    })?;
    rows.collect()
}

pub fn edit(
    conn: &Connection,
    name: &str,
    new_name: Option<&str>,
    cardinality: Option<&str>,
) -> Result<bool, Box<dyn std::error::Error>> {
    let Some(current) = get(conn, name)? else {
        return Ok(false);
    };

    let final_name = new_name.unwrap_or(&current.name);
    let final_cardinality = cardinality.unwrap_or(&current.cardinality);
    validate_name(final_name)?;
    validate_cardinality(final_cardinality)?;

    conn.execute(
        "UPDATE keys SET name = ?2, cardinality = ?3 WHERE id = ?1",
        params![current.id, final_name, final_cardinality],
    )?;
    Ok(true)
}

pub fn delete(conn: &Connection, name: &str) -> rusqlite::Result<bool> {
    Ok(conn.execute("DELETE FROM keys WHERE name = ?1", [name])? > 0)
}

fn validate_name(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if name.is_empty() {
        return Err(invalid_input("key name cannot be empty"));
    }
    if !name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        return Err(invalid_input(format!(
            "key '{name}' contains unsupported characters"
        )));
    }
    Ok(())
}

fn validate_cardinality(value: &str) -> Result<(), Box<dyn std::error::Error>> {
    if matches!(value, "single" | "multi") {
        Ok(())
    } else {
        Err(invalid_input(format!(
            "unsupported cardinality '{value}'; expected single or multi"
        )))
    }
}

fn invalid_input(message: impl Into<String>) -> Box<dyn std::error::Error> {
    io::Error::new(io::ErrorKind::InvalidInput, message.into()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_crud_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let connection = crate::db::memory()?;

        add(&connection, "Source", "single")?;
        add(&connection, "Field", "multi")?;

        assert_eq!(get(&connection, "Source")?.unwrap().cardinality, "single");
        assert_eq!(list(&connection)?.len(), 2);

        assert!(edit(&connection, "Source", Some("Origin"), Some("multi"))?);
        let key = get(&connection, "Origin")?.unwrap();
        assert_eq!(key.cardinality, "multi");

        assert!(delete(&connection, "Origin")?);
        assert!(get(&connection, "Origin")?.is_none());
        Ok(())
    }
}
