use super::{App, Mode};
use super::mode::StartMenuState;
use crate::models::node::Node;

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
                let default_name = std::path::Path::new(&path_str)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("project")
                    .to_string();
                let cursor = default_name.len();
                
                // Update the temporary node label for the next step
                self.nodes[self.selected].label = format!("name: {}", default_name);
                
                self.mode = Mode::StartMenu { state: StartMenuState::NewName {
                    path:   path_str,
                    buf:    default_name,
                    cursor,
                }};
            }
            Mode::StartMenu { state: StartMenuState::NewName { path, buf, .. } } => {
                let path_str = path.clone();
                let name = if buf.trim().is_empty() {
                    std::path::Path::new(&path_str)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("project")
                        .to_string()
                } else {
                    buf.trim().to_string()
                };
                let base = std::path::PathBuf::from(&path_str);
                self.create_project(&base, &name);
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
        // Keep the parent expanded
        self.nodes[0].collapsed = false;
        self.selected = 0;
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

    // ── New project input ─────────────────────────────────────────────────────

    pub fn start_menu_start_new(&mut self) {
        let new_node_idx = self.nodes.iter().position(|n| n.label.contains("+ New")).unwrap_or(0);
        let child_idx = self.nodes.len();
        
        self.nodes.push(Node {
            label: format!("path: {}", self.settings.default_projects_path),
            parent: Some(new_node_idx),
            children: vec![],
            links: vec![],
            collapsed: false,
            row: 0,
            world_x: 0,
            world_y: 0,
            world_x_end: 0,
        });
        self.nodes[new_node_idx].children.push(child_idx);
        self.nodes[new_node_idx].collapsed = false;
        self.selected = child_idx;

        self.mode = Mode::StartMenu { state: StartMenuState::NewPath {
            buf:    self.settings.default_projects_path.clone(),
            cursor: self.settings.default_projects_path.len(),
        }};
    }

    pub fn start_menu_input_char(&mut self, c: char) {
        match &mut self.mode {
            Mode::StartMenu { state: StartMenuState::NewPath { buf, cursor } }
            | Mode::StartMenu { state: StartMenuState::NewName { buf, cursor, .. } }
            | Mode::StartMenu { state: StartMenuState::EditSetting { buf, cursor, .. } } => {
                buf.insert(*cursor, c);
                *cursor += c.len_utf8();
            }
            _ => {}
        }
    }

    pub fn start_menu_backspace(&mut self) {
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
            _ => {}
        }
    }

    pub fn start_menu_move_cursor(&mut self, delta: i32) {
        match &mut self.mode {
            Mode::StartMenu { state: StartMenuState::NewPath { buf, cursor } }
            | Mode::StartMenu { state: StartMenuState::NewName { buf, cursor, .. } }
            | Mode::StartMenu { state: StartMenuState::EditSetting { buf, cursor, .. } } => {
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
