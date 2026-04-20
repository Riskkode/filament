use crate::models::node::{Node, TimeTag};
use rusqlite::{Connection, Result, params};
use std::collections::HashMap;

// ── Load ──────────────────────────────────────────────────────────────────────

/// Row returned from the nodes query before Vec-index mapping.
struct DbRow {
    id:        i64,
    label:     String,
    parent_id: Option<i64>,
    #[allow(dead_code)]
    sort_key:  i64,
    collapsed: bool,
    world_x:   i32,
    world_y:   i32,
    is_managed_note: bool,
}

/// Load all nodes and links from the database, rebuilding the in-memory Vec.
/// Returns (nodes, id_to_idx) so callers can map a stored DB id back to a Vec index.
pub fn load(conn: &Connection) -> Result<(Vec<Node>, HashMap<i64, usize>)> {
    // Load nodes ordered so parents always appear before children and siblings
    // are in their stored sort order.
    let mut stmt = conn.prepare(
        "SELECT id, label, parent_id, sort_key, collapsed, world_x, world_y, is_managed_note
         FROM nodes
         ORDER BY COALESCE(parent_id, 0), sort_key"
    )?;

    let rows: Vec<DbRow> = stmt.query_map([], |row| {
        Ok(DbRow {
            id:        row.get(0)?,
            label:     row.get(1)?,
            parent_id: row.get(2)?,
            sort_key:  row.get(3)?,
            collapsed: row.get::<_, i32>(4)? != 0,
            world_x:   row.get(5)?,
            world_y:   row.get(6)?,
            is_managed_note: row.get::<_, i32>(7)? != 0,
        })
    })?.collect::<Result<_>>()?;

    // Assign Vec indices in load order and build id→index map.
    let id_to_idx: HashMap<i64, usize> = rows.iter().enumerate()
        .map(|(i, r)| (r.id, i))
        .collect();

    let mut nodes: Vec<Node> = rows.iter().map(|r| Node {
        label:     r.label.clone(),
        parent:    r.parent_id.and_then(|pid| id_to_idx.get(&pid).copied()),
        children:  vec![],
        links:     vec![],
        collapsed: r.collapsed,
        row:       usize::MAX,
        world_x:   r.world_x,
        world_y:   r.world_y,
        world_x_end: 0,
        tags:      HashMap::new(),
        times:     HashMap::new(),
        is_managed_note: r.is_managed_note,
        managed: None,
    }).collect();

    // Build children lists (rows were ordered by parent_id, sort_key so
    // children are pushed in their correct sibling order).
    for (i, row) in rows.iter().enumerate() {
        if let Some(pid) = row.parent_id {
            if let Some(&parent_idx) = id_to_idx.get(&pid) {
                nodes[parent_idx].children.push(i);
            }
        }
    }

    // Load links.
    let mut link_stmt = conn.prepare("SELECT src_id, tgt_id FROM links")?;
    let link_pairs: Vec<(i64, i64)> = link_stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?.collect::<Result<_>>()?;

    for (src_id, tgt_id) in link_pairs {
        if let (Some(&si), Some(&ti)) = (id_to_idx.get(&src_id), id_to_idx.get(&tgt_id)) {
            nodes[si].links.push(ti);
        }
    }

    // Load tags.
    let mut tag_stmt = conn.prepare("SELECT node_id, tag_key, tag_value FROM node_tags")?;
    let tags_list: Vec<(i64, String, String)> = tag_stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?.collect::<Result<_>>()?;

    for (node_id, key, value) in tags_list {
        if let Some(&idx) = id_to_idx.get(&node_id) {
            nodes[idx].tags.insert(key, value);
        }
    }

    // Load times.
    let mut time_stmt = conn.prepare("SELECT node_id, time_type, time_value, time_pattern FROM node_times")?;
    let times_list: Vec<(i64, String, i64, Option<String>)> = time_stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    })?.collect::<Result<_>>()?;

    for (node_id, key, value, pattern) in times_list {
        if let Some(&idx) = id_to_idx.get(&node_id) {
            nodes[idx].times.insert(key, TimeTag { timestamp: value, pattern });
        }
    }

    Ok((nodes, id_to_idx))
}

// ── Save ──────────────────────────────────────────────────────────────────────

/// Replace the entire canvas with the current node list. Runs inside a single
/// transaction so the database is never left in a partial state.
///
/// Returns a mapping from Vec index → new DB row-id so callers can persist
/// the selected node's id in project settings.
pub fn save(conn: &mut Connection, nodes: &[Node]) -> Result<Vec<i64>> {
    let tx = conn.transaction()?;

    // Deleting nodes with self-referencing foreign keys can be tricky.
    // Disable FKs for this connection (within the transaction).
    tx.execute_batch("PRAGMA foreign_keys=OFF;")?;

    tx.execute("DELETE FROM node_times", [])?;
    tx.execute("DELETE FROM node_tags", [])?;
    tx.execute("DELETE FROM links", [])?;
    tx.execute("DELETE FROM nodes", [])?;

    let order = topo_order(nodes);
    let mut idx_to_id: Vec<i64> = vec![0; nodes.len()];

    for &i in &order {
        let node = &nodes[i];
        if node.managed.is_some() { continue; }

        let parent_id: Option<i64> = node.parent.and_then(|p| {
            let id = idx_to_id[p];
            if id == 0 { None } else { Some(id) }
        });
        let sort_key: i64 = node.parent
            .and_then(|p| nodes[p].children.iter().position(|&c| c == i))
            .unwrap_or(0) as i64;

        tx.execute(
            "INSERT INTO nodes (label, parent_id, sort_key, collapsed, world_x, world_y, is_managed_note)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                node.label,
                parent_id,
                sort_key,
                node.collapsed as i32,
                node.world_x,
                node.world_y,
                node.is_managed_note as i32,
            ],
        )?;
        idx_to_id[i] = tx.last_insert_rowid();

        for (key, value) in &node.tags {
            tx.execute(
                "INSERT INTO node_tags (node_id, tag_key, tag_value) VALUES (?1, ?2, ?3)",
                params![idx_to_id[i], key, value],
            )?;
        }

        for (key, value) in &node.times {
            tx.execute(
                "INSERT INTO node_times (node_id, time_type, time_value, time_pattern) VALUES (?1, ?2, ?3, ?4)",
                params![idx_to_id[i], key, value.timestamp, value.pattern],
            )?;
        }
    }

    for (i, node) in nodes.iter().enumerate() {
        if node.managed.is_some() || idx_to_id[i] == 0 { continue; }
        for &tgt in &node.links {
            if tgt < nodes.len() && nodes[tgt].managed.is_none() && idx_to_id[tgt] != 0 {
                tx.execute(
                    "INSERT OR IGNORE INTO links (src_id, tgt_id) VALUES (?1, ?2)",
                    params![idx_to_id[i], idx_to_id[tgt]],
                )?;
            }
        }
    }

    tx.execute_batch("PRAGMA foreign_keys=ON;")?;
    tx.commit()?;
    Ok(idx_to_id)
}

// ── History ───────────────────────────────────────────────────────────────────

pub fn push_history(conn: &Connection, nodes: &[Node]) -> Result<()> {
    let json = serde_json::to_string(nodes).unwrap_or_else(|_| "[]".to_string());
    conn.execute(
        "INSERT INTO history (snapshot) VALUES (?1)",
        params![json],
    )?;

    // Keep only last 100
    conn.execute(
        "DELETE FROM history WHERE id NOT IN (
            SELECT id FROM history ORDER BY id DESC LIMIT 100
        )",
        [],
    )?;

    Ok(())
}

pub fn load_history(conn: &Connection) -> Result<Vec<Vec<Node>>> {
    let mut stmt = conn.prepare("SELECT snapshot FROM history ORDER BY id ASC")?;
    let history: Vec<Vec<Node>> = stmt.query_map([], |row| {
        let json: String = row.get(0)?;
        Ok(serde_json::from_str(&json).unwrap_or_default())
    })?.collect::<Result<_>>()?;
    Ok(history)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Depth-first topological order so parents are always inserted before children.
fn topo_order(nodes: &[Node]) -> Vec<usize> {
    let mut out = Vec::with_capacity(nodes.len());
    let roots: Vec<usize> = (0..nodes.len())
        .filter(|&i| nodes[i].parent.is_none() && nodes[i].managed.is_none())
        .collect();
    for root in roots {
        dfs(nodes, root, &mut out);
    }
    // Catch any nodes not reachable from roots (detached, shouldn't happen but be safe).
    let in_out: std::collections::HashSet<usize> = out.iter().copied().collect();
    for i in 0..nodes.len() {
        if !in_out.contains(&i) && nodes[i].managed.is_none() { out.push(i); }
    }
    out
}

fn dfs(nodes: &[Node], node: usize, out: &mut Vec<usize>) {
    if nodes[node].managed.is_some() { return; }
    out.push(node);
    for &child in &nodes[node].children {
        dfs(nodes, child, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::node::ManagedNodeType;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("
            CREATE TABLE nodes (
                id               INTEGER PRIMARY KEY AUTOINCREMENT,
                label            TEXT    NOT NULL,
                parent_id        INTEGER REFERENCES nodes(id),
                sort_key         INTEGER NOT NULL DEFAULT 0,
                collapsed        INTEGER NOT NULL DEFAULT 0,
                world_x          INTEGER NOT NULL DEFAULT 0,
                world_y          INTEGER NOT NULL DEFAULT 0,
                is_managed_note  INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE links (
                src_id INTEGER NOT NULL REFERENCES nodes(id),
                tgt_id INTEGER NOT NULL REFERENCES nodes(id),
                PRIMARY KEY (src_id, tgt_id)
            );
            CREATE TABLE history (
                id        INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                snapshot  TEXT NOT NULL
            );
            CREATE TABLE node_tags (
                node_id    INTEGER NOT NULL REFERENCES nodes(id),
                tag_key    TEXT NOT NULL,
                tag_value  TEXT NOT NULL,
                PRIMARY KEY (node_id, tag_key)
            );
            CREATE TABLE node_times (
                node_id      INTEGER NOT NULL REFERENCES nodes(id),
                time_type    TEXT NOT NULL,
                time_value   INTEGER NOT NULL,
                time_pattern TEXT,
                PRIMARY KEY (node_id, time_type)
            );
        ").unwrap();
        conn
    }

    #[test]
    fn test_managed_nodes_not_saved() {
        let mut conn = setup_test_db();
        let nodes = vec![
            Node {
                label: "Root".to_string(),
                parent: None,
                children: vec![1],
                links: vec![],
                collapsed: false,
                row: 0,
                world_x: 0,
                world_y: 0,
                world_x_end: 0,
                tags: HashMap::new(),
                times: HashMap::new(),
                is_managed_note: false,
                managed: None,
            },
            Node {
                label: "Managed".to_string(),
                parent: Some(0),
                children: vec![],
                links: vec![],
                collapsed: false,
                row: 0,
                world_x: 0,
                world_y: 0,
                world_x_end: 0,
                tags: HashMap::new(),
                times: HashMap::new(),
                is_managed_note: false,
                managed: Some(ManagedNodeType::TimeGroup),
            },
        ];

        save(&mut conn, &nodes).unwrap();

        let (loaded_nodes, _) = load(&conn).unwrap();
        assert_eq!(loaded_nodes.len(), 1);
        assert_eq!(loaded_nodes[0].label, "Root");
    }

    #[test]
    fn test_managed_nodes_in_history() {
        let conn = setup_test_db();
        let nodes = vec![
            Node {
                label: "Managed".to_string(),
                parent: None,
                children: vec![],
                links: vec![],
                collapsed: false,
                row: 0,
                world_x: 0,
                world_y: 0,
                world_x_end: 0,
                tags: HashMap::new(),
                times: HashMap::new(),
                is_managed_note: false,
                managed: Some(ManagedNodeType::TimeGroup),
            },
        ];

        push_history(&conn, &nodes).unwrap();
        let history = load_history(&conn).unwrap();
        
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].len(), 1);
        assert!(history[0][0].managed.is_some());
        assert_eq!(history[0][0].managed, Some(ManagedNodeType::TimeGroup));
    }
}
