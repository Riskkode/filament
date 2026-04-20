use crate::models::archive_note::{ArchiveNote, NoteType};
use rusqlite::{Connection, Result, params};

pub fn load_notes(conn: &Connection) -> Result<Vec<ArchiveNote>> {
    let mut stmt = conn.prepare("SELECT id, title, content, note_type FROM notes ORDER BY id ASC")?;
    let notes: Vec<ArchiveNote> = stmt.query_map([], |row| {
        let note_type_raw: i32 = row.get(3)?;
        Ok(ArchiveNote {
            id:      Some(row.get(0)?),
            title:   row.get(1)?,
            content: row.get(2)?,
            note_type: if note_type_raw == 1 { NoteType::Reference } else { NoteType::Quick },
        })
    })?.collect::<Result<_>>()?;
    Ok(notes)
}

pub fn save_notes(conn: &Connection, notes: &[ArchiveNote]) -> Result<()> {
    conn.execute("DELETE FROM notes", [])?;
    for n in notes {
        let note_type_raw = if n.note_type == NoteType::Reference { 1 } else { 0 };
        conn.execute(
            "INSERT INTO notes (title, content, note_type) VALUES (?1, ?2, ?3)",
            params![n.title, n.content, note_type_raw],
        )?;
    }
    Ok(())
}
