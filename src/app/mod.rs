mod mode;
pub use mode::Mode;

use crate::models::node::Node;

pub struct App {
    pub nodes:    Vec<Node>,
    pub selected: usize,   // valid only when has_selection() is true
    pub camera_x: i32,
    pub camera_y: i32,
    pub mode:     Mode,
}

impl App {
    pub fn new() -> Self {
        Self {
            nodes:    vec![],
            selected: 0,
            camera_x: -4,
            camera_y: -2,
            mode:     Mode::Normal,
        }
    }

    pub fn has_selection(&self) -> bool {
        self.selected < self.nodes.len()
    }

    // ── Layout ────────────────────────────────────────────────────────────────

    pub fn recompute_layout(&mut self) -> Vec<(usize, usize)> {
        for n in self.nodes.iter_mut() { n.row = usize::MAX; }
        let mut order: Vec<(usize, usize)> = Vec::new();

        let roots: Vec<usize> = (0..self.nodes.len())
            .filter(|&i| self.nodes[i].parent.is_none())
            .collect();

        for root in roots {
            let offset = order.len();
            let mut local: Vec<(usize, usize)> = Vec::new();
            Self::collect_visible_inner(&mut self.nodes, root, 0, &mut local);
            let (rx, ry) = (self.nodes[root].world_x, self.nodes[root].world_y);
            for (lr, &(id, _)) in local.iter().enumerate() {
                self.nodes[id].row     = offset + lr;
                self.nodes[id].world_x = rx;
                self.nodes[id].world_y = ry + lr as i32;
            }
            order.extend(local);
        }
        order
    }

    fn collect_visible_inner(nodes: &mut Vec<Node>, node: usize, depth: usize, out: &mut Vec<(usize, usize)>) {
        out.push((node, depth));
        if !nodes[node].collapsed {
            let children = nodes[node].children.clone();
            for child in children { Self::collect_visible_inner(nodes, child, depth + 1, out); }
        }
    }

    // ── Camera ────────────────────────────────────────────────────────────────

    pub fn pan(&mut self, dx: i32, dy: i32) { self.camera_x += dx; self.camera_y += dy; }

    pub fn scroll_to_selected(&mut self, canvas_h: usize) {
        if !self.has_selection() || canvas_h == 0 { return; }
        let wy = self.nodes[self.selected].world_y;
        let sy = wy - self.camera_y;
        if sy < 0 { self.camera_y = wy; }
        else if sy >= canvas_h as i32 { self.camera_y = wy - (canvas_h as i32 - 1); }
    }

    pub fn center_on_selected(&mut self, canvas_w: u16, canvas_h: u16) {
        if !self.has_selection() { return; }
        let wx = self.nodes[self.selected].world_x;
        let wy = self.nodes[self.selected].world_y;
        self.camera_x = wx - (canvas_w / 2) as i32;
        self.camera_y = wy - (canvas_h / 2) as i32;
    }

    // ── Focus navigation ──────────────────────────────────────────────────────

    pub fn nav_down(&mut self) {
        if !self.has_selection() { return; }
        let order = self.recompute_layout();
        let cur_row = self.nodes[self.selected].row;
        if cur_row != usize::MAX {
            if let Some(&(next, _)) = order.get(cur_row + 1) { self.selected = next; }
        }
    }

    pub fn nav_up(&mut self) {
        if !self.has_selection() { return; }
        let cur_row = self.nodes[self.selected].row;
        if cur_row == usize::MAX || cur_row == 0 { return; }
        let order = self.recompute_layout();
        if let Some(&(prev, _)) = order.get(cur_row - 1) { self.selected = prev; }
    }

    pub fn nav_parent(&mut self) {
        if !self.has_selection() { return; }
        if let Some(p) = self.nodes[self.selected].parent { self.selected = p; }
    }

    pub fn nav_child(&mut self) {
        if !self.has_selection() { return; }
        let has_ch    = !self.nodes[self.selected].children.is_empty();
        let collapsed = self.nodes[self.selected].collapsed;
        if has_ch && collapsed {
            self.nodes[self.selected].collapsed = false;
        } else if let Some(&c) = self.nodes[self.selected].children.first() {
            self.selected = c;
        }
    }

    pub fn toggle_collapse(&mut self) {
        if !self.has_selection() { return; }
        if !self.nodes[self.selected].children.is_empty() {
            let c = self.nodes[self.selected].collapsed;
            self.nodes[self.selected].collapsed = !c;
        }
    }

    // ── Nodes mode ────────────────────────────────────────────────────────────

    /// Enter Nodes mode; cursor starts at the camera centre.
    pub fn enter_nodes(&mut self, canvas_w: u16, canvas_h: u16) {
        let cx = self.camera_x + (canvas_w / 2) as i32;
        let cy = self.camera_y + (canvas_h / 2) as i32;
        self.mode = Mode::Nodes { cursor_x: cx, cursor_y: cy };
    }

    pub fn nodes_move(&mut self, dx: i32, dy: i32, canvas_w: u16, canvas_h: u16) {
        if let Mode::Nodes { ref mut cursor_x, ref mut cursor_y } = self.mode {
            *cursor_x += dx;
            *cursor_y += dy;
        }
        if let Mode::Nodes { cursor_x, cursor_y } = self.mode {
            self.camera_x = cursor_x - (canvas_w / 2) as i32;
            self.camera_y = cursor_y - (canvas_h / 2) as i32;
        }
    }

    /// In Nodes mode, `n` starts typing a new root node at the cursor.
    pub fn nodes_start_new(&mut self) {
        if let Mode::Nodes { cursor_x, cursor_y } = self.mode {
            self.mode = Mode::NodeNew { cursor_x, cursor_y, buf: String::new(), text_cursor: 0 };
        }
    }

    /// In Nodes mode, `p` picks the node nearest the cursor; in NodePick mode, places it.
    pub fn nodes_pick_or_place(&mut self) {
        match self.mode {
            Mode::Nodes { cursor_x, cursor_y } => {
                if let Some(id) = self.node_near_cursor(cursor_x, cursor_y) {
                    let (ox, oy) = (self.nodes[id].world_x, self.nodes[id].world_y);
                    self.mode = Mode::NodePick {
                        cursor_x: ox,
                        cursor_y: oy,
                        origin_id: id,
                        origin_x: ox,
                        origin_y: oy,
                    };
                }
            }
            Mode::NodePick { cursor_x, cursor_y, origin_id, .. } => {
                if let Some(p) = self.nodes[origin_id].parent {
                    self.nodes[p].children.retain(|&c| c != origin_id);
                    self.nodes[origin_id].parent = None;
                }
                self.nodes[origin_id].world_x = cursor_x;
                self.nodes[origin_id].world_y = cursor_y;
                self.selected = origin_id;
                self.mode = Mode::Nodes { cursor_x, cursor_y };
            }
            _ => {}
        }
    }

    pub fn nodes_pick_move(&mut self, dx: i32, dy: i32, canvas_w: u16, canvas_h: u16) {
        if let Mode::NodePick { ref mut cursor_x, ref mut cursor_y, .. } = self.mode {
            *cursor_x += dx;
            *cursor_y += dy;
        }
        if let Mode::NodePick { cursor_x, cursor_y, .. } = self.mode {
            self.camera_x = cursor_x - (canvas_w / 2) as i32;
            self.camera_y = cursor_y - (canvas_h / 2) as i32;
        }
    }

    /// Find the closest visible node to world point (cx, cy), within Manhattan distance 4.
    pub fn node_near_cursor(&self, cx: i32, cy: i32) -> Option<usize> {
        self.nodes.iter().enumerate()
            .filter(|(_, n)| n.row != usize::MAX)
            .map(|(id, n)| (id, (n.world_x - cx).abs() + (n.world_y - cy).abs()))
            .filter(|&(_, d)| d <= 4)
            .min_by_key(|&(_, d)| d)
            .map(|(id, _)| id)
    }

    // ── NodeNew ───────────────────────────────────────────────────────────────

    pub fn node_new_char(&mut self, c: char) {
        if let Mode::NodeNew { ref mut buf, ref mut text_cursor, .. } = self.mode {
            buf.insert(*text_cursor, c);
            *text_cursor += c.len_utf8();
        }
    }

    pub fn node_new_backspace(&mut self) {
        if let Mode::NodeNew { ref mut buf, ref mut text_cursor, .. } = self.mode {
            if *text_cursor > 0 {
                let prev = buf[..*text_cursor].char_indices().last().map(|(i, _)| i).unwrap_or(0);
                buf.drain(prev..*text_cursor);
                *text_cursor = prev;
            }
        }
    }

    pub fn node_new_move_cursor(&mut self, delta: i32) {
        if let Mode::NodeNew { ref buf, ref mut text_cursor, .. } = self.mode {
            if delta < 0 {
                *text_cursor = buf[..*text_cursor].char_indices().last().map(|(i, _)| i).unwrap_or(0);
            } else {
                let next = buf[*text_cursor..]
                    .char_indices().nth(1).map(|(i, _)| *text_cursor + i).unwrap_or(buf.len());
                *text_cursor = next;
            }
        }
    }

    pub fn confirm_node_new(&mut self) {
        let (cx, cy, buf) = match &self.mode {
            Mode::NodeNew { cursor_x, cursor_y, buf, .. } => (*cursor_x, *cursor_y, buf.clone()),
            _ => return,
        };
        if !buf.is_empty() {
            let id = self.nodes.len();
            self.nodes.push(Node {
                label: buf, parent: None, children: vec![],
                collapsed: false, row: usize::MAX, world_x: cx, world_y: cy,
            });
            self.selected = id;
        }
        self.mode = Mode::Nodes { cursor_x: cx, cursor_y: cy };
    }

    pub fn cancel_node_new(&mut self) {
        let (cx, cy) = match self.mode {
            Mode::NodeNew { cursor_x, cursor_y, .. } => (cursor_x, cursor_y),
            _ => return,
        };
        self.mode = Mode::Nodes { cursor_x: cx, cursor_y: cy };
    }

    pub fn cancel_nodes(&mut self) {
        self.mode = Mode::Normal;
    }

    // ── Reparent mode ─────────────────────────────────────────────────────────

    pub fn enter_reparent(&mut self) {
        if !self.has_selection() { return; }
        let subject     = self.selected;
        let orig_parent = self.nodes[subject].parent;
        let orig_pos    = orig_parent
            .map(|p| self.nodes[p].children.iter().position(|&c| c == subject).unwrap_or(0))
            .unwrap_or(0);
        let subtree = self.collect_subtree(subject);
        let cursor  = orig_parent.unwrap_or_else(|| {
            (0..self.nodes.len()).find(|&i| !subtree.contains(&i)).unwrap_or(subject)
        });
        self.mode = Mode::Reparent { subject, orig_parent, orig_pos, cursor };
    }

    pub fn reparent_nav_to(&mut self, new_cursor: usize) {
        let (subject, old_cursor) = match self.mode {
            Mode::Reparent { subject, cursor, .. } => (subject, cursor),
            _ => return,
        };
        if new_cursor == old_cursor { return; }
        if let Some(p) = self.nodes[subject].parent { self.nodes[p].children.retain(|&c| c != subject); }
        self.nodes[new_cursor].children.push(subject);
        self.nodes[subject].parent = Some(new_cursor);
        if let Mode::Reparent { ref mut cursor, .. } = self.mode { *cursor = new_cursor; }
    }

    pub fn reparent_nav_vertical(&mut self, delta: i32) {
        let (subject, cursor) = match self.mode {
            Mode::Reparent { subject, cursor, .. } => (subject, cursor),
            _ => return,
        };
        let subtree = self.collect_subtree(subject);
        let mut nav: Vec<(usize, usize)> = self.nodes.iter().enumerate()
            .filter(|&(id, n)| n.row != usize::MAX && !subtree.contains(&id))
            .map(|(id, n)| (n.row, id))
            .collect();
        nav.sort_by_key(|&(row, _)| row);
        if nav.is_empty() { return; }
        let pos = nav.iter().position(|&(_, id)| id == cursor).unwrap_or(0);
        let new_pos = if delta > 0 { (pos + 1).min(nav.len() - 1) } else { pos.saturating_sub(1) };
        let new_cursor = nav[new_pos].1;
        self.reparent_nav_to(new_cursor);
    }

    pub fn reparent_nav_parent(&mut self) {
        let (subject, cursor) = match self.mode {
            Mode::Reparent { subject, cursor, .. } => (subject, cursor),
            _ => return,
        };
        let subtree = self.collect_subtree(subject);
        if let Some(parent) = self.nodes[cursor].parent {
            if !subtree.contains(&parent) { self.reparent_nav_to(parent); }
        }
    }

    pub fn reparent_nav_child(&mut self) {
        let (subject, cursor) = match self.mode {
            Mode::Reparent { subject, cursor, .. } => (subject, cursor),
            _ => return,
        };
        let subtree = self.collect_subtree(subject);
        let child = self.nodes[cursor].children.iter().find(|&&c| !subtree.contains(&c)).copied();
        if let Some(c) = child { self.reparent_nav_to(c); }
    }

    pub fn confirm_reparent(&mut self) {
        if let Mode::Reparent { subject, .. } = self.mode { self.selected = subject; }
        self.mode = Mode::Normal;
    }

    pub fn cancel_reparent(&mut self) {
        let (subject, orig_parent, orig_pos) = match self.mode {
            Mode::Reparent { subject, orig_parent, orig_pos, .. } => (subject, orig_parent, orig_pos),
            _ => return,
        };
        if let Some(p) = self.nodes[subject].parent { self.nodes[p].children.retain(|&c| c != subject); }
        self.nodes[subject].parent = orig_parent;
        if let Some(p) = orig_parent {
            let pos = orig_pos.min(self.nodes[p].children.len());
            self.nodes[p].children.insert(pos, subject);
        }
        self.mode = Mode::Normal;
    }

    // ── Depth adjustment ──────────────────────────────────────────────────────

    pub fn indent_increase(&mut self) {
        if !self.has_selection() { return; }
        let parent = match self.nodes[self.selected].parent { Some(p) => p, None => return };
        let pos = match self.nodes[parent].children.iter().position(|&s| s == self.selected) {
            Some(p) => p, None => return,
        };
        if pos == 0 { return; }
        let prev = self.nodes[parent].children[pos - 1];
        self.nodes[parent].children.remove(pos);
        self.nodes[prev].children.push(self.selected);
        self.nodes[self.selected].parent = Some(prev);
    }

    pub fn indent_decrease(&mut self) {
        if !self.has_selection() { return; }
        let parent = match self.nodes[self.selected].parent { Some(p) => p, None => return };
        let grandparent = match self.nodes[parent].parent { Some(gp) => gp, None => return };
        let parent_pos = self.nodes[grandparent].children.iter().position(|&c| c == parent).unwrap();
        self.nodes[parent].children.retain(|&c| c != self.selected);
        self.nodes[grandparent].children.insert(parent_pos + 1, self.selected);
        self.nodes[self.selected].parent = Some(grandparent);
    }

    // ── Insert ────────────────────────────────────────────────────────────────

    pub fn enter_insert(&mut self) {
        if !self.has_selection() { return; }
        self.mode = Mode::Insert { parent: self.selected, buf: String::new(), cursor: 0 };
    }

    pub fn insert_indent(&mut self) {
        if let Mode::Insert { ref mut parent, .. } = self.mode {
            if let Some(&last) = self.nodes[*parent].children.last() { *parent = last; }
        }
    }

    pub fn insert_dedent(&mut self) {
        if let Mode::Insert { ref mut parent, .. } = self.mode {
            if let Some(gp) = self.nodes[*parent].parent { *parent = gp; }
        }
    }

    pub fn scroll_to_insert(&mut self, canvas_h: usize) {
        let (parent, visible_count) = match self.mode {
            Mode::Insert { parent, .. } => {
                let subtree = self.collect_subtree(parent);
                let count = subtree.iter().filter(|&&id| self.nodes[id].row != usize::MAX).count();
                (parent, count)
            }
            _ => return,
        };
        let insert_wy = self.nodes[parent].world_y + visible_count as i32;
        let sy = insert_wy - self.camera_y;
        if sy < 0 { self.camera_y = insert_wy; }
        else if canvas_h > 0 && sy >= canvas_h as i32 { self.camera_y = insert_wy - (canvas_h as i32 - 1); }
    }

    pub fn insert_char(&mut self, c: char) {
        if let Mode::Insert { ref mut buf, ref mut cursor, .. } = self.mode {
            buf.insert(*cursor, c);
            *cursor += c.len_utf8();
        }
    }

    pub fn insert_backspace(&mut self) {
        if let Mode::Insert { ref mut buf, ref mut cursor, .. } = self.mode {
            if *cursor > 0 {
                let prev = buf[..*cursor].char_indices().last().map(|(i, _)| i).unwrap_or(0);
                buf.drain(prev..*cursor);
                *cursor = prev;
            }
        }
    }

    pub fn insert_move_cursor(&mut self, delta: i32) {
        if let Mode::Insert { ref buf, ref mut cursor, .. } = self.mode {
            if delta < 0 {
                *cursor = buf[..*cursor].char_indices().last().map(|(i, _)| i).unwrap_or(0);
            } else {
                let next = buf[*cursor..].char_indices().nth(1)
                    .map(|(i, _)| *cursor + i).unwrap_or(buf.len());
                *cursor = next;
            }
        }
    }

    pub fn confirm_insert(&mut self) {
        let action = match &self.mode {
            Mode::Insert { parent, buf, .. } => Some((*parent, buf.clone())),
            _ => None,
        };
        if let Some((parent, buf)) = action {
            if buf.is_empty() { self.mode = Mode::Normal; return; }
            let new_idx = self.nodes.len();
            self.nodes.push(Node {
                label: buf, parent: Some(parent), children: vec![],
                collapsed: false, row: usize::MAX, world_x: 0, world_y: 0,
            });
            self.nodes[parent].children.push(new_idx);
            self.nodes[parent].collapsed = false;
            self.selected = new_idx;
            self.mode = Mode::Insert { parent, buf: String::new(), cursor: 0 };
        }
    }

    pub fn cancel_insert(&mut self) { self.mode = Mode::Normal; }

    // ── Delete ────────────────────────────────────────────────────────────────

    pub fn enter_confirm_delete(&mut self) {
        if !self.has_selection() { return; }
        self.mode = Mode::Confirm { target: self.selected };
    }

    pub fn collect_subtree(&self, root: usize) -> Vec<usize> {
        let mut result = vec![root];
        let mut i = 0;
        while i < result.len() {
            for &child in &self.nodes[result[i]].children { result.push(child); }
            i += 1;
        }
        result
    }

    pub fn confirm_delete(&mut self) {
        let target = match self.mode {
            Mode::Confirm { target } => target,
            _ => { self.mode = Mode::Normal; return; }
        };

        let to_delete = self.collect_subtree(target);

        if let Some(parent) = self.nodes[target].parent {
            self.nodes[parent].children.retain(|&c| c != target);
        }

        let n = self.nodes.len();
        let mut remap: Vec<Option<usize>> = vec![None; n];
        let mut next = 0usize;
        for i in 0..n {
            if !to_delete.contains(&i) { remap[i] = Some(next); next += 1; }
        }

        let mut new_nodes: Vec<Node> = Vec::new();
        for i in 0..n {
            if remap[i].is_some() {
                let node = &self.nodes[i];
                new_nodes.push(Node {
                    label:     node.label.clone(),
                    parent:    node.parent.and_then(|p| remap[p]),
                    children:  node.children.iter().filter_map(|&c| remap[c]).collect(),
                    collapsed: node.collapsed,
                    row:       usize::MAX,
                    world_x:   node.world_x,
                    world_y:   node.world_y,
                });
            }
        }

        self.nodes = new_nodes;
        self.selected = self.nodes[target].parent
            .and_then(|p| remap[p])
            .unwrap_or(0);
        self.mode = Mode::Normal;
    }

    pub fn cancel_confirm(&mut self) { self.mode = Mode::Normal; }
}
