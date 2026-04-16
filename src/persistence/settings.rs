use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct GlobalSettings {
    pub default_projects_path: String,
    pub username: String,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            default_projects_path: "~/Documents/filaments".to_string(),
            username: "user".to_string(),
        }
    }
}

impl GlobalSettings {
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("filament").join("settings.toml"))
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
}
