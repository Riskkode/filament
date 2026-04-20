mod mode;
mod canvas;
mod delete;
mod input;
mod start_menu;
mod reparent;
pub mod query;

pub use mode::{ArrowFidelity, ArrowSettings, CanvasState, InputAction, Mode, StartMenuState, StatusPageState, ArchiveState};

use crate::models::node::{Node, ManagedNodeType};
use crate::models::query::StatusQuery;
use crate::models::archive_note::{ArchiveNote, NoteType};
use crate::persistence::project::ProjectSettings;
use crate::persistence::registry::Registry;
use crate::persistence::settings::GlobalSettings;
use crate::ui::palette::{Palette, get_palette};
use std::path::PathBuf;
use std::collections::{HashSet, HashMap};
use chrono::{Local, Utc, TimeZone};

fn format_duration(seconds: i64) -> String {
    let mut s = seconds.abs();
    let days = s / 86400;
    s %= 86400;
    let hours = s / 3600;
    s %= 3600;
    let minutes = s / 60;
    let secs = s % 60;

    let mut parts = Vec::new();
    if days > 0 { parts.push(format!("{}d", days)); }
    if hours > 0 { parts.push(format!("{}h", hours)); }
    if minutes > 0 { parts.push(format!("{}m", minutes)); }
    if secs > 0 || parts.is_empty() { parts.push(format!("{}s", secs)); }

    let joined = parts.join(" ");
    if seconds < 0 { format!("-{}", joined) } else { joined }
}

pub struct App {
    pub arrow:    ArrowSettings,
    pub nodes:    Vec<Node>,
    /// User defined queries for the Status page.
    pub queries:  Vec<StatusQuery>,
    /// User defined notes for the Archive page.
    pub notes:    Vec<ArchiveNote>,
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
    pub palette:  Palette,
    /// Transient message shown in the status bar (errors, hints).
    pub status_message: Option<String>,
    /// Stack of previous node states for undo.
    pub undo_stack: Vec<Vec<Node>>,
    /// Track last link origin and index for Tab cycling.
    pub last_link_origin: Option<usize>,
    pub last_link_idx:    usize,
    /// Track previous mode for Help restoration.
    pub help_previous_mode: Option<Mode>,
    pub show_note_previews: bool,
}

impl App {
    pub fn build_concatenated_document(&self, note_idx: usize) -> String {
        let note_title = &self.notes[note_idx].title;
        
        // 1. Find the first node on canvas that points to this note
        let Some(root_node_idx) = self.nodes.iter().enumerate().find(|(_, n)| {
            n.is_managed_note && &n.label == note_title
        }).and_then(|(i, _)| self.nodes[i].parent) else {
            return self.notes[note_idx].content.clone();
        };

        // 2. Recursively build content
        let mut result = String::new();
        self.collect_reference_notes_recursive(root_node_idx, 1, &mut result);
        result
    }

    fn collect_reference_notes_recursive(&self, node_idx: usize, depth: usize, out: &mut String) {
        // Find if this node has a Reference Note child
        let ref_note = self.nodes[node_idx].children.iter()
            .map(|&cid| &self.nodes[cid])
            .filter(|n| n.is_managed_note)
            .find_map(|mn| self.notes.iter().find(|n| n.title == mn.label && n.note_type == crate::models::archive_note::NoteType::Reference));

        if let Some(note) = ref_note {
            // Add Markdown heading based on depth
            let heading = "#".repeat(depth.min(6));
            if !out.is_empty() { out.push_str("\n\n"); }
            out.push_str(&format!("{} {}\n\n", heading, note.title));
            out.push_str(&note.content);
        }

        // Recursively process children
        for &cid in &self.nodes[node_idx].children {
            if !self.nodes[cid].is_managed_note {
                self.collect_reference_notes_recursive(cid, depth + 1, out);
            }
        }
    }

    pub fn new() -> Self {
        let settings = GlobalSettings::load();
        let palette = get_palette(&settings.palette);
        let mut app = Self {
            arrow:    ArrowSettings::default(),
            nodes:    vec![],
            queries:  vec![],
            notes:    vec![],
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
            settings,
            palette,
            status_message:     None,
            undo_stack:         vec![],
            last_link_origin:   None,
            last_link_idx:      0,
            help_previous_mode: None,
            show_note_previews: true,
        };
        app.init_main_menu_nodes();
        app
    }

    pub fn sync_managed_nodes(&mut self) {
        let mut managed_ids: HashSet<usize> = self.nodes.iter().enumerate()
            .filter(|(_, n)| n.managed.is_some())
            .map(|(i, _)| i)
            .collect();

        // 1. Ensure all nodes have their managed groups/tags
        let n = self.nodes.len();
        for i in 0..n {
            // Only process real nodes that have times
            if self.nodes[i].managed.is_some() || self.nodes[i].times.is_empty() { continue; }
            
            let group_idx = if let Some(&g) = self.nodes[i].children.iter().find(|&&c| {
                if c < self.nodes.len() {
                    matches!(self.nodes[c].managed, Some(ManagedNodeType::TimeGroup))
                } else { false }
            }) {
                managed_ids.remove(&g);
                g
            } else {
                let new_idx = self.nodes.len();
                let group = Node {
                    label: "[Times]".to_string(),
                    parent: Some(i),
                    children: vec![],
                    links: vec![],
                    collapsed: false,
                    row: usize::MAX,
                    world_x: 0,
                    world_y: 0,
                    world_x_end: 0,
                    tags: HashMap::new(),
                    times: HashMap::new(),
                    is_managed_note: false,
                    managed: Some(ManagedNodeType::TimeGroup),
                };
                self.nodes.push(group);
                self.nodes[i].children.push(new_idx);
                new_idx
            };

            let keys: Vec<String> = self.nodes[i].times.keys().cloned().collect();
            for key in keys {
                let tag = self.nodes[i].times.get(&key).unwrap();
                let label = if key == "duration" {
                    format!("duration: {}", format_duration(tag.timestamp))
                } else {
                    let dt = Local.timestamp_opt(tag.timestamp, 0).unwrap();
                    let fmt = if tag.pattern.is_some() { "%Y-%m-%d %H:%M (recurring)" } else { "%Y-%m-%d %H:%M" };
                    format!("{}: {}", key, dt.format(fmt))
                };

                if let Some(&t) = self.nodes[group_idx].children.iter().find(|&&c| {
                    if c < self.nodes.len() {
                        if let Some(ManagedNodeType::TimeTag { key: k }) = &self.nodes[c].managed {
                            k == &key
                        } else { false }
                    } else { false }
                }) {
                    managed_ids.remove(&t);
                    self.nodes[t].label = label;
                } else {
                    let new_tag_idx = self.nodes.len();
                    let tag_node = Node {
                        label,
                        parent: Some(group_idx),
                        children: vec![],
                        links: vec![],
                        collapsed: false,
                        row: usize::MAX,
                        world_x: 0,
                        world_y: 0,
                        world_x_end: 0,
                        tags: HashMap::new(),
                        times: HashMap::new(),
                        is_managed_note: false,
                        managed: Some(ManagedNodeType::TimeTag { key }),
                    };
                    self.nodes.push(tag_node);
                    self.nodes[group_idx].children.push(new_tag_idx);
                }
            }
        }

        // 2. Remove any managed nodes that were not reconciled (orphans)
        if !managed_ids.is_empty() {
            self.remove_nodes(&managed_ids);
        }
    }

    pub fn push_undo(&mut self) {
        // Only push if we have nodes (don't undo the start menu state)
        if self.project_path.is_none() { return; }
        
        // Cap stack size at 50 to prevent excessive memory usage
        if self.undo_stack.len() >= 50 {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(self.nodes.clone());
        self.save_project();
    }

    pub fn undo(&mut self) {
        if let Some(prev_nodes) = self.undo_stack.pop() {
            self.nodes = prev_nodes;
            self.recompute_layout();
            self.save_project();

            // When we undo, save_project pushes the current state to history.
            // But we actually want to pop the history in the DB too.
            // Since save_project just pushed, the "undone" state is now at the top.
            // We should remove the entry that was just pushed by save_project,
            // AND the entry that we just popped from self.undo_stack.
            // Actually, if we just want to revert, removing the last 2 entries from DB 
            // after save_project() might be what's needed to stay in sync.
            if let Some(base) = &self.project_path {
                let db_path = crate::persistence::project::db_path(base);
                if let Ok(conn) = crate::db::connection::open(&db_path) {
                    let _ = conn.execute("DELETE FROM history WHERE id IN (SELECT id FROM history ORDER BY id DESC LIMIT 2)", []);
                }
            }

            self.status_message = Some("Undo applied".to_string());
        } else {
            self.status_message = Some("Nothing to undo".to_string());
        }
    }

    // ── Persistence ───────────────────────────────────────────────────────────

    /// Open an existing project: reads `project.toml` and `canvas.db` from
    /// `<base>/.filament/` and applies the state to `self`.
    pub fn load_project(&mut self, base: &std::path::Path) {
        use crate::db::connection;
        use crate::repositories::{node_repository, query_repository, note_repository};

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

        self.queries = query_repository::load_queries(&conn).unwrap_or_default();
        self.notes   = note_repository::load_notes(&conn).unwrap_or_default();
        self.nodes    = nodes;
        self.camera_x = settings.view.camera_x;
        self.camera_y = settings.view.camera_y;
        self.cursor_x = settings.view.cursor_x;
        self.cursor_y = settings.view.cursor_y;
        self.show_note_previews = settings.view.show_note_previews;
        self.selected = id_to_idx.get(&settings.view.selected_db_id).copied().unwrap_or(0);
        self.arrow    = ArrowSettings {
            global:   settings.arrows.global,
            incoming: if settings.arrows.incoming == "Selected" { ArrowFidelity::Selected } else { ArrowFidelity::Tree },
            outgoing: if settings.arrows.outgoing == "Selected" { ArrowFidelity::Selected } else { ArrowFidelity::Tree },
        };
        self.project_path       = Some(base.to_path_buf());
        self.project_name       = settings.name;
        self.project_created_at = settings.created_at;

        if let Ok(history) = node_repository::load_history(&conn) {
            self.undo_stack = history;
        }

        self.status_message     = None;
        self.mode               = Mode::Canvas { state: CanvasState::Browse };
        self.sync_managed_nodes();
    }

    /// Persist the current canvas state to `<project_path>/.filament/`.
    /// Silent no-op when no project is open.
    pub fn save_project(&mut self) {
        use crate::db::connection;
        use crate::repositories::{node_repository, query_repository, note_repository};

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

        let _ = query_repository::save_queries(&conn, &self.queries);
        let _ = note_repository::save_notes(&conn, &self.notes);
        
        // Save to history too
        let _ = node_repository::push_history(&conn, &self.nodes);

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
                show_note_previews: self.show_note_previews,
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

        let project_root = base.join(name);

        // Create the .filament/ directory and an empty canvas.db.
        let db_path = crate::persistence::project::db_path(&project_root);
        match connection::open(&db_path) {
            Ok(_) => {}
            Err(e) => { self.status_message = Some(format!("init error: {e}")); return; }
        }

        // Write initial project.toml.
        let settings = ProjectSettings::new(name);
        if let Err(e) = settings.save(&project_root) {
            self.status_message = Some(format!("init error: {e}"));
            return;
        }

        // Register and open.
        self.registry.add_project(ProjectEntry {
            name: name.to_string(),
            path: project_root.to_string_lossy().into_owned(),
        });
        let _ = self.registry.save();
        self.load_project(&project_root);
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
            tags: HashMap::new(),
            times: HashMap::new(),
            is_managed_note: false,
            managed: None,
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
                tags: HashMap::new(),
                times: HashMap::new(),
                is_managed_note: false,
                managed: None,
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
            tags: HashMap::new(),
            times: HashMap::new(),
            is_managed_note: false,
            managed: None,
        });

        // 3. Root: Import
        self.nodes.push(Node {
            label: "📥 Import".to_string(),
            parent: None,
            children: vec![],
            links: vec![],
            collapsed: false,
            row: self.nodes.len(),
            world_x: 0,
            world_y: self.nodes.len() as i32,
            world_x_end: 0,
            tags: HashMap::new(),
            times: HashMap::new(),
            is_managed_note: false,
            managed: None,
        });

        // 4. Root: Find
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
            tags: HashMap::new(),
            times: HashMap::new(),
            is_managed_note: false,
            managed: None,
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
            tags: HashMap::new(),
            times: HashMap::new(),
            is_managed_note: false,
            managed: None,
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
            tags: HashMap::new(),
            times: HashMap::new(),
            is_managed_note: false,
            managed: None,
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
            tags: HashMap::new(),
            times: HashMap::new(),
            is_managed_note: false,
            managed: None,
        });
        self.nodes[settings_idx].children.push(user_idx);

        // Themes
        let themes_idx = self.nodes.len();
        self.nodes.push(Node {
            label: "🎨 Themes".to_string(),
            parent: Some(settings_idx),
            children: vec![],
            links: vec![],
            collapsed: true,
            row: themes_idx,
            world_x: 0,
            world_y: themes_idx as i32,
            world_x_end: 0,
            tags: HashMap::new(),
            times: HashMap::new(),
            is_managed_note: false,
            managed: None,
        });
        self.nodes[settings_idx].children.push(themes_idx);

        for palette in crate::ui::palette::load_all() {
            let p_idx = self.nodes.len();
            let label = if palette.name == self.settings.palette {
                format!("• {}", palette.name)
            } else {
                palette.name.clone()
            };
            self.nodes.push(Node {
                label,
                parent: Some(themes_idx),
                children: vec![],
                links: vec![],
                collapsed: false,
                row: p_idx,
                world_x: 0,
                world_y: p_idx as i32,
                world_x_end: 0,
                tags: HashMap::new(),
                times: HashMap::new(),
                is_managed_note: false,
                managed: None,
            });
            self.nodes[themes_idx].children.push(p_idx);
        }

        // 5. Root: Help
        self.nodes.push(Node {
            label: "❓ Help".to_string(),
            parent: None,
            children: vec![],
            links: vec![],
            collapsed: false,
            row: self.nodes.len(),
            world_x: 0,
            world_y: self.nodes.len() as i32,
            world_x_end: 0,
            tags: HashMap::new(),
            times: HashMap::new(),
            is_managed_note: false,
            managed: None,
        });
    }

    pub fn quit_to_main_menu(&mut self) {
        self.save_project();
        self.project_path = None;
        self.project_name = String::new();
        self.init_main_menu_nodes();
        self.selected = 0;
        self.mode = Mode::StartMenu { state: StartMenuState::Main { selected: 0 } };
    }

    pub fn has_selection(&self) -> bool {
        self.selected < self.nodes.len()
            && self.node_near_cursor(self.cursor_x, self.cursor_y).is_some()
    }

    // ── Queries ──────────────────────────────────────────────────────────────

    pub fn status_add_query(&mut self, name: String, logic: String) {
        self.queries.push(StatusQuery { id: None, name, logic });
        self.save_project();
    }

    pub fn status_remove_query(&mut self, idx: usize) {
        if idx < self.queries.len() {
            self.queries.remove(idx);
            self.save_project();
        }
    }

    pub fn status_update_query(&mut self, idx: usize, name: String, logic: String) {
        if idx < self.queries.len() {
            self.queries[idx].name = name;
            self.queries[idx].logic = logic;
            self.save_project();
        }
    }

    pub fn get_query_results(&self, query_idx: usize) -> Vec<usize> {
        if query_idx >= self.queries.len() { return vec![]; }
        let logic = &self.queries[query_idx].logic;
        self.nodes.iter().enumerate()
            .filter(|(_, n)| crate::app::query::evaluate(logic, n))
            .map(|(i, _)| i)
            .collect()
    }

    // ── Archive ──────────────────────────────────────────────────────────────

    pub fn archive_add_note(&mut self, title: String, content: String) {
        self.notes.push(ArchiveNote { id: None, title, content, note_type: NoteType::Quick });
        self.save_project();
    }

    pub fn archive_remove_note(&mut self, idx: usize) {
        if idx < self.notes.len() {
            self.notes.remove(idx);
            self.save_project();
        }
    }

    pub fn archive_update_note(&mut self, idx: usize, title: String, content: String) {
        if idx < self.notes.len() {
            self.notes[idx].title = title;
            self.notes[idx].content = content;
            self.save_project();
        }
    }

    pub fn archive_nav(&mut self, delta: i32) {
        if let Mode::ArchivePage { state: ArchiveState::BrowseList { selected }, .. } = self.mode {
            if self.notes.is_empty() { return; }
            let next = (selected as i32 + delta).rem_euclid(self.notes.len() as i32) as usize;
            self.mode = Mode::ArchivePage { state: ArchiveState::BrowseList { selected: next }, previous: Box::new(self.mode.clone()) };
        }
    }

    pub fn archive_new_note(&mut self) {
        let idx = self.notes.len();
        self.notes.push(ArchiveNote { id: None, title: "New Note".to_string(), content: String::new(), note_type: NoteType::Quick });
        self.mode = Mode::ArchivePage { state: ArchiveState::EditTitle { idx, buf: "New Note".to_string(), cursor: 8 }, previous: Box::new(self.mode.clone()) };
        self.save_project();
    }

    pub fn archive_delete_note(&mut self) {
        if let Mode::ArchivePage { state: ArchiveState::BrowseList { selected }, .. } = self.mode {
            if selected < self.notes.len() {
                self.notes.remove(selected);
                let next = if self.notes.is_empty() { 0 } else { selected.min(self.notes.len() - 1) };
                self.mode = Mode::ArchivePage { state: ArchiveState::BrowseList { selected: next }, previous: Box::new(self.mode.clone()) };
                self.save_project();
            }
        }
    }

    pub fn archive_enter_editor(&mut self) {
        if let Mode::ArchivePage { state: ArchiveState::BrowseList { selected }, .. } = self.mode {
            if let Some(note) = self.notes.get(selected) {
                self.mode = Mode::ArchivePage { state: ArchiveState::EditContent { idx: selected, buf: note.content.clone(), cursor: note.content.len() }, previous: Box::new(self.mode.clone()) };
            }
        }
    }

    pub fn archive_edit_title(&mut self) {
        if let Mode::ArchivePage { state: ArchiveState::BrowseList { selected }, .. } = self.mode {
            if let Some(note) = self.notes.get(selected) {
                self.mode = Mode::ArchivePage { state: ArchiveState::EditTitle { idx: selected, buf: note.title.clone(), cursor: note.title.len() }, previous: Box::new(self.mode.clone()) };
            }
        }
    }

    pub fn archive_editor_char(&mut self, c: char) {
        if let Mode::ArchivePage { state: ref mut s, .. } = self.mode {
            match s {
                ArchiveState::EditTitle { buf, cursor, .. } | ArchiveState::EditContent { buf, cursor, .. } => {
                    buf.insert(*cursor, c);
                    *cursor += c.len_utf8();
                }
                _ => {}
            }
        }
    }

    pub fn archive_editor_backspace(&mut self) {
        if let Mode::ArchivePage { state: ref mut s, .. } = self.mode {
            match s {
                ArchiveState::EditTitle { buf, cursor, .. } | ArchiveState::EditContent { buf, cursor, .. } => {
                    if *cursor > 0 {
                        let prev = buf[..*cursor].char_indices().last().map(|(i, _)| i).unwrap_or(0);
                        buf.drain(prev..*cursor);
                        *cursor = prev;
                    }
                }
                _ => {}
            }
        }
    }

    pub fn archive_editor_confirm(&mut self) {
        match self.mode.clone() {
            Mode::ArchivePage { state: ArchiveState::EditTitle { idx, buf, .. }, .. } => {
                let index = idx;
                let title = buf;
                if index < self.notes.len() {
                    self.notes[index].title = title;
                } else {
                    self.notes.push(ArchiveNote { id: None, title, content: String::new(), note_type: NoteType::Quick });
                }
                self.save_project();
                self.mode = Mode::ArchivePage { state: ArchiveState::BrowseList { selected: index }, previous: Box::new(self.mode.clone()) };
            }
            Mode::ArchivePage { state: ArchiveState::EditContent { idx, buf, .. }, .. } => {
                if idx < self.notes.len() {
                    self.notes[idx].content = buf;
                    self.save_project();
                }
                // Enter stays in editor but adds newline?
                // The requirement says: "Enter (newline)"
                // So I should just insert a newline if in EditContent.
                // Wait, if I handle Enter as newline, how do I exit? "Esc (back to list)".
                // OK, so Enter in EditContent is just a character.
                self.archive_editor_char('\n');
            }
            _ => {}
        }
    }

    pub fn archive_editor_save_and_exit(&mut self) {
        match self.mode.clone() {
            Mode::ArchivePage { state: ArchiveState::EditTitle { idx, buf, .. }, .. } => {
                let index = idx;
                let title = buf;
                if index < self.notes.len() {
                    self.notes[index].title = title;
                } else {
                    self.notes.push(ArchiveNote { id: None, title, content: String::new(), note_type: NoteType::Quick });
                }
                self.save_project();
                self.mode = Mode::ArchivePage { state: ArchiveState::BrowseList { selected: index }, previous: Box::new(self.mode.clone()) };
            }
            Mode::ArchivePage { state: ArchiveState::EditContent { idx, buf, .. }, .. } => {
                let index = idx;
                let content = buf;
                if let Some(note) = self.notes.get_mut(index) {
                    note.content = content;
                    self.save_project();
                }
                self.mode = Mode::ArchivePage { state: ArchiveState::BrowseList { selected: index }, previous: Box::new(self.mode.clone()) };
            }
            _ => {}
        }
    }

    pub fn archive_jump_at(&mut self, canvas_w: u16, canvas_h: u16) {
        let content = match &self.mode {
            Mode::ArchivePage { state: ArchiveState::EditContent { buf, .. }, .. } => buf.clone(),
            Mode::ArchivePage { state: ArchiveState::BrowseList { selected }, .. } => {
                self.notes.get(*selected).map(|n| n.content.clone()).unwrap_or_default()
            }
            _ => return,
        };

        // Find the last @mention before the cursor or just any @mention
        // For simplicity, let's look for @ followed by alphanumeric
        if let Some(at_idx) = content.rfind('@') {
            let rest = &content[at_idx+1..];
            let name = rest.split_whitespace().next().unwrap_or("")
                .trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != ' ');
            
            if let Some(node_idx) = self.nodes.iter().position(|n| n.label == name) {
                self.selected = node_idx;
                self.center_on_selected(canvas_w, canvas_h);
                self.mode = Mode::Canvas { state: CanvasState::Browse };
            }
        }
    }

    // ── Layout ────────────────────────────────────────────────────────────────

    pub fn recompute_layout(&mut self) -> Vec<(usize, usize)> {
        // Auto-advance recurring tags
        let now = Local::now();
        let mut changed = false;
        for node in self.nodes.iter_mut() {
            for tag in node.times.values_mut() {
                if let Some(ref pattern) = tag.pattern {
                    if tag.timestamp < now.timestamp() {
                        let parse_buf = pattern.to_lowercase().replace("every", "next");
                        if let Ok(dt) = chrono_english::parse_date_string(&parse_buf, now, chrono_english::Dialect::Uk) {
                            tag.timestamp = dt.with_timezone(&Utc).timestamp();
                            changed = true;
                        }
                    }
                }
            }
        }
        if changed { self.save_project(); }

        for n in self.nodes.iter_mut() { n.row = usize::MAX; }
        let mut order: Vec<(usize, usize)> = Vec::new();

        let roots: Vec<usize> = (0..self.nodes.len())
            .filter(|&i| self.nodes[i].parent.is_none())
            .collect();

        // ── Insertion offset ──────────────────────────────────────────────────
        // We only push down nodes within the SAME hierarchy where the insertion is happening.
        for root in roots {
            let start_row = order.len();
            let mut local: Vec<(usize, usize)> = Vec::new();
            Self::collect_visible_inner(&mut self.nodes, root, 0, &mut local);
            
            let (rx, ry) = (self.nodes[root].world_x, self.nodes[root].world_y);
            
            // Check if we are inserting a child into THIS hierarchy
            let mut root_insertion_row = None;
            if let Mode::Input { action: InputAction::InsertChild { parent }, .. } = &self.mode {
                if let Some(pos) = local.iter().position(|&(id, _)| id == *parent) {
                    // Find the last visible child in the subtree of this parent
                    let subtree_ids: HashSet<usize> = self.collect_subtree(*parent).into_iter().collect();
                    let mut last_idx = pos;
                    for (i, &(id, _)) in local.iter().enumerate().skip(pos + 1) {
                        if subtree_ids.contains(&id) {
                            last_idx = i;
                        } else {
                            break; // local is DFS/topological, so subtrees are contiguous
                        }
                    }
                    root_insertion_row = Some(last_idx + 1);
                }
            }

            for (lr, &(id, depth)) in local.iter().enumerate() {
                let mut node_y = ry + lr as i32;
                
                // Apply offset only to nodes in this specific tree that are below the insertion point
                if let Some(rir) = root_insertion_row {
                    if lr >= rir { node_y += 1; }
                }
                
                self.nodes[id].row       = start_row + lr;
                self.nodes[id].world_x   = rx;
                self.nodes[id].world_y   = node_y;
                let label_cols = self.nodes[id].label.chars().count() as i32;
                let mut decoration_cols = 0;
                if !self.nodes[id].children.is_empty() && self.nodes[id].collapsed {
                    decoration_cols += 5 + self.nodes[id].children.len().to_string().len() as i32;
                }
                if self.nodes[id].tags.contains_key("status") {
                    decoration_cols += 2;
                }
                if self.nodes[id].is_managed_note {
                    decoration_cols += 2;
                }
                if self.nodes[id].collapsed {
                    for (key, tag) in &self.nodes[id].times {
                        let mut val_width = if key == "duration" { 15 } else { 10 };
                        if let Some(ref pattern) = tag.pattern {
                            val_width += 3 + pattern.chars().count() as i32; // " (pattern)"
                        }
                        decoration_cols += 4 + key.chars().count() as i32 + val_width; // " [key: value]"
                    }
                }
                self.nodes[id].world_x_end = rx + depth as i32 * 2 + 2 + label_cols + decoration_cols;
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
        self.push_undo();
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
        self.push_undo();
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
            .map(|(id, n)| {
                let dx = if cx < n.world_x {
                    n.world_x - cx
                } else if cx > n.world_x_end {
                    cx - n.world_x_end
                } else {
                    0
                };
                let dy = (n.world_y - cy).abs();
                (id, dx + dy)
            })
            .filter(|&(_, d)| d <= 4)
            .min_by_key(|&(_, d)| d)
            .map(|(id, _)| id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::node::Node;

    #[test]
    fn test_node_near_cursor_range() {
        let mut app = App::new();
        app.nodes = vec![
            Node {
                label: "Long label".to_string(),
                parent: None,
                children: vec![],
                links: vec![],
                collapsed: false,
                row: 0,
                world_x: 10,
                world_y: 5,
                world_x_end: 30,
                tags: HashMap::new(),
                times: HashMap::new(),
                is_managed_note: false,
                managed: None,
            }
        ];

        // Middle of the range
        assert_eq!(app.node_near_cursor(20, 5), Some(0));
        // Right edge
        assert_eq!(app.node_near_cursor(30, 5), Some(0));
        // Just outside right edge (within fuzzy distance 4)
        assert_eq!(app.node_near_cursor(34, 5), Some(0));
        // Outside right edge (beyond fuzzy distance 4)
        assert_eq!(app.node_near_cursor(35, 5), None);
        // Vertical fuzzy distance
        assert_eq!(app.node_near_cursor(20, 9), Some(0));
        assert_eq!(app.node_near_cursor(20, 10), None);
    }

    #[test]
    fn test_recompute_layout_world_x_end() {
        let mut app = App::new();
        // Clear main menu nodes
        app.nodes.clear();
        
        let root = 0;
        app.nodes.push(Node {
            label: "Root".to_string(),
            parent: None,
            children: vec![1],
            links: vec![],
            collapsed: true,
            row: usize::MAX,
            world_x: 10,
            world_y: 5,
            world_x_end: 0,
            tags: HashMap::new(),
            times: HashMap::new(),
            is_managed_note: false,
            managed: None,
        });
        app.nodes.push(Node {
            label: "Child".to_string(),
            parent: Some(0),
            children: vec![],
            links: vec![],
            collapsed: false,
            row: usize::MAX,
            world_x: 0,
            world_y: 0,
            world_x_end: 0,
            tags: HashMap::new(),
            times: HashMap::new(),
            is_managed_note: false,
            managed: None,
        });

        app.recompute_layout();

        // Root: world_x=10, depth=0, label="Root"(4), collapsed=true, children=[1]
        // prefix length = 0*2 = 0
        // prefix + "> " = 0 + 2 = 2
        // label_cols = 4
        // suffix_cols = 5 + "1".len() = 6
        // world_x_end = 10 + 2 + 4 + 6 = 22
        assert_eq!(app.nodes[root].world_x_end, 22);
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

pub(crate) fn move_cursor_in_buf(buf: &str, cursor: &mut usize, delta: i32) {
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

