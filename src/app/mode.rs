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

// ── Arrow display settings ────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum ArrowFidelity { Tree, Selected }

pub struct ArrowSettings {
    /// When true: render ALL links in a dim colour first, then highlight
    /// the subset selected by incoming/outgoing fidelity on top.
    pub global:   bool,
    /// Which incoming arrows to highlight (into the tree, or only the selected node).
    pub incoming: ArrowFidelity,
    /// Which outgoing arrows to highlight (from the tree, or only the selected node).
    pub outgoing: ArrowFidelity,
}

impl Default for ArrowSettings {
    fn default() -> Self {
        Self {
            global:   false,
            incoming: ArrowFidelity::Tree,
            outgoing: ArrowFidelity::Tree,
        }
    }
}

// ── Canvas sub-states ─────────────────────────────────────────────────────────

pub enum CanvasState {
    Browse,
    New  { buf: String, text_cursor: usize },
    Pick { origin_id: usize, origin_x: i32, origin_y: i32 },
    /// Linking: navigate cursor to a target; Enter toggles the link, Esc cancels.
    Link { origin_id: usize },
    /// Arrow-display settings menu open — waiting for i / o / g / F / Esc.
    Menu,
    /// Menu sub-state: waiting for T or S to set incoming fidelity.
    MenuIncoming,
    /// Menu sub-state: waiting for T or S to set outgoing fidelity.
    MenuOutgoing,
}
