use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum NoteType {
    Quick,
    Reference,
}

impl Default for NoteType {
    fn default() -> Self { NoteType::Quick }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ArchiveNote {
    pub id: Option<i64>,
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub note_type: NoteType,
}
