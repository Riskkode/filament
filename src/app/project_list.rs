use super::{App, Mode};
use super::mode::ProjectListState;

impl App {
    // ── Navigation ────────────────────────────────────────────────────────────

    pub fn project_list_nav(&mut self, delta: i32) {
        let count = self.registry.projects.len();
        if count == 0 { return; }
        if let Mode::ProjectList { state: ProjectListState::Browse { ref mut selected } } = self.mode {
            let s = (*selected as i32 + delta).clamp(0, count as i32 - 1);
            *selected = s as usize;
        }
    }

    pub fn project_list_open_selected(&mut self) {
        let idx = match &self.mode {
            Mode::ProjectList { state: ProjectListState::Browse { selected } } => *selected,
            _ => return,
        };
        let path = match self.registry.projects.get(idx) {
            Some(e) => std::path::PathBuf::from(&e.path),
            None    => return,
        };
        self.load_project(&path);
    }

    pub fn project_list_remove_selected(&mut self) {
        let idx = match &self.mode {
            Mode::ProjectList { state: ProjectListState::Browse { selected } } => *selected,
            _ => return,
        };
        self.registry.remove_at(idx);
        let _ = self.registry.save();
        let count = self.registry.projects.len();
        let new_sel = if count == 0 { 0 } else { idx.min(count - 1) };
        self.mode = Mode::ProjectList { state: ProjectListState::Browse { selected: new_sel } };
    }

    // ── New project input ─────────────────────────────────────────────────────

    pub fn project_list_start_new(&mut self) {
        self.mode = Mode::ProjectList { state: ProjectListState::NewPath {
            buf:    String::new(),
            cursor: 0,
        }};
    }

    pub fn project_list_input_char(&mut self, c: char) {
        match &mut self.mode {
            Mode::ProjectList { state: ProjectListState::NewPath { buf, cursor } } => {
                buf.insert(*cursor, c);
                *cursor += c.len_utf8();
            }
            Mode::ProjectList { state: ProjectListState::NewName { buf, cursor, .. } } => {
                buf.insert(*cursor, c);
                *cursor += c.len_utf8();
            }
            _ => {}
        }
    }

    pub fn project_list_backspace(&mut self) {
        match &mut self.mode {
            Mode::ProjectList { state: ProjectListState::NewPath { buf, cursor } } => {
                if *cursor > 0 {
                    let ch = buf[..*cursor].chars().last().unwrap();
                    *cursor -= ch.len_utf8();
                    buf.remove(*cursor);
                }
            }
            Mode::ProjectList { state: ProjectListState::NewName { buf, cursor, .. } } => {
                if *cursor > 0 {
                    let ch = buf[..*cursor].chars().last().unwrap();
                    *cursor -= ch.len_utf8();
                    buf.remove(*cursor);
                }
            }
            _ => {}
        }
    }

    pub fn project_list_move_cursor(&mut self, delta: i32) {
        match &mut self.mode {
            Mode::ProjectList { state: ProjectListState::NewPath { buf, cursor } } => {
                move_cursor_in_buf(buf, cursor, delta);
            }
            Mode::ProjectList { state: ProjectListState::NewName { buf, cursor, .. } } => {
                move_cursor_in_buf(buf, cursor, delta);
            }
            _ => {}
        }
    }

    pub fn project_list_cancel(&mut self) {
        let count = self.registry.projects.len();
        let selected = match &self.mode {
            // Preserve current selection when cancelling back from input states.
            Mode::ProjectList { state: ProjectListState::Browse { selected } } => *selected,
            _ => 0,
        };
        // If there's a project open and the user presses Esc, go back to it.
        if self.project_path.is_some() {
            self.mode = Mode::Canvas { state: super::CanvasState::Browse };
        } else {
            self.mode = Mode::ProjectList {
                state: ProjectListState::Browse {
                    selected: if count == 0 { 0 } else { selected.min(count - 1) },
                },
            };
        }
    }

    /// Confirm the current input step (NewPath → NewName → create project).
    pub fn project_list_confirm(&mut self) {
        match &self.mode {
            Mode::ProjectList { state: ProjectListState::NewPath { buf, .. } } => {
                let raw = buf.trim().to_string();
                if raw.is_empty() {
                    self.project_list_cancel();
                    return;
                }
                // Expand leading `~` to the home directory.
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
                self.mode = Mode::ProjectList { state: ProjectListState::NewName {
                    path:   path_str,
                    buf:    default_name,
                    cursor,
                }};
            }
            Mode::ProjectList { state: ProjectListState::NewName { path, buf, .. } } => {
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
