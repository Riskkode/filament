mod mode;
mod canvas;
mod delete;
mod input;
mod start_menu;
mod reparent;

pub use mode::{ArrowFidelity, ArrowSettings, CanvasState, InputAction, Mode, StartMenuState};

use crate::models::node::Node;
use crate::persistence::project::ProjectSettings;
use crate::persistence::registry::Registry;
use crate::persistence::settings::GlobalSettings;
use std::path::PathBuf;

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

    // ── Persistence ───────────────────────────────────────────────────────────
    /// Path to the open project's root directory (where `.filament/` lives).
    pub project_path: Option<PathBuf>,
    /// Display name of the open project.
    pub project_name: String,
    /// Unix timestamp from project.toml — preserved across saves.
    pub project_created_at: u64,
    /// Global project registry loaded from `~/.config/filament/registry.toml`.
    pub registry: Registry,
    /// Global application settings.
    pub settings: GlobalSettings,
    /// Transient message shown in the status bar (errors, hints).
    pub status_message: Option<String>,
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            arrow:    ArrowSettings::default(),
            nodes:    vec![],
            selected: 0,
            camera_x: 0,
            camera_y: 0,
            cursor_x: 0,
            cursor_y: 0,
            mode:     Mode::StartMenu { state: StartMenuState::Main { selected: 0 } },
            project_path:       None,
            project_name:       String::new(),
            project_created_at: 0,
            registry:           Registry::load(),
            settings:           GlobalSettings::load(),
            status_message:     None,
        };
        app.init_main_menu_nodes();
        app
    }

    // ── Persistence ───────────────────────────────────────────────────────────

    /// Open an existing project: reads `project.toml` and `canvas.db` from
    /// `<base>/.filament/` and applies the state to `self`.
    pub fn load_project(&mut self, base: &std::path::Path) {
        use crate::db::connection;
        use crate::repositories::node_repository;

        let settings = match ProjectSettings::load(base) {
            Ok(s) => s,
            Err(e) => { self.status_message = Some(format!("load error: {e}")); return; }
        };

        let db_path = crate::persistence::project::db_path(base);
        let conn = match connection::open(&db_path) {
            Ok(c) => c,
            Err(e) => { self.status_message = Some(format!("db error: {e}")); return; }
        };

        let (nodes, id_to_idx) = match node_repository::load(&conn) {
            Ok(r) => r,
            Err(e) => { self.status_message = Some(format!("load error: {e}")); return; }
        };

        self.nodes    = nodes;
        self.camera_x = settings.view.camera_x;
        self.camera_y = settings.view.camera_y;
        self.cursor_x = settings.view.cursor_x;
        self.cursor_y = settings.view.cursor_y;
        self.selected = id_to_idx.get(&settings.view.selected_db_id).copied().unwrap_or(0);
        self.arrow    = ArrowSettings {
            global:   settings.arrows.global,
            incoming: if settings.arrows.incoming == "Selected" { ArrowFidelity::Selected } else { ArrowFidelity::Tree },
            outgoing: if settings.arrows.outgoing == "Selected" { ArrowFidelity::Selected } else { ArrowFidelity::Tree },
        };
        self.project_path       = Some(base.to_path_buf());
        self.project_name       = settings.name;
        self.project_created_at = settings.created_at;
        self.status_message     = None;
        self.mode               = Mode::Canvas { state: CanvasState::Browse };
    }

    /// Persist the current canvas state to `<project_path>/.filament/`.
    /// Silent no-op when no project is open.
    pub fn save_project(&self) {
        use crate::db::connection;
        use crate::repositories::node_repository;

        let Some(base) = &self.project_path else { return };

        let db_path = crate::persistence::project::db_path(base);
        let mut conn = match connection::open(&db_path) {
            Ok(c) => c,
            Err(_) => return,
        };

        let idx_to_id = match node_repository::save(&mut conn, &self.nodes) {
            Ok(m) => m,
            Err(_) => return,
        };

        let selected_db_id = idx_to_id.get(self.selected).copied().unwrap_or(0);

        let settings = ProjectSettings {
            name:       self.project_name.clone(),
            created_at: self.project_created_at,
            view: crate::persistence::project::ViewState {
                camera_x:       self.camera_x,
                camera_y:       self.camera_y,
                cursor_x:       self.cursor_x,
                cursor_y:       self.cursor_y,
                selected_db_id,
            },
            arrows: crate::persistence::project::SavedArrows {
                global:   self.arrow.global,
                incoming: match self.arrow.incoming { ArrowFidelity::Tree => "Tree", ArrowFidelity::Selected => "Selected" }.into(),
                outgoing: match self.arrow.outgoing { ArrowFidelity::Tree => "Tree", ArrowFidelity::Selected => "Selected" }.into(),
            },
        };

        let _ = settings.save(base);
    }

    /// Initialise a new project on disk, register it, and open it.
    pub fn create_project(&mut self, base: &std::path::Path, name: &str) {
        use crate::db::connection;
        use crate::persistence::registry::ProjectEntry;

        // Create the .filament/ directory and an empty canvas.db.
        let db_path = crate::persistence::project::db_path(base);
        match connection::open(&db_path) {
            Ok(_) => {}
            Err(e) => { self.status_message = Some(format!("init error: {e}")); return; }
        }

        // Write initial project.toml.
        let settings = ProjectSettings::new(name);
        if let Err(e) = settings.save(base) {
            self.status_message = Some(format!("init error: {e}"));
            return;
        }

        // Register and open.
        self.registry.add_project(ProjectEntry {
            name: name.to_string(),
            path: base.to_string_lossy().into_owned(),
        });
        let _ = self.registry.save();
        self.load_project(base);
    }

    pub fn init_main_menu_nodes(&mut self) {
        self.nodes.clear();
        
        // 1. Root: Open
        self.nodes.push(Node {
            label: "📁 Open".to_string(),
            parent: None,
            children: vec![],
            links: vec![],
            collapsed: false, // Start expanded
            row: 0,
            world_x: 0,
            world_y: 0,
            world_x_end: 0,
        });

        // Add projects as children of "Open"
        let open_idx = 0;
        for (i, entry) in self.registry.projects.iter().enumerate() {
            let child_idx = self.nodes.len();
            self.nodes.push(Node {
                label: format!("{}  ({})", entry.name, entry.path),
                parent: Some(open_idx),
                children: vec![],
                links: vec![],
                collapsed: false,
                row: i + 1,
                world_x: 0,
                world_y: (i + 1) as i32,
                world_x_end: 0,
            });
            self.nodes[open_idx].children.push(child_idx);
        }

        // 2. Root: New
        self.nodes.push(Node {
            label: "+ New".to_string(),
            parent: None,
            children: vec![],
            links: vec![],
            collapsed: false,
            row: self.nodes.len(),
            world_x: 0,
            world_y: self.nodes.len() as i32,
            world_x_end: 0,
        });

        // 3. Root: Find
        self.nodes.push(Node {
            label: "🔍 Find".to_string(),
            parent: None,
            children: vec![],
            links: vec![],
            collapsed: false,
            row: self.nodes.len(),
            world_x: 0,
            world_y: self.nodes.len() as i32,
            world_x_end: 0,
        });

        // 4. Root: Settings
        let settings_idx = self.nodes.len();
        self.nodes.push(Node {
            label: "⚙ Settings".to_string(),
            parent: None,
            children: vec![],
            links: vec![],
            collapsed: true, // Start collapsed
            row: settings_idx,
            world_x: 0,
            world_y: settings_idx as i32,
            world_x_end: 0,
        });
        
        // Add specific settings as children of "Settings"
        let path_idx = self.nodes.len();
        self.nodes.push(Node {
            label: format!("Default filaments path: {}", self.settings.default_projects_path),
            parent: Some(settings_idx),
            children: vec![],
            links: vec![],
            collapsed: false,
            row: path_idx,
            world_x: 0,
            world_y: path_idx as i32,
            world_x_end: 0,
        });
        self.nodes[settings_idx].children.push(path_idx);

        let user_idx = self.nodes.len();
        self.nodes.push(Node {
            label: format!("Username: {}", self.settings.username),
            parent: Some(settings_idx),
            children: vec![],
            links: vec![],
            collapsed: false,
            row: user_idx,
            world_x: 0,
            world_y: user_idx as i32,
            world_x_end: 0,
        });
        self.nodes[settings_idx].children.push(user_idx);

        self.selected = 0;
    }

    pub fn quit_to_main_menu(&mut self) {
        self.save_project();
        self.project_path = None;
        self.project_name = String::new();
        self.init_main_menu_nodes();
        self.mode = Mode::StartMenu { state: StartMenuState::Main { selected: 0 } };
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
