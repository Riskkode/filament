use super::{App, CanvasState, Mode};

impl App {
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
        self.reparent_nav_to(nav[new_pos].1);
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
        self.mode = Mode::Canvas { state: CanvasState::Browse };
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
        self.mode = Mode::Canvas { state: CanvasState::Browse };
    }
}
