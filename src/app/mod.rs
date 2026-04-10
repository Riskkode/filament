mod mode;
mod canvas;
mod delete;
mod input;
mod reparent;

pub use mode::{ArrowFidelity, ArrowSettings, CanvasState, InputAction, Mode};

use crate::models::node::Node;

pub struct App {
    pub arrow:    ArrowSettings,
    pub nodes:    Vec<Node>,
    /// The node currently under (or nearest to) the world cursor.
    /// Valid only when `has_selection()` is true.
    pub selected: usize,
    pub camera_x: i32,
    pub camera_y: i32,
    /// World-space cursor — the single source of truth for "where the user is".
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub mode:     Mode,
}

impl App {
    pub fn new() -> Self {
        Self {
            arrow:    ArrowSettings::default(),
            nodes:    vec![],
            selected: 0,
            camera_x: 0,
            camera_y: 0,
            cursor_x: 0,
            cursor_y: 0,
            mode:     Mode::Canvas { state: CanvasState::Browse },
        }
    }

    pub fn has_selection(&self) -> bool {
        self.selected < self.nodes.len()
            && self.node_near_cursor(self.cursor_x, self.cursor_y).is_some()
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
            for (lr, &(id, depth)) in local.iter().enumerate() {
                self.nodes[id].row       = offset + lr;
                self.nodes[id].world_x   = rx;
                self.nodes[id].world_y   = ry + lr as i32;
                let label_cols = self.nodes[id].label.chars().count() as i32;
                // prefix = depth*2 cols (each level adds 2)  |  "> " = 2 cols
                self.nodes[id].world_x_end = rx + depth as i32 * 2 + 2 + label_cols;
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

    // ── Cursor + camera ───────────────────────────────────────────────────────

    /// Move the world cursor by (dx, dy). Camera lazily follows when the cursor
    /// approaches the edge. Selection snaps to the nearest visible node.
    pub fn cursor_move(&mut self, dx: i32, dy: i32, canvas_w: u16, canvas_h: u16) {
        self.cursor_x += dx;
        self.cursor_y += dy;

        // Snap selection to nearest node.
        if let Some(id) = self.node_near_cursor(self.cursor_x, self.cursor_y) {
            self.selected = id;
        }

        // Lazy camera scroll: keep cursor inside a margin.
        let margin = 6i32;
        let cw = canvas_w as i32;
        let ch = canvas_h as i32;
        let sx = self.cursor_x - self.camera_x;
        let sy = self.cursor_y - self.camera_y;
        if sx < margin          { self.camera_x = self.cursor_x - margin; }
        if sx >= cw - margin    { self.camera_x = self.cursor_x - (cw - margin - 1); }
        if sy < margin          { self.camera_y = self.cursor_y - margin; }
        if sy >= ch - margin    { self.camera_y = self.cursor_y - (ch - margin - 1); }
    }

    /// Warp the cursor directly to the selected node and centre the camera.
    pub fn center_on_selected(&mut self, canvas_w: u16, canvas_h: u16) {
        if self.selected >= self.nodes.len() { return; }
        self.cursor_x = self.nodes[self.selected].world_x;
        self.cursor_y = self.nodes[self.selected].world_y;
        self.camera_x = self.cursor_x - (canvas_w / 2) as i32;
        self.camera_y = self.cursor_y - (canvas_h / 2) as i32;
    }

    /// Scroll camera so the selected node stays visible (used by Reparent navigation).
    pub fn scroll_to_selected(&mut self, canvas_h: usize) {
        if self.selected >= self.nodes.len() || canvas_h == 0 { return; }
        let wy = self.nodes[self.selected].world_y;
        let sy = wy - self.camera_y;
        if sy < 0 { self.camera_y = wy; }
        else if sy >= canvas_h as i32 { self.camera_y = wy - (canvas_h as i32 - 1); }
    }

    // ── Cardinal warp ─────────────────────────────────────────────────────────

    /// Warp the cursor to the next occupied row/column in a cardinal direction.
    /// dx/dy must be exactly ±1 with the other zero.
    /// Does nothing if there are no nodes in that direction.
    pub fn cursor_warp(&mut self, dx: i32, dy: i32, canvas_w: u16, canvas_h: u16) {
        let target = if dy != 0 {
            let candidates: Vec<_> = self.nodes.iter()
                .filter(|n| n.row != usize::MAX)
                .filter(|n| if dy > 0 { n.world_y > self.cursor_y } else { n.world_y < self.cursor_y })
                .collect();
            if candidates.is_empty() { return; }
            let extreme_y = if dy > 0 {
                candidates.iter().map(|n| n.world_y).min().unwrap()
            } else {
                candidates.iter().map(|n| n.world_y).max().unwrap()
            };
            candidates.iter()
                .filter(|n| n.world_y == extreme_y)
                .min_by_key(|n| (n.world_x - self.cursor_x).abs())
                .map(|n| (n.world_x, n.world_y))
        } else {
            let candidates: Vec<_> = self.nodes.iter()
                .filter(|n| n.row != usize::MAX)
                .filter(|n| if dx > 0 { n.world_x > self.cursor_x } else { n.world_x < self.cursor_x })
                .collect();
            if candidates.is_empty() { return; }
            let extreme_x = if dx > 0 {
                candidates.iter().map(|n| n.world_x).min().unwrap()
            } else {
                candidates.iter().map(|n| n.world_x).max().unwrap()
            };
            candidates.iter()
                .filter(|n| n.world_x == extreme_x)
                .min_by_key(|n| (n.world_y - self.cursor_y).abs())
                .map(|n| (n.world_x, n.world_y))
        };

        if let Some((wx, wy)) = target {
            self.cursor_x = wx;
            self.cursor_y = wy;
            if let Some(id) = self.node_near_cursor(self.cursor_x, self.cursor_y) {
                self.selected = id;
            }
            let margin = 6i32;
            let cw = canvas_w as i32;
            let ch = canvas_h as i32;
            let sx = self.cursor_x - self.camera_x;
            let sy = self.cursor_y - self.camera_y;
            if sx < margin       { self.camera_x = self.cursor_x - margin; }
            if sx >= cw - margin { self.camera_x = self.cursor_x - (cw - margin - 1); }
            if sy < margin       { self.camera_y = self.cursor_y - margin; }
            if sy >= ch - margin { self.camera_y = self.cursor_y - (ch - margin - 1); }
        }
    }

    // ── Collapse ──────────────────────────────────────────────────────────────

    pub fn toggle_collapse(&mut self) {
        if !self.has_selection() { return; }
        if !self.nodes[self.selected].children.is_empty() {
            let c = self.nodes[self.selected].collapsed;
            self.nodes[self.selected].collapsed = !c;
        }
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

    // ── Shared helpers ────────────────────────────────────────────────────────

    pub fn collect_subtree(&self, root: usize) -> Vec<usize> {
        let mut result = vec![root];
        let mut i = 0;
        while i < result.len() {
            for &child in &self.nodes[result[i]].children { result.push(child); }
            i += 1;
        }
        result
    }

    /// Returns the id of the closest visible node within Manhattan distance 4,
    /// or `None` if the cursor is in empty space.
    pub fn node_near_cursor(&self, cx: i32, cy: i32) -> Option<usize> {
        self.nodes.iter().enumerate()
            .filter(|(_, n)| n.row != usize::MAX)
            .map(|(id, n)| (id, (n.world_x - cx).abs() + (n.world_y - cy).abs()))
            .filter(|&(_, d)| d <= 4)
            .min_by_key(|&(_, d)| d)
            .map(|(id, _)| id)
    }
}
