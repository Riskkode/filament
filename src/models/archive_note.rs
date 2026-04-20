use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct ArchiveNote {
    pub id: Option<i64>,
    pub title: String,
    pub content: String,
}
