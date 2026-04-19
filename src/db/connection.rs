use rusqlite::{Connection, Result};
use std::path::Path;

/// Open (or create) the SQLite canvas database at the given path and ensure
/// the schema exists. Returns a ready-to-use connection.
pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                // Since rusqlite::Error doesn't have a direct Io variant in all versions,
                // we'll let it fail at Connection::open if we can't create the dir,
                // or we could use a custom error. For now, let's just try to create it
                // and ignore the error here, as Connection::open will provide a better
                // sqlite-specific error if it still fails.
                // Alternatively, we can use rusqlite::Error::InvalidPath if it's just about the path.
                let _ = e;
                rusqlite::Error::InvalidPath(parent.to_path_buf())
            })?;
        }
    }
    let conn = Connection::open(path)?;

    // Enable WAL for better concurrent read performance and crash safety.
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS nodes (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            label     TEXT    NOT NULL,
            parent_id INTEGER REFERENCES nodes(id),
            sort_key  INTEGER NOT NULL DEFAULT 0,
            collapsed INTEGER NOT NULL DEFAULT 0,
            world_x   INTEGER NOT NULL DEFAULT 0,
            world_y   INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS links (
            src_id INTEGER NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
            tgt_id INTEGER NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
            PRIMARY KEY (src_id, tgt_id)
        );

        CREATE TABLE IF NOT EXISTS history (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
            snapshot  TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS node_tags (
            node_id    INTEGER NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
            tag_key    TEXT NOT NULL,
            tag_value  TEXT NOT NULL,
            PRIMARY KEY (node_id, tag_key)
        );

        CREATE TABLE IF NOT EXISTS node_times (
            node_id      INTEGER NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
            time_type    TEXT NOT NULL,
            time_value   INTEGER NOT NULL,
            time_pattern TEXT,
            PRIMARY KEY (node_id, time_type)
        );
    ")?;

    // Migration: Add time_pattern column if it doesn't exist.
    let has_pattern: i32 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('node_times') WHERE name='time_pattern'", 
        [], |r| r.get(0)
    ).unwrap_or(0);
    if has_pattern == 0 {
        let _ = conn.execute("ALTER TABLE node_times ADD COLUMN time_pattern TEXT", []);
    }

    Ok(conn)
}
