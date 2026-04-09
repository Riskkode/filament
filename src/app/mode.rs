pub enum Mode {
    /// Ground state. hjkl moves the world cursor; everything else is a
    /// sub-mode entered from here.
    Canvas   { state: CanvasState },
    Input    { action: InputAction, buf: String, cursor: usize },
    Reparent { subject: usize, orig_parent: Option<usize>, orig_pos: usize, cursor: usize },
}

#[derive(Clone)]
pub enum InputAction {
    InsertChild { parent: usize },
    EditLabel   { node: usize },
    Overwrite   { node: usize },
}

pub enum CanvasState {
    Browse,
    New  { buf: String, text_cursor: usize },
    Pick { origin_id: usize, origin_x: i32, origin_y: i32 },
    /// Linking: navigate cursor to a target; Enter toggles the link, Esc cancels.
    Link { origin_id: usize },
}
