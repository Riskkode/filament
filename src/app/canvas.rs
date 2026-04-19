use super::{App, CanvasState, InputAction, Mode};
use crate::models::node::Node;
use std::collections::HashMap;

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
        let is_browse = matches!(self.mode, Mode::Canvas { state: CanvasState::Browse });
        let is_pick = matches!(self.mode, Mode::Canvas { state: CanvasState::Pick { .. } });

        if is_browse {
            if let Some(id) = self.node_near_cursor(self.cursor_x, self.cursor_y) {
                self.push_undo();
                let (ox, oy) = (self.nodes[id].world_x, self.nodes[id].world_y);
                self.mode = Mode::Canvas {
                    state: CanvasState::Pick { 
                        origin_id: id, origin_x: ox, origin_y: oy,
                        buf: String::new(), cursor: 0
                    },
                };
            }
        } else if is_pick {
            if let Mode::Canvas { state: CanvasState::Pick { origin_id, ref buf, .. } } = self.mode {
                let id = origin_id;
                
                // If buffer has coordinates, parse and move there instead of current cursor
                let mut target_x = self.cursor_x;
                let mut target_y = self.cursor_y;
                
                if !buf.is_empty() {
                    let parts: Vec<&str> = buf.split(',').collect();
                    if parts.len() == 2 {
                        if let (Ok(x), Ok(y)) = (parts[0].trim().parse::<i32>(), parts[1].trim().parse::<i32>()) {
                            target_x = x;
                            target_y = y;
                        }
                    }
                }

                self.push_undo();
                if let Some(p) = self.nodes[id].parent {
                    self.nodes[p].children.retain(|&c| c != id);
                    self.nodes[id].parent = None;
                }
                self.nodes[id].world_x = target_x;
                self.nodes[id].world_y = target_y;
                self.cursor_x = target_x;
                self.cursor_y = target_y;
                self.selected = id;
                self.mode = Mode::Canvas { state: CanvasState::Browse };
            }
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

    pub fn canvas_pick_char(&mut self, c: char) {
        if let Mode::Canvas { state: CanvasState::Pick { ref mut buf, ref mut cursor, .. } } = self.mode {
            buf.insert(*cursor, c);
            *cursor += c.len_utf8();
        }
    }

    pub fn canvas_pick_backspace(&mut self) {
        if let Mode::Canvas { state: CanvasState::Pick { ref mut buf, ref mut cursor, .. } } = self.mode {
            if *cursor > 0 {
                let prev = buf[..*cursor].char_indices().last().map(|(i, _)| i).unwrap_or(0);
                buf.drain(prev..*cursor);
                *cursor = prev;
            }
        }
    }

    pub fn canvas_confirm_new(&mut self) {
        let buf = match &self.mode {
            Mode::Canvas { state: CanvasState::New { buf, .. } } => buf.clone(),
            _ => return,
        };
        if !buf.is_empty() {
            self.push_undo();
            let id = self.nodes.len();
            self.nodes.push(Node {
                label: buf, parent: None, children: vec![], links: vec![],
                collapsed: false, row: usize::MAX,
                world_x: self.cursor_x, world_y: self.cursor_y, world_x_end: 0,
                tags: HashMap::new(),
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

    /// `s` — enter Status Tagging state from the currently selected node.
    pub fn canvas_start_status_tagging(&mut self) {
        if !self.has_selection() { return; }
        self.mode = Mode::Canvas { state: CanvasState::TagStatus };
    }

    /// Set or clear the "status" tag on the currently selected node.
    pub fn canvas_set_status(&mut self, status: Option<&str>) {
        if !self.has_selection() {
            self.mode = Mode::Canvas { state: CanvasState::Browse };
            return;
        }
        let id = self.selected;
        self.push_undo();
        if let Some(s) = status {
            self.nodes[id].tags.insert("status".to_string(), s.to_string());
        } else {
            self.nodes[id].tags.remove("status");
        }
        self.mode = Mode::Canvas { state: CanvasState::Browse };
        self.save_project();
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
                self.push_undo();
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
        if let Mode::Canvas { state: CanvasState::Pick { origin_id, origin_x, origin_y, .. } } = &self.mode {
            let id = *origin_id;
            self.nodes[id].world_x = *origin_x;
            self.nodes[id].world_y = *origin_y;
            // We don't restore the parent here because picking already detached it.
            // Instead, we rely on the user to undo if they want the parent back,
            // or we could manually re-attach if we stored the parent.
            // For now, let's at least restore the position.
        }
        self.mode = Mode::Canvas { state: CanvasState::Browse };
    }

    pub fn canvas_jump_link(&mut self, delta: i32, canvas_w: u16, canvas_h: u16) {
        if !self.has_selection() { return; }
        
        let origin = if let Some(org) = self.last_link_origin {
            if self.nodes[org].links.contains(&self.selected) { org } else { self.selected }
        } else {
            self.selected
        };

        let links = &self.nodes[origin].links;
        if links.is_empty() { return; }

        if Some(origin) != self.last_link_origin {
            // New jump session
            self.last_link_origin = Some(origin);
            self.last_link_idx = if delta > 0 { 0 } else { links.len() - 1 };
        } else {
            // Continue existing session
            if delta > 0 {
                self.last_link_idx = (self.last_link_idx + 1) % links.len();
            } else {
                self.last_link_idx = (self.last_link_idx + links.len() - 1) % links.len();
            }
        }

        self.selected = links[self.last_link_idx];
        self.center_on_selected(canvas_w, canvas_h);
    }

    // ── Goto ──────────────────────────────────────────────────────────────────

    pub fn canvas_start_goto(&mut self) {
        let current_state = if let Mode::Canvas { state } = &self.mode {
            state.clone()
        } else {
            CanvasState::Browse
        };

        self.mode = Mode::Canvas { state: CanvasState::Goto {
            buf: String::new(),
            cursor: 0,
            matches: vec![],
            match_idx: 0,
            previous: Box::new(current_state),
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
        let (selected_id, previous) = if let Mode::Canvas { state: CanvasState::Goto { ref matches, match_idx, ref previous, .. } } = self.mode {
            let id = if !matches.is_empty() { Some(matches[match_idx]) } else { None };
            (id, Some(*previous.clone()))
        } else {
            (None, None)
        };

        if let Some(id) = selected_id {
            self.selected = id;
            self.center_on_selected(canvas_w, canvas_h);
        }

        if let Some(prev) = previous {
            self.mode = Mode::Canvas { state: prev };
        } else {
            self.canvas_cancel_sub();
        }
    }

    // ── Help ──────────────────────────────────────────────────────────────────

    pub fn canvas_start_help(&mut self) {
        // Capture previous mode before changing anything
        self.help_previous_mode = Some(self.mode.clone());

        // We push current state to undo so we can easily restore it, 
        // but only if we are currently IN a project.
        if self.project_path.is_some() {
            self.push_undo(); 
        }

        self.nodes.clear();

        fn add_help_node(nodes: &mut Vec<Node>, label: &str, parent: Option<usize>) -> usize {
            let id = nodes.len();
            nodes.push(Node {
                label: label.to_string(),
                parent,
                children: vec![],
                links: vec![],
                collapsed: false,
                row: usize::MAX,
                world_x: 0,
                world_y: 0,
                world_x_end: 0,
                tags: HashMap::new(),
            });
            if let Some(p) = parent { nodes[p].children.push(id); }
            id
        }

        let root = add_help_node(&mut self.nodes, "HELP: Commands & Shortcuts", None);

        let nav = add_help_node(&mut self.nodes, "Navigation", Some(root));
        add_help_node(&mut self.nodes, "hjkl / Arrows : Move cursor", Some(nav));
        add_help_node(&mut self.nodes, "HJKL (Shift)  : Cardinal warp (jump to next node)", Some(nav));
        add_help_node(&mut self.nodes, "Tab / S-Tab   : Cycle through outgoing links", Some(nav));
        add_help_node(&mut self.nodes, "c             : Center camera on selection", Some(nav));
        add_help_node(&mut self.nodes, "g             : Goto (fuzzy search nodes)", Some(nav));

        let ops = add_help_node(&mut self.nodes, "Operations", Some(root));
        add_help_node(&mut self.nodes, "i             : Insert new child", Some(ops));
        add_help_node(&mut self.nodes, "n             : New root node at cursor", Some(ops));
        add_help_node(&mut self.nodes, "e             : Edit label (append)", Some(ops));
        add_help_node(&mut self.nodes, "E (Shift)     : Edit label (overwrite)", Some(ops));
        add_help_node(&mut self.nodes, "x             : Delete node and subtree", Some(ops));
        add_help_node(&mut self.nodes, "p             : Pick / Place node (hjkl to move)", Some(ops));
        add_help_node(&mut self.nodes, "v             : Interactive Reparent", Some(ops));
        add_help_node(&mut self.nodes, "f             : Link (toggle connection)", Some(ops));
        add_help_node(&mut self.nodes, "u             : Undo last action", Some(ops));

        let struct_ = add_help_node(&mut self.nodes, "Structure", Some(root));
        add_help_node(&mut self.nodes, "d             : Increase depth (nest)", Some(struct_));
        add_help_node(&mut self.nodes, "D (Shift)     : Decrease depth (promote)", Some(struct_));
        add_help_node(&mut self.nodes, "z / Space     : Toggle collapse subtree", Some(struct_));
        add_help_node(&mut self.nodes, "F (Shift)     : Arrow display settings", Some(struct_));

        let sys = add_help_node(&mut self.nodes, "System", Some(root));
        add_help_node(&mut self.nodes, "q             : Return to Main Menu", Some(sys));
        add_help_node(&mut self.nodes, "Q (Shift)     : Save and Quit Filament", Some(sys));
        add_help_node(&mut self.nodes, "?             : Toggle this help tree", Some(sys));
        add_help_node(&mut self.nodes, "Esc           : Cancel / Close modal", Some(sys));

        self.selected = root;
        
        // Center the help tree
        let (tw, th) = crossterm::terminal::size().unwrap_or((80, 24));
        let help_w = 50i32; // Approximate width
        let help_h = 25i32; // Approximate height
        let center_x = (tw as i32 - help_w) / 2;
        let center_y = (th as i32 - help_h) / 2;
        
        self.nodes[root].world_x = center_x.max(0);
        self.nodes[root].world_y = center_y.max(0);
        self.cursor_x = self.nodes[root].world_x;
        self.cursor_y = self.nodes[root].world_y;
        self.camera_x = 0;
        self.camera_y = 0;
        
        self.recompute_layout();
        self.mode = Mode::Help;
    }

    pub fn canvas_close_help(&mut self) {
        let prev_mode = self.help_previous_mode.take();
        
        if let Some(Mode::StartMenu { .. }) = prev_mode {
            self.init_main_menu_nodes();
        } else {
            self.undo(); // Restore the nodes we cleared
        }
        
        self.mode = prev_mode.unwrap_or(Mode::Canvas { state: CanvasState::Browse });
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
