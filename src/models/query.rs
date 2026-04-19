use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct StatusQuery {
    pub id:    Option<i64>,
    pub name:  String,
    pub logic: String,
}
