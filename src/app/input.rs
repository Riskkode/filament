use super::{App, CanvasState, InputAction, Mode};
use crate::models::node::Node;

impl App {
    /// `i` — insert new child under selected node.
    pub fn enter_insert(&mut self) {
        if !self.has_selection() { return; }
        self.mode = Mode::Input {
            action: InputAction::InsertChild { parent: self.selected },
            buf: String::new(), cursor: 0,
        };
    }

    /// `e` — edit selected node's label; cursor starts at end.
    pub fn enter_edit(&mut self) {
        if !self.has_selection() { return; }
        let buf = self.nodes[self.selected].label.clone();
        let cursor = buf.len();
        self.mode = Mode::Input { action: InputAction::EditLabel { node: self.selected }, buf, cursor };
    }

    /// `E` — overwrite selected node's label; starts with empty buffer.
    pub fn enter_overwrite(&mut self) {
        if !self.has_selection() { return; }
        self.mode = Mode::Input {
            action: InputAction::Overwrite { node: self.selected },
            buf: String::new(), cursor: 0,
        };
    }

    pub fn input_char(&mut self, c: char) {
        if let Mode::Input { ref mut buf, ref mut cursor, .. } = self.mode {
            buf.insert(*cursor, c);
            *cursor += c.len_utf8();
        }
    }

    pub fn input_backspace(&mut self) {
        if let Mode::Input { ref mut buf, ref mut cursor, .. } = self.mode {
            if *cursor > 0 {
                let prev = buf[..*cursor].char_indices().last().map(|(i, _)| i).unwrap_or(0);
                buf.drain(prev..*cursor);
                *cursor = prev;
            }
        }
    }

    pub fn input_move_cursor(&mut self, delta: i32) {
        if let Mode::Input { ref buf, ref mut cursor, .. } = self.mode {
            if delta < 0 {
                *cursor = buf[..*cursor].char_indices().last().map(|(i, _)| i).unwrap_or(0);
            } else {
                let next = buf[*cursor..].char_indices().nth(1)
                    .map(|(i, _)| *cursor + i).unwrap_or(buf.len());
                *cursor = next;
            }
        }
    }

    /// Tab: nest the insert target under its last sibling (InsertChild only).
    pub fn input_indent(&mut self) {
        if let Mode::Input { action: InputAction::InsertChild { ref mut parent }, .. } = self.mode {
            if let Some(&last) = self.nodes[*parent].children.last() { *parent = last; }
        }
    }

    /// Shift+Tab: promote the insert target to its grandparent (InsertChild only).
    pub fn input_dedent(&mut self) {
        if let Mode::Input { action: InputAction::InsertChild { ref mut parent }, .. } = self.mode {
            if let Some(gp) = self.nodes[*parent].parent { *parent = gp; }
        }
    }

    pub fn confirm_input(&mut self) {
        let action = match &self.mode {
            Mode::Input { action, .. } => action.clone(),
            _ => return,
        };
        let buf = match &self.mode {
            Mode::Input { buf, .. } => buf.clone(),
            _ => return,
        };

        match action {
            InputAction::InsertChild { parent } => {
                if buf.is_empty() { self.mode = Mode::Canvas { state: CanvasState::Browse }; return; }
                let new_idx = self.nodes.len();
                self.nodes.push(Node {
                    label: buf, parent: Some(parent), children: vec![],
                    collapsed: false, row: usize::MAX, world_x: 0, world_y: 0,
                });
                self.nodes[parent].children.push(new_idx);
                self.nodes[parent].collapsed = false;
                self.selected = new_idx;
                self.mode = Mode::Input {
                    action: InputAction::InsertChild { parent },
                    buf: String::new(), cursor: 0,
                };
            }
            InputAction::EditLabel { node } | InputAction::Overwrite { node } => {
                if !buf.is_empty() { self.nodes[node].label = buf; }
                self.mode = Mode::Canvas { state: CanvasState::Browse };
            }
        }
    }

    pub fn cancel_input(&mut self) {
        self.mode = Mode::Canvas { state: CanvasState::Browse };
    }

    /// Scroll so the pending insertion point is visible (InsertChild only).
    pub fn scroll_to_input(&mut self, canvas_h: usize) {
        let parent = match &self.mode {
            Mode::Input { action: InputAction::InsertChild { parent }, .. } => *parent,
            _ => return,
        };
        let subtree = self.collect_subtree(parent);
        let count = subtree.iter().filter(|&&id| self.nodes[id].row != usize::MAX).count();
        let insert_wy = self.nodes[parent].world_y + count as i32;
        let sy = insert_wy - self.camera_y;
        if sy < 0 { self.camera_y = insert_wy; }
        else if canvas_h > 0 && sy >= canvas_h as i32 {
            self.camera_y = insert_wy - (canvas_h as i32 - 1);
        }
    }
}
