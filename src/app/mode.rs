pub enum Mode {
    Normal,
    Insert   { parent: usize, buf: String, cursor: usize },
    Confirm  { target: usize },
    Reparent { subject: usize, orig_parent: Option<usize>, orig_pos: usize, cursor: usize },
    /// Free-roam canvas mode: hjkl moves the world cursor.
    Nodes    { cursor_x: i32, cursor_y: i32 },
    /// Typing a label for a brand-new root node at the cursor position.
    NodeNew  { cursor_x: i32, cursor_y: i32, buf: String, text_cursor: usize },
    /// Carrying a picked node to a new canvas location.
    NodePick { cursor_x: i32, cursor_y: i32, origin_id: usize, origin_x: i32, origin_y: i32 },
}
