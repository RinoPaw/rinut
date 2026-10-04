use std::io;

use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    Text,
    Integer,
    Number,
    Boolean,
    Taxonomy,
}

impl KeyType {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "text" => Some(Self::Text),
            "integer" => Some(Self::Integer),
            "number" => Some(Self::Number),
            "boolean" => Some(Self::Boolean),
            "taxonomy" => Some(Self::Taxonomy),
            _ => None,
        }
    }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cardinality {
    Single,
    Multi,
}

impl Cardinality {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "single" => Some(Self::Single),
            "multi" => Some(Self::Multi),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Multi => "multi",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Key {
    pub id: i64,
    pub name: String,
    pub key_type: KeyType,
    pub cardinality: Cardinality,
}

pub fn add(
    conn: &Connection,
    name: &str,
    key_type: KeyType,
    cardinality: Cardinality,
) -> Result<i64, Box<dyn std::error::Error>> {
    if key_type == KeyType::Boolean && cardinality == Cardinality::Multi {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "boolean keys must have single cardinality",
        )
        .into());
    }

    conn.execute(
        "INSERT INTO property_keys (name, type, cardinality) VALUES (?1, ?2, ?3)",
        params![name, key_type.as_str(), cardinality.as_str()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn list(conn: &Connection) -> Result<Vec<Key>, Box<dyn std::error::Error>> {
    let mut statement = conn.prepare(
        "SELECT id, name, type, cardinality
         FROM property_keys
         ORDER BY name",
    )?;

    let rows = statement.query_map([], |row| {
        let id: i64 = row.get(0)?;
        let name: String = row.get(1)?;
        let key_type: String = row.get(2)?;
        let cardinality: String = row.get(3)?;
        Ok((id, name, key_type, cardinality))
    })?;

    rows.map(|row| {
        let (id, name, key_type, cardinality) = row?;
        key_from_parts(id, name, &key_type, &cardinality)
    })
    .collect()
}

pub fn get(conn: &Connection, name: &str) -> Result<Option<Key>, Box<dyn std::error::Error>> {
    let row = conn
        .query_row(
            "SELECT id, name, type, cardinality
             FROM property_keys
             WHERE name = ?1",
            [name],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?;

    row.map(|(id, name, key_type, cardinality)| {
        key_from_parts(id, name, &key_type, &cardinality)
    })
    .transpose()
}

pub fn rename(conn: &Connection, name: &str, new_name: &str) -> SqlResult<bool> {
    Ok(conn.execute(
        "UPDATE property_keys SET name = ?1 WHERE name = ?2",
        params![new_name, name],
    )? > 0)
}

pub fn delete(conn: &Connection, name: &str) -> SqlResult<bool> {
    Ok(conn.execute("DELETE FROM property_keys WHERE name = ?1", [name])? > 0)
}

fn key_from_parts(
    id: i64,
    name: String,
    key_type: &str,
    cardinality: &str,
) -> Result<Key, Box<dyn std::error::Error>> {
    let key_type = KeyType::parse(key_type).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, format!("unknown key type '{key_type}'"))
    })?;
    let cardinality = Cardinality::parse(cardinality).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unknown cardinality '{cardinality}'"),
        )
    })?;

    Ok(Key {
        id,
        name,
        key_type,
        cardinality,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
        let connection = crate::db::memory()?;
        add(&connection, "topic", KeyType::Taxonomy, Cardinality::Multi)?;

        let key = get(&connection, "topic")?.expect("key should exist");
        assert_eq!(key.key_type, KeyType::Taxonomy);
        assert_eq!(key.cardinality, Cardinality::Multi);

        assert!(rename(&connection, "topic", "subject")?);
        assert!(get(&connection, "topic")?.is_none());
        assert!(get(&connection, "subject")?.is_some());

        assert!(delete(&connection, "subject")?);
        assert!(get(&connection, "subject")?.is_none());
        Ok(())
    }

    #[test]
    fn boolean_cannot_be_multi() -> Result<(), Box<dyn std::error::Error>> {
        let connection = crate::db::memory()?;
        let result = add(&connection, "flag", KeyType::Boolean, Cardinality::Multi);
        assert!(result.is_err());
        Ok(())
    }
}
