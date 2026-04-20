use crate::models::archive_note::ArchiveNote;
use rusqlite::{Connection, Result, params};

pub fn load_notes(conn: &Connection) -> Result<Vec<ArchiveNote>> {
    let mut stmt = conn.prepare("SELECT id, title, content FROM notes ORDER BY id ASC")?;
    let notes: Vec<ArchiveNote> = stmt.query_map([], |row| {
        Ok(ArchiveNote {
            id:      Some(row.get(0)?),
            title:   row.get(1)?,
            content: row.get(2)?,
        })
    })?.collect::<Result<_>>()?;
    Ok(notes)
}

pub fn save_notes(conn: &Connection, notes: &[ArchiveNote]) -> Result<()> {
    conn.execute("DELETE FROM notes", [])?;
    for n in notes {
        conn.execute(
            "INSERT INTO notes (title, content) VALUES (?1, ?2)",
            params![n.title, n.content],
        )?;
    }
    Ok(())
}
