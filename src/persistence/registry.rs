use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Registry {
    #[serde(default)]
    pub projects: Vec<ProjectEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ProjectEntry {
    pub name: String,
    pub path: String,
}

impl Registry {
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("filament").join("registry.toml"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else { return Self::default() };
        let Ok(content) = std::fs::read_to_string(&path) else { return Self::default() };
        toml::from_str(&content).unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "cannot resolve config dir")
        })?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(&path, content)
    }

    pub fn add_project(&mut self, entry: ProjectEntry) {
        if let Some(e) = self.projects.iter_mut().find(|e| e.path == entry.path) {
            e.name = entry.name;
        } else {
            self.projects.push(entry);
        }
    }

    pub fn remove_at(&mut self, idx: usize) {
        if idx < self.projects.len() {
            self.projects.remove(idx);
        }
    }
}
