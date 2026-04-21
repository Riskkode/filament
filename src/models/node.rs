use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize)]
pub struct TimeTag {
    pub timestamp: i64,
    pub pattern:   Option<String>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum ManagedNodeType {
    TimeGroup,
    TimeTag { key: String },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Node {
    pub label:     String,
    pub parent:    Option<usize>,
    pub children:  Vec<usize>,
    /// Outgoing logical links to other nodes (by index). A node may have any
    /// number of outgoing links to distinct targets.
    pub links:     Vec<usize>,
    pub collapsed: bool,
    /// Row in the combined visible list (`usize::MAX` = hidden).
    pub row:       usize,
    /// World-space anchor. Root nodes are freely positioned; children derive
    /// their position from `(root.world_x, root.world_y + local_row)`.
    pub world_x:     i32,
    pub world_y:     i32,
    /// Right anchor: world_x + depth*2 + 2 + label.chars().count().
    /// Recomputed every frame by recompute_layout; initialise to 0.
    pub world_x_end: i32,
    /// Key-value metadata tags (e.g., "status" => "todo").
    pub tags:      HashMap<String, String>,
    /// Time tags (e.g., "deadline" => TimeTag).
    pub times:     HashMap<String, TimeTag>,
    /// If true, this node represents a link to an Archive note (label is the title).
    pub is_managed_note: bool,
    /// If set, this node is a virtual managed subnode that represents part of another node's metadata.
    pub managed: Option<ManagedNodeType>,
}
