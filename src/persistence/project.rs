use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn project_dir(base: &Path) -> PathBuf {
    base.join(".filament")
}

pub fn settings_path(base: &Path) -> PathBuf {
    project_dir(base).join("project.toml")
}

pub fn db_path(base: &Path) -> PathBuf {
    project_dir(base).join("canvas.db")
}

// ── Serialisable project settings (stored in project.toml) ───────────────────

#[derive(Serialize, Deserialize)]
pub struct ProjectSettings {
    pub name: String,
    pub created_at: u64,
    #[serde(default)]
    pub view: ViewState,
    #[serde(default)]
    pub arrows: SavedArrows,
}

#[derive(Serialize, Deserialize, Default, Clone, Copy)]
pub struct ViewState {
    pub camera_x: i32,
    pub camera_y: i32,
    pub cursor_x: i32,
    pub cursor_y: i32,
    /// DB row-id of the selected node, 0 = no selection.
    pub selected_db_id: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavedArrows {
    pub global: bool,
    /// "Tree" or "Selected"
    pub incoming: String,
    /// "Tree" or "Selected"
    pub outgoing: String,
}

impl Default for SavedArrows {
    fn default() -> Self {
        Self { global: false, incoming: "Tree".into(), outgoing: "Tree".into() }
    }
}

impl ProjectSettings {
    pub fn new(name: &str) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            name: name.to_string(),
            created_at: ts,
            view: ViewState::default(),
            arrows: SavedArrows::default(),
        }
    }

    pub fn load(base: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(settings_path(base))?;
        toml::from_str(&content).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    pub fn save(&self, base: &Path) -> std::io::Result<()> {
        let dir = project_dir(base);
        std::fs::create_dir_all(&dir)?;
        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(settings_path(base), content)
    }
}
