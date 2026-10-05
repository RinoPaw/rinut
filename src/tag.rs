use std::{
    collections::{HashMap, HashSet},
    io,
};

use rusqlite::{params, Connection, OptionalExtension};

#[derive(Debug, Clone)]
pub struct Tag {
    pub id: i64,
    pub name: String,
}

#[derive(Debug)]
pub struct Selector {
    include: bool,
    tag_id: i64,
}

pub fn add(conn: &Connection, name: &str) -> Result<i64, Box<dyn std::error::Error>> {
    validate_name(name)?;
    conn.execute("INSERT INTO tags (name) VALUES (?1)", [name])?;
    Ok(conn.last_insert_rowid())
}

pub fn get(conn: &Connection, name: &str) -> rusqlite::Result<Option<Tag>> {
    conn.query_row(
        "SELECT id, name FROM tags WHERE name = ?1",
        [name],
        |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        },
    )
    .optional()
}

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<Tag>> {
    let mut statement = conn.prepare("SELECT id, name FROM tags ORDER BY name")?;
    let rows = statement.query_map([], |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    })?;
    rows.collect()
}

pub fn rename(
    conn: &Connection,
    name: &str,
    new_name: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    validate_name(new_name)?;
    Ok(conn.execute(
        "UPDATE tags SET name = ?2 WHERE name = ?1",
        params![name, new_name],
    )? > 0)
}

pub fn delete(conn: &Connection, name: &str) -> rusqlite::Result<bool> {
    Ok(conn.execute("DELETE FROM tags WHERE name = ?1", [name])? > 0)
}

pub fn assign(
    conn: &Connection,
    bookmark_id: i64,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let tag = require(conn, name)?;
    conn.execute(
        "INSERT OR IGNORE INTO bookmark_tags (bookmark_id, tag_id) VALUES (?1, ?2)",
        params![bookmark_id, tag.id],
    )?;
    Ok(())
}

pub fn unassign(
    conn: &Connection,
    bookmark_id: i64,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(tag) = get(conn, name)? else {
        return Ok(());
    };
    conn.execute(
        "DELETE FROM bookmark_tags WHERE bookmark_id = ?1 AND tag_id = ?2",
        params![bookmark_id, tag.id],
    )?;
    Ok(())
}

pub fn list_for_bookmark(conn: &Connection, bookmark_id: i64) -> rusqlite::Result<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT t.name
         FROM bookmark_tags bt
         JOIN tags t ON t.id = bt.tag_id
         WHERE bt.bookmark_id = ?1
         ORDER BY t.name",
    )?;
    let rows = statement.query_map([bookmark_id], |row| row.get(0))?;
    rows.collect()
}

pub fn link(
    conn: &Connection,
    parent: &str,
    child: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let parent = require(conn, parent)?;
    let child = require(conn, child)?;
    if parent.id == child.id || reachable(conn, child.id, parent.id)? {
        return Err(invalid_input(format!(
            "linking '{}' -> '{}' would create a cycle",
            parent.name, child.name
        )));
    }
    conn.execute(
        "INSERT OR IGNORE INTO tag_edges (parent_id, child_id) VALUES (?1, ?2)",
        params![parent.id, child.id],
    )?;
    Ok(())
}

pub fn unlink(
    conn: &Connection,
    parent: &str,
    child: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let parent = require(conn, parent)?;
    let child = require(conn, child)?;
    Ok(conn.execute(
        "DELETE FROM tag_edges WHERE parent_id = ?1 AND child_id = ?2",
        params![parent.id, child.id],
    )? > 0)
}

pub fn parents(conn: &Connection, tag_id: i64) -> rusqlite::Result<Vec<String>> {
    relationship_names(conn, tag_id, true)
}

pub fn children(conn: &Connection, tag_id: i64) -> rusqlite::Result<Vec<String>> {
    relationship_names(conn, tag_id, false)
}

pub fn tree_lines(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let tags = list(conn)?;
    let mut by_id = HashMap::new();
    for tag in tags {
        by_id.insert(tag.id, tag.name);
    }

    let mut children_by_parent: HashMap<i64, Vec<i64>> = HashMap::new();
    let mut child_ids = HashSet::new();
    let mut statement = conn.prepare("SELECT parent_id, child_id FROM tag_edges")?;
    let edges = statement.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
    })?;
    for edge in edges {
        let (parent, child) = edge?;
        children_by_parent.entry(parent).or_default().push(child);
        child_ids.insert(child);
    }

    for children in children_by_parent.values_mut() {
        children.sort_by_key(|id| by_id.get(id).cloned().unwrap_or_default());
    }

    let mut roots: Vec<i64> = by_id
        .keys()
        .copied()
        .filter(|id| !child_ids.contains(id))
        .collect();
    roots.sort_by_key(|id| by_id.get(id).cloned().unwrap_or_default());

    let mut lines = Vec::new();
    for root in roots {
        append_tree_lines(root, 0, &by_id, &children_by_parent, &mut lines);
    }
    Ok(lines)
}

pub fn resolve_selectors(
    conn: &Connection,
    raw: &[String],
) -> Result<Vec<Selector>, Box<dyn std::error::Error>> {
    raw.iter()
        .map(|value| {
            let (include, name) = match value.as_bytes().first() {
                Some(b'+') => (true, &value[1..]),
                Some(b'-') => (false, &value[1..]),
                _ => {
                    return Err(invalid_input(format!(
                        "expected +TAG or -TAG, got '{value}'"
                    )))
                }
            };
            if name.is_empty() {
                return Err(invalid_input("tag selector cannot be empty"));
            }
            let tag = require(conn, name)?;
            Ok(Selector {
                include,
                tag_id: tag.id,
            })
        })
        .collect()
}

pub fn bookmark_matches(
    conn: &Connection,
    bookmark_id: i64,
    selectors: &[Selector],
) -> rusqlite::Result<bool> {
    for selector in selectors {
        let has = has_tag_or_descendant(conn, bookmark_id, selector.tag_id)?;
        if selector.include != has {
            return Ok(false);
        }
    }
    Ok(true)
}

fn has_tag_or_descendant(
    conn: &Connection,
    bookmark_id: i64,
    tag_id: i64,
) -> rusqlite::Result<bool> {
    conn.query_row(
        "WITH RECURSIVE descendants(id) AS (
             SELECT ?2
             UNION
             SELECT e.child_id
             FROM tag_edges e
             JOIN descendants d ON e.parent_id = d.id
         )
         SELECT EXISTS(
             SELECT 1 FROM bookmark_tags bt
             WHERE bt.bookmark_id = ?1
               AND bt.tag_id IN (SELECT id FROM descendants)
         )",
        params![bookmark_id, tag_id],
        |row| row.get(0),
    )
}

fn reachable(conn: &Connection, from: i64, target: i64) -> rusqlite::Result<bool> {
    conn.query_row(
        "WITH RECURSIVE descendants(id) AS (
             SELECT ?1
             UNION
             SELECT e.child_id
             FROM tag_edges e
             JOIN descendants d ON e.parent_id = d.id
         )
         SELECT EXISTS(SELECT 1 FROM descendants WHERE id = ?2)",
        params![from, target],
        |row| row.get(0),
    )
}

fn relationship_names(
    conn: &Connection,
    tag_id: i64,
    parents: bool,
) -> rusqlite::Result<Vec<String>> {
    let sql = if parents {
        "SELECT t.name FROM tag_edges e JOIN tags t ON t.id = e.parent_id WHERE e.child_id = ?1 ORDER BY t.name"
    } else {
        "SELECT t.name FROM tag_edges e JOIN tags t ON t.id = e.child_id WHERE e.parent_id = ?1 ORDER BY t.name"
    };
    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map([tag_id], |row| row.get(0))?;
    rows.collect()
}

fn append_tree_lines(
    id: i64,
    depth: usize,
    names: &HashMap<i64, String>,
    children: &HashMap<i64, Vec<i64>>,
    lines: &mut Vec<String>,
) {
    if let Some(name) = names.get(&id) {
        lines.push(format!("{}{}", "  ".repeat(depth), name));
    }
    if let Some(child_ids) = children.get(&id) {
        for child in child_ids {
            append_tree_lines(*child, depth + 1, names, children, lines);
        }
    }
}

fn require(conn: &Connection, name: &str) -> Result<Tag, Box<dyn std::error::Error>> {
    get(conn, name)?.ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, format!("tag '{name}' not found")).into()
    })
}

fn validate_name(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if name.is_empty() {
        return Err(invalid_input("tag name cannot be empty"));
    }
    if !name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '+' | '-' | '.' | '_'))
    {
        return Err(invalid_input(format!(
            "tag '{name}' contains unsupported characters"
        )));
    }
    Ok(())
}

fn invalid_input(message: impl Into<String>) -> Box<dyn std::error::Error> {
    io::Error::new(io::ErrorKind::InvalidInput, message.into()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_assign_and_unassign() -> Result<(), Box<dyn std::error::Error>> {
        let connection = crate::db::memory()?;
        let bookmark_id = crate::bookmark::add(&connection, "https://example.com")?;
        add(&connection, "vulkan")?;
        assign(&connection, bookmark_id, "vulkan")?;
        assert_eq!(list_for_bookmark(&connection, bookmark_id)?, vec!["vulkan"]);
        unassign(&connection, bookmark_id, "vulkan")?;
        assert!(list_for_bookmark(&connection, bookmark_id)?.is_empty());
        Ok(())
    }

    #[test]
    fn parent_selector_matches_descendant_assignment() -> Result<(), Box<dyn std::error::Error>> {
        let connection = crate::db::memory()?;
        let bookmark_id = crate::bookmark::add(&connection, "https://example.com")?;
        add(&connection, "computer-science")?;
        add(&connection, "computer-graphics")?;
        link(&connection, "computer-science", "computer-graphics")?;
        assign(&connection, bookmark_id, "computer-graphics")?;

        let include = resolve_selectors(&connection, &["+computer-science".into()])?;
        let exclude = resolve_selectors(&connection, &["-computer-science".into()])?;
        assert!(bookmark_matches(&connection, bookmark_id, &include)?);
        assert!(!bookmark_matches(&connection, bookmark_id, &exclude)?);
        Ok(())
    }

    #[test]
    fn non_leaf_tags_can_be_assigned() -> Result<(), Box<dyn std::error::Error>> {
        let connection = crate::db::memory()?;
        let bookmark_id = crate::bookmark::add(&connection, "https://example.com")?;
        add(&connection, "technology")?;
        add(&connection, "computer-science")?;
        link(&connection, "technology", "computer-science")?;
        assign(&connection, bookmark_id, "technology")?;
        assert_eq!(list_for_bookmark(&connection, bookmark_id)?, vec!["technology"]);
        Ok(())
    }

    #[test]
    fn cycles_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let connection = crate::db::memory()?;
        add(&connection, "a")?;
        add(&connection, "b")?;
        add(&connection, "c")?;
        link(&connection, "a", "b")?;
        link(&connection, "b", "c")?;
        assert!(link(&connection, "c", "a").is_err());
        Ok(())
    }
}
