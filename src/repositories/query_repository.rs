use crate::models::query::StatusQuery;
use rusqlite::{Connection, Result, params};

pub fn load_queries(conn: &Connection) -> Result<Vec<StatusQuery>> {
    let mut stmt = conn.prepare("SELECT id, name, logic FROM queries ORDER BY id ASC")?;
    let queries: Vec<StatusQuery> = stmt.query_map([], |row| {
        Ok(StatusQuery {
            id:    Some(row.get(0)?),
            name:  row.get(1)?,
            logic: row.get(2)?,
        })
    })?.collect::<Result<_>>()?;
    Ok(queries)
}

pub fn save_queries(conn: &Connection, queries: &[StatusQuery]) -> Result<()> {
    // For simplicity, we just clear and rewrite since it's a small list.
    conn.execute("DELETE FROM queries", [])?;
    for q in queries {
        conn.execute(
            "INSERT INTO queries (name, logic) VALUES (?1, ?2)",
            params![q.name, q.logic],
        )?;
    }
    Ok(())
}
