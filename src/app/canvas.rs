use super::{App, CanvasState, InputAction, Mode};
use crate::models::node::Node;

impl App {
    // ── Sub-state transitions ─────────────────────────────────────────────────

    /// `n` — open inline new-node prompt at the current cursor position.
    pub fn canvas_start_new(&mut self) {
        if matches!(self.mode, Mode::Canvas { state: CanvasState::Browse }) {
            self.mode = Mode::Canvas {
                state: CanvasState::New { buf: String::new(), text_cursor: 0 },
            };
        }
    }

    /// `p` — pick the node nearest the cursor; or, if already carrying one, place it.
    pub fn canvas_pick_or_place(&mut self) {
        match &self.mode {
            Mode::Canvas { state: CanvasState::Browse } => {
                if let Some(id) = self.node_near_cursor(self.cursor_x, self.cursor_y) {
                    let (ox, oy) = (self.nodes[id].world_x, self.nodes[id].world_y);
                    self.mode = Mode::Canvas {
                        state: CanvasState::Pick { origin_id: id, origin_x: ox, origin_y: oy },
                    };
                }
            }
            Mode::Canvas { state: CanvasState::Pick { origin_id, .. } } => {
                let id = *origin_id;
                if let Some(p) = self.nodes[id].parent {
                    self.nodes[p].children.retain(|&c| c != id);
                    self.nodes[id].parent = None;
                }
                self.nodes[id].world_x = self.cursor_x;
                self.nodes[id].world_y = self.cursor_y;
                self.selected = id;
                self.mode = Mode::Canvas { state: CanvasState::Browse };
            }
            _ => {}
        }
    }

    // ── New-node text editing ─────────────────────────────────────────────────

    pub fn canvas_new_char(&mut self, c: char) {
        if let Mode::Canvas { state: CanvasState::New { ref mut buf, ref mut text_cursor }, .. } = self.mode {
            buf.insert(*text_cursor, c);
            *text_cursor += c.len_utf8();
        }
    }

    pub fn canvas_new_backspace(&mut self) {
        if let Mode::Canvas { state: CanvasState::New { ref mut buf, ref mut text_cursor }, .. } = self.mode {
            if *text_cursor > 0 {
                let prev = buf[..*text_cursor].char_indices().last().map(|(i, _)| i).unwrap_or(0);
                buf.drain(prev..*text_cursor);
                *text_cursor = prev;
            }
        }
    }

    pub fn canvas_new_move_cursor(&mut self, delta: i32) {
        if let Mode::Canvas { state: CanvasState::New { ref buf, ref mut text_cursor }, .. } = self.mode {
            if delta < 0 {
                *text_cursor = buf[..*text_cursor].char_indices().last().map(|(i, _)| i).unwrap_or(0);
            } else {
                let next = buf[*text_cursor..]
                    .char_indices().nth(1).map(|(i, _)| *text_cursor + i).unwrap_or(buf.len());
                *text_cursor = next;
            }
        }
    }

    pub fn canvas_confirm_new(&mut self) {
        let buf = match &self.mode {
            Mode::Canvas { state: CanvasState::New { buf, .. } } => buf.clone(),
            _ => return,
        };
        if !buf.is_empty() {
            let id = self.nodes.len();
            self.nodes.push(Node {
                label: buf, parent: None, children: vec![], links: vec![],
                collapsed: false, row: usize::MAX,
                world_x: self.cursor_x, world_y: self.cursor_y, world_x_end: 0,
            });
            self.selected = id;
            // Chain straight into insert-child so editing continues normally.
            self.mode = Mode::Input {
                action: InputAction::InsertChild { parent: id },
                buf: String::new(),
                cursor: 0,
            };
        } else {
            self.mode = Mode::Canvas { state: CanvasState::Browse };
        }
    }

    // ── Links ─────────────────────────────────────────────────────────────────

    /// `f` — enter Link state from the currently selected node.
    pub fn canvas_start_link(&mut self) {
        if !self.has_selection() { return; }
        let origin_id = self.selected;
        self.mode = Mode::Canvas { state: CanvasState::Link { origin_id } };
    }

    /// Enter — toggle the link between origin and the node nearest the cursor.
    /// Creates the link if absent, removes it if already present.
    /// Does nothing if the cursor is on the origin itself or empty space.
    pub fn canvas_confirm_link(&mut self) {
        let origin_id = match self.mode {
            Mode::Canvas { state: CanvasState::Link { origin_id } } => origin_id,
            _ => return,
        };
        if let Some(target_id) = self.node_near_cursor(self.cursor_x, self.cursor_y) {
            if target_id != origin_id {
                if self.nodes[origin_id].links.contains(&target_id) {
                    self.nodes[origin_id].links.retain(|&l| l != target_id);
                } else {
                    self.nodes[origin_id].links.push(target_id);
                }
            }
        }
        self.mode = Mode::Canvas { state: CanvasState::Browse };
    }

    // ── Cancellation ─────────────────────────────────────────────────────────

    /// Return to Browse from a New, Pick, or Goto sub-state.
    pub fn canvas_cancel_sub(&mut self) {
        self.mode = Mode::Canvas { state: CanvasState::Browse };
    }

    // ── Goto ──────────────────────────────────────────────────────────────────

    pub fn canvas_start_goto(&mut self) {
        self.mode = Mode::Canvas { state: CanvasState::Goto {
            buf: String::new(),
            cursor: 0,
            matches: vec![],
            match_idx: 0,
        }};
    }

    pub fn canvas_goto_input_char(&mut self, c: char) {
        if let Mode::Canvas { state: CanvasState::Goto { ref mut buf, ref mut cursor, .. } } = self.mode {
            buf.insert(*cursor, c);
            *cursor += c.len_utf8();
        }
        self.update_goto_matches();
    }

    pub fn canvas_goto_backspace(&mut self) {
        if let Mode::Canvas { state: CanvasState::Goto { ref mut buf, ref mut cursor, .. } } = self.mode {
            if *cursor > 0 {
                let ch = buf[..*cursor].chars().last().unwrap();
                *cursor -= ch.len_utf8();
                buf.remove(*cursor);
            }
        }
        self.update_goto_matches();
    }

    pub fn canvas_goto_move_cursor(&mut self, delta: i32) {
        if let Mode::Canvas { state: CanvasState::Goto { ref buf, ref mut cursor, .. } } = self.mode {
            if delta < 0 {
                if *cursor > 0 {
                    let ch = buf[..*cursor].chars().last().unwrap();
                    *cursor -= ch.len_utf8();
                }
            } else if *cursor < buf.len() {
                let ch = buf[*cursor..].chars().next().unwrap();
                *cursor += ch.len_utf8();
            }
        }
    }

    fn update_goto_matches(&mut self) {
        if let Mode::Canvas { state: CanvasState::Goto { ref buf, ref mut matches, ref mut match_idx, .. } } = self.mode {
            matches.clear();
            *match_idx = 0;
            if buf.is_empty() { return; }

            let search = buf.to_lowercase();
            for (i, node) in self.nodes.iter().enumerate() {
                if fuzzy_match(&node.label.to_lowercase(), &search) {
                    matches.push(i);
                }
            }
            
            // If we have matches, auto-select the first one
            if !matches.is_empty() {
                self.selected = matches[0];
            }
        }
    }

    pub fn canvas_goto_next(&mut self) {
        if let Mode::Canvas { state: CanvasState::Goto { ref matches, ref mut match_idx, .. } } = self.mode {
            if matches.is_empty() { return; }
            *match_idx = (*match_idx + 1) % matches.len();
            self.selected = matches[*match_idx];
        }
    }

    pub fn canvas_goto_confirm(&mut self, canvas_w: u16, canvas_h: u16) {
        if let Mode::Canvas { state: CanvasState::Goto { ref matches, match_idx, .. } } = self.mode {
            if !matches.is_empty() {
                self.selected = matches[match_idx];
                self.center_on_selected(canvas_w, canvas_h);
            }
        }
        self.canvas_cancel_sub();
    }
}

fn fuzzy_match(text: &str, pattern: &str) -> bool {
    let mut it = text.chars();
    for p in pattern.chars() {
        match it.find(|&t| t == p) {
            Some(_) => {}
            None => return false,
        }
    }
    true
}
