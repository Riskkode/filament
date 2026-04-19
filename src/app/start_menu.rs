use super::{App, Mode};
use super::mode::StartMenuState;
use crate::models::node::Node;
use std::collections::HashMap;

impl App {
    // ── Navigation ────────────────────────────────────────────────────────────

    pub fn start_menu_nav(&mut self, delta: i32) {
        let order = self.recompute_layout();
        let count = order.len();
        if count == 0 { return; }

        let current_pos = order.iter().position(|&(id, _)| id == self.selected).unwrap_or(0);
        let next_pos = (current_pos as i32 + delta).clamp(0, count as i32 - 1) as usize;
        self.selected = order[next_pos].0;

        if let Mode::StartMenu { state: StartMenuState::Main { ref mut selected } } = self.mode {
            *selected = self.selected;
        }
    }

    pub fn start_menu_go_to_label(&mut self, target: &str) {
        let target_lower = if target == "new" { "+ new".to_string() } 
            else if target == "help" { "help".to_string() }
            else { target.to_lowercase() };
        if let Some((idx, _)) = self.nodes.iter().enumerate()
            .find(|(_, n)| n.parent.is_none() && n.label.to_lowercase().contains(&target_lower))
        {
            self.selected = idx;
            if let Mode::StartMenu { state: StartMenuState::Main { ref mut selected } } = self.mode {
                *selected = idx;
            }
            self.start_menu_confirm();
        }
    }

    pub fn start_menu_confirm(&mut self) {
        let node_id = self.selected;
        let has_children = !self.nodes[node_id].children.is_empty();

        if has_children {
            self.nodes[node_id].collapsed = !self.nodes[node_id].collapsed;
            return;
        }

        // Action-based confirmation
        match &mut self.mode {
            Mode::StartMenu { state: StartMenuState::Main { .. } } => {
                let label = self.nodes[node_id].label.to_lowercase();
                if label.contains("find") {
                    self.status_message = Some("Find not implemented yet".to_string());
                } else if label.contains("new") {
                    self.start_menu_start_new();
                } else if label.contains("import") {
                    self.start_menu_import();
                } else if label.contains("help") {
                    self.canvas_start_help();
                } else if label.contains("default filaments path") || label.contains("username") {
                    self.start_menu_edit();
                } else if let Some(parent) = self.nodes[node_id].parent {
                    if self.nodes[parent].label.to_lowercase().contains("open") {
                        // This is a project node
                        let project_idx = self.nodes[parent].children.iter().position(|&id| id == node_id).unwrap();
                        let path = match self.registry.projects.get(project_idx) {
                            Some(e) => std::path::PathBuf::from(&e.path),
                            None    => return,
                        };
                        self.load_project(&path);
                    } else if self.nodes[parent].label.to_lowercase().contains("themes") {
                        let theme_name = self.nodes[node_id].label.trim_start_matches("• ").to_string();
                        self.settings.palette = theme_name.clone();
                        self.palette = crate::ui::palette::get_palette(&theme_name);
                        let _ = self.settings.save();
                        self.init_main_menu_nodes();
                    }
                }
            }
            Mode::StartMenu { state: StartMenuState::EditSetting { key, buf, .. } } => {
                if key.starts_with("rename_project:") {
                    let idx_str = key.trim_start_matches("rename_project:");
                    if let Ok(idx) = idx_str.parse::<usize>() {
                        if let Some(entry) = self.registry.projects.get_mut(idx) {
                            entry.name = buf.clone();
                            let _ = self.registry.save();
                        }
                    }
                } else if key.starts_with("repath_project:") {
                    let idx_str = key.trim_start_matches("repath_project:");
                    if let Ok(idx) = idx_str.parse::<usize>() {
                        if let Some(entry) = self.registry.projects.get_mut(idx) {
                            entry.path = buf.clone();
                            let _ = self.registry.save();
                        }
                    }
                } else {
                    match key.as_str() {
                        "default_projects_path" => self.settings.default_projects_path = buf.clone(),
                        "username" => self.settings.username = buf.clone(),
                        _ => {}
                    }
                    let _ = self.settings.save();
                }
                self.init_main_menu_nodes();
                self.mode = Mode::StartMenu { state: StartMenuState::Main { selected: self.selected } };
            }
            Mode::StartMenu { state: StartMenuState::NewPath { buf, .. } } => {
                let raw = buf.trim().to_string();
                if raw.is_empty() {
                    self.start_menu_cancel();
                    return;
                }
                let path_str = if raw.starts_with("~/") || raw == "~" {
                    dirs::home_dir()
                        .map(|h| h.join(raw.trim_start_matches("~/")).to_string_lossy().into_owned())
                        .unwrap_or(raw.clone())
                } else {
                    raw.clone()
                };
                let default_name = "project".to_string();
                let cursor = default_name.len();
                
                // Update the temporary node label for the next step
                self.nodes[self.selected].label = format!("project: {}", default_name);
                
                self.mode = Mode::StartMenu { state: StartMenuState::NewName {
                    path:   path_str,
                    buf:    default_name,
                    cursor,
                }};
            }
            Mode::StartMenu { state: StartMenuState::NewName { path, buf, .. } } => {
                let path_str = path.clone();
                let name = if buf.trim().is_empty() {
                    "project".to_string()
                } else {
                    buf.trim().to_string()
                };
                let base = std::path::PathBuf::from(&path_str);
                self.create_project(&base, &name);
            }
            Mode::StartMenu { state: StartMenuState::Import { matches, match_idx, .. } } => {
                if let Some(path_str) = matches.get(*match_idx) {
                    let path = std::path::PathBuf::from(path_str);
                    if let Ok(settings) = crate::persistence::project::ProjectSettings::load(&path) {
                        use crate::persistence::registry::ProjectEntry;
                        self.registry.add_project(ProjectEntry {
                            name: settings.name,
                            path: path_str.clone(),
                        });
                        let _ = self.registry.save();
                        self.load_project(&path);
                    } else {
                        self.status_message = Some("Invalid project file".into());
                        self.mode = Mode::StartMenu { state: StartMenuState::Main { selected: self.selected } };
                    }
                } else {
                    self.mode = Mode::StartMenu { state: StartMenuState::Main { selected: self.selected } };
                }
            }
            _ => {}
        }
    }

    pub fn start_menu_cancel(&mut self) {
        if self.project_path.is_some() {
            self.mode = Mode::Canvas { state: super::CanvasState::Browse };
        } else {
            // If in a sub-input state, just reset to main menu roots
            self.init_main_menu_nodes();
            self.mode = Mode::StartMenu { state: StartMenuState::Main { selected: 0 } };
        }
    }

    pub fn start_menu_remove_selected(&mut self) {
        let node_id = self.selected;
        let Some(parent) = self.nodes[node_id].parent else { return };
        if !self.nodes[parent].label.to_lowercase().contains("open") { return; }

        let project_idx = self.nodes[parent].children.iter().position(|&id| id == node_id).unwrap();
        self.registry.remove_at(project_idx);
        let _ = self.registry.save();
        
        // Re-initialize to refresh the list
        self.init_main_menu_nodes();
        
        // Try to keep the same selection position or previous one if we deleted the last
        let new_count = self.registry.projects.len();
        if new_count > 0 {
            let open_node_idx = 0;
            let next_idx = project_idx.min(new_count - 1);
            self.selected = self.nodes[open_node_idx].children[next_idx];
        } else {
            self.selected = 0;
        }

        if let Mode::StartMenu { state: StartMenuState::Main { ref mut selected } } = self.mode {
            *selected = self.selected;
        }
    }

    pub fn start_menu_edit(&mut self) {
        let node_id = self.selected;
        let label = self.nodes[node_id].label.to_lowercase();

        if label.contains("default filaments path") {
            let buf = self.settings.default_projects_path.clone();
            let cursor = buf.len();
            self.mode = Mode::StartMenu { state: StartMenuState::EditSetting {
                key: "default_projects_path".to_string(),
                buf,
                cursor,
            }};
        } else if label.contains("username") {
            let buf = self.settings.username.clone();
            let cursor = buf.len();
            self.mode = Mode::StartMenu { state: StartMenuState::EditSetting {
                key: "username".to_string(),
                buf,
                cursor,
            }};
        } else if let Some(parent) = self.nodes[node_id].parent {
            if self.nodes[parent].label.to_lowercase().contains("open") {
                // Project node: edit name
                let project_idx = self.nodes[parent].children.iter().position(|&id| id == node_id).unwrap();
                if let Some(entry) = self.registry.projects.get(project_idx) {
                    let buf = entry.name.clone();
                    let cursor = buf.len();
                    self.mode = Mode::StartMenu { state: StartMenuState::EditSetting {
                        key: format!("rename_project:{}", project_idx),
                        buf,
                        cursor,
                    }};
                }
            }
        }
    }

    pub fn start_menu_edit_path(&mut self) {
        let node_id = self.selected;
        if let Some(parent) = self.nodes[node_id].parent {
            if self.nodes[parent].label.to_lowercase().contains("open") {
                // Project node: edit path
                let project_idx = self.nodes[parent].children.iter().position(|&id| id == node_id).unwrap();
                if let Some(entry) = self.registry.projects.get(project_idx) {
                    let buf = entry.path.clone();
                    let cursor = buf.len();
                    self.mode = Mode::StartMenu { state: StartMenuState::EditSetting {
                        key: format!("repath_project:{}", project_idx),
                        buf,
                        cursor,
                    }};
                }
            }
        }
    }

    // ── New project input ─────────────────────────────────────────────────────

    pub fn start_menu_start_new(&mut self) {
        let new_node_idx = self.nodes.iter().position(|n| n.label.contains("+ New")).unwrap_or(0);
        let child_idx = self.nodes.len();
        
        self.nodes.push(Node {
            label: format!("location: {}", self.settings.default_projects_path),
            parent: Some(new_node_idx),
            children: vec![],
            links: vec![],
            collapsed: false,
            row: 0,
            world_x: 0,
            world_y: 0,
            world_x_end: 0,
            tags: HashMap::new(),
        });
        self.nodes[new_node_idx].children.push(child_idx);
        self.nodes[new_node_idx].collapsed = false;
        self.selected = child_idx;

        self.mode = Mode::StartMenu { state: StartMenuState::NewPath {
            buf:    self.settings.default_projects_path.clone(),
            cursor: self.settings.default_projects_path.len(),
        }};
    }

    pub fn start_menu_import(&mut self) {
        let root = self.settings.default_projects_path.clone();
        let buf = String::new();
        let matches = self.start_menu_import_scan(&root, "");
        self.mode = Mode::StartMenu { state: StartMenuState::Import {
            root,
            buf,
            cursor: 0,
            matches,
            match_idx: 0,
            editing_root: false,
        }};
    }

    fn start_menu_import_scan(&self, root_path: &str, pattern: &str) -> Vec<String> {
        let root = if root_path.starts_with("~/") || root_path == "~" {
            dirs::home_dir().map(|h| h.join(root_path.trim_start_matches("~/")))
                .unwrap_or(std::path::PathBuf::from(root_path))
        } else {
            std::path::PathBuf::from(root_path)
        };

        if !root.exists() { return vec![]; }

        let mut results = Vec::new();
        let pattern_lower = pattern.to_lowercase();
        
        let walker = ignore::WalkBuilder::new(root)
            .hidden(false)
            .filter_entry(|e| {
                // Don't descend into already found project dirs or hidden dirs (except .filament itself)
                let name = e.file_name().to_string_lossy();
                if name.starts_with('.') && name != ".filament" { return false; }
                true
            })
            .build();

        for entry in walker.flatten() {
            if entry.file_name() == ".filament" && entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(parent) = entry.path().parent() {
                    let path_str = parent.to_string_lossy().into_owned();
                    if pattern.is_empty() || path_str.to_lowercase().contains(&pattern_lower) {
                        results.push(path_str);
                    }
                }
            }
            if results.len() >= 20 { break; }
        }
        results
    }

    pub fn start_menu_import_toggle_root(&mut self) {
        if let Mode::StartMenu { state: StartMenuState::Import { ref mut buf, ref mut cursor, ref mut editing_root, ref root, .. } } = self.mode {
            *editing_root = !*editing_root;
            if *editing_root {
                *buf = root.clone();
            } else {
                *buf = String::new();
            }
            *cursor = buf.len();
        }
    }

    pub fn start_menu_import_next(&mut self) {
        if let Mode::StartMenu { state: StartMenuState::Import { ref matches, ref mut match_idx, .. } } = self.mode {
            if !matches.is_empty() {
                *match_idx = (*match_idx + 1) % matches.len();
            }
        }
    }

    pub fn start_menu_import_prev(&mut self) {
        if let Mode::StartMenu { state: StartMenuState::Import { ref matches, ref mut match_idx, .. } } = self.mode {
            if !matches.is_empty() {
                *match_idx = (*match_idx + matches.len() - 1) % matches.len();
            }
        }
    }

    pub fn start_menu_input_char(&mut self, c: char) {
        let mut do_import_scan = false;
        let mut pattern = String::new();

        match &mut self.mode {
            Mode::StartMenu { state: StartMenuState::NewPath { buf, cursor } }
            | Mode::StartMenu { state: StartMenuState::NewName { buf, cursor, .. } }
            | Mode::StartMenu { state: StartMenuState::EditSetting { buf, cursor, .. } } => {
                buf.insert(*cursor, c);
                *cursor += c.len_utf8();
            }
            Mode::StartMenu { state: StartMenuState::Import { buf, cursor, .. } } => {
                buf.insert(*cursor, c);
                *cursor += c.len_utf8();
                pattern = buf.clone();
                do_import_scan = true;
            }
            _ => {}
        }

        if do_import_scan {
            let (new_matches, new_root) = if let Mode::StartMenu { state: StartMenuState::Import { ref root, editing_root, .. } } = self.mode {
                if editing_root {
                    (self.start_menu_import_scan(&pattern, ""), pattern.clone())
                } else {
                    (self.start_menu_import_scan(root, &pattern), root.clone())
                }
            } else {
                (vec![], String::new())
            };

            if let Mode::StartMenu { state: StartMenuState::Import { matches: ref mut m, match_idx: ref mut mi, root: ref mut r, .. } } = self.mode {
                *m = new_matches;
                *r = new_root;
                *mi = 0;
            }
        }
    }

    pub fn start_menu_backspace(&mut self) {
        let mut do_import_scan = false;
        let mut pattern = String::new();

        match &mut self.mode {
            Mode::StartMenu { state: StartMenuState::NewPath { buf, cursor } }
            | Mode::StartMenu { state: StartMenuState::NewName { buf, cursor, .. } }
            | Mode::StartMenu { state: StartMenuState::EditSetting { buf, cursor, .. } } => {
                if *cursor > 0 {
                    let ch = buf[..*cursor].chars().last().unwrap();
                    *cursor -= ch.len_utf8();
                    buf.remove(*cursor);
                }
            }
            Mode::StartMenu { state: StartMenuState::Import { buf, cursor, .. } } => {
                if *cursor > 0 {
                    let ch = buf[..*cursor].chars().last().unwrap();
                    *cursor -= ch.len_utf8();
                    buf.remove(*cursor);
                    pattern = buf.clone();
                    do_import_scan = true;
                }
            }
            _ => {}
        }

        if do_import_scan {
            let (new_matches, new_root) = if let Mode::StartMenu { state: StartMenuState::Import { ref root, editing_root, .. } } = self.mode {
                if editing_root {
                    (self.start_menu_import_scan(&pattern, ""), pattern.clone())
                } else {
                    (self.start_menu_import_scan(root, &pattern), root.clone())
                }
            } else {
                (vec![], String::new())
            };

            if let Mode::StartMenu { state: StartMenuState::Import { matches: ref mut m, match_idx: ref mut mi, root: ref mut r, .. } } = self.mode {
                *m = new_matches;
                *r = new_root;
                *mi = 0;
            }
        }
    }

    pub fn start_menu_move_cursor(&mut self, delta: i32) {
        match &mut self.mode {
            Mode::StartMenu { state: StartMenuState::NewPath { buf, cursor } }
            | Mode::StartMenu { state: StartMenuState::NewName { buf, cursor, .. } }
            | Mode::StartMenu { state: StartMenuState::EditSetting { buf, cursor, .. } }
            | Mode::StartMenu { state: StartMenuState::Import { buf, cursor, .. } } => {
                move_cursor_in_buf(buf, cursor, delta);
            }
            _ => {}
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn move_cursor_in_buf(buf: &str, cursor: &mut usize, delta: i32) {
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
