use crate::models::node::Node;
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
}

/// Load all nodes and links from the database, rebuilding the in-memory Vec.
/// Returns (nodes, id_to_idx) so callers can map a stored DB id back to a Vec index.
pub fn load(conn: &Connection) -> Result<(Vec<Node>, HashMap<i64, usize>)> {
    // Load nodes ordered so parents always appear before children and siblings
    // are in their stored sort order.
    let mut stmt = conn.prepare(
        "SELECT id, label, parent_id, sort_key, collapsed, world_x, world_y
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

    tx.execute("DELETE FROM node_tags", [])?;
    tx.execute("DELETE FROM links", [])?;
    tx.execute("DELETE FROM nodes", [])?;

    let order = topo_order(nodes);
    let mut idx_to_id: Vec<i64> = vec![0; nodes.len()];

    for &i in &order {
        let node = &nodes[i];
        let parent_id: Option<i64> = node.parent.map(|p| idx_to_id[p]);
        let sort_key: i64 = node.parent
            .and_then(|p| nodes[p].children.iter().position(|&c| c == i))
            .unwrap_or(0) as i64;

        tx.execute(
            "INSERT INTO nodes (label, parent_id, sort_key, collapsed, world_x, world_y)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                node.label,
                parent_id,
                sort_key,
                node.collapsed as i32,
                node.world_x,
                node.world_y,
            ],
        )?;
        idx_to_id[i] = tx.last_insert_rowid();

        for (key, value) in &node.tags {
            tx.execute(
                "INSERT INTO node_tags (node_id, tag_key, tag_value) VALUES (?1, ?2, ?3)",
                params![idx_to_id[i], key, value],
            )?;
        }
    }

    for (i, node) in nodes.iter().enumerate() {
        for &tgt in &node.links {
            if tgt < nodes.len() {
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
    let roots: Vec<usize> = (0..nodes.len()).filter(|&i| nodes[i].parent.is_none()).collect();
    for root in roots {
        dfs(nodes, root, &mut out);
    }
    // Catch any nodes not reachable from roots (detached, shouldn't happen but be safe).
    let in_out: std::collections::HashSet<usize> = out.iter().copied().collect();
    for i in 0..nodes.len() {
        if !in_out.contains(&i) { out.push(i); }
    }
    out
}

fn dfs(nodes: &[Node], node: usize, out: &mut Vec<usize>) {
    out.push(node);
    for &child in &nodes[node].children {
        dfs(nodes, child, out);
    }
}
