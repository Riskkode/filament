pub struct Node {
    pub label:     String,
    pub parent:    Option<usize>,
    pub children:  Vec<usize>,
    pub collapsed: bool,
    /// Row in the combined visible list (`usize::MAX` = hidden).
    pub row:       usize,
    /// World-space anchor. Root nodes are freely positioned; children derive
    /// their position from `(root.world_x, root.world_y + local_row)`.
    pub world_x:   i32,
    pub world_y:   i32,
}
