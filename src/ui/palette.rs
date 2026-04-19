use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize)]
pub struct Palette {
    pub insert:   Color,
    pub edit:     Color,
    pub reparent: Color,
    pub canvas:   Color,
    pub pick:     Color,
    pub link:     Color,

    pub link_arrow: Color,
    pub arrow_dim:  Color,

    pub node:     Color,
    pub selected: Color,
    pub prefix:   Color,
    pub dim:      Color,

    pub status_todo:     Color,
    pub status_progress: Color,
    pub status_done:     Color,
    pub status_blocked:  Color,
}

impl Palette {
    pub fn dracula() -> Self {
        Self {
            insert:   Color::Rgb(80,  250, 123),
            edit:     Color::Rgb(241, 250, 140),
            reparent: Color::Rgb(139, 233, 253),
            canvas:   Color::Rgb(98,  114, 164),
            pick:     Color::Rgb(255, 121, 198),
            link:     Color::Rgb(189, 147, 249),

            link_arrow: Color::Rgb(80, 250, 123),
            arrow_dim:  Color::Rgb(68, 71, 90),

            node:     Color::Rgb(139, 233, 253),
            selected: Color::Rgb(241, 250, 140),
            prefix:   Color::Rgb(98,  114, 164),
            dim:      Color::Rgb(98,  114, 164),

            status_todo:     Color::Rgb(248, 248, 242),
            status_progress: Color::Rgb(241, 250, 140),
            status_done:     Color::Rgb(80,  250, 123),
            status_blocked:  Color::Rgb(255, 85,  85),
        }
    }

    pub fn themes_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("filament").join("themes"))
    }

    pub fn load_all() -> HashMap<String, Palette> {
        let mut themes = HashMap::new();
        // Provide all defaults as built-ins so they always show up
        themes.insert("dracula".to_string(), Self::dracula());
        themes.insert("solarized_dark".to_string(), Self::solarized_dark());
        themes.insert("nord".to_string(), Self::nord());
        themes.insert("monokai".to_string(), Self::monokai());
        themes.insert("github_light".to_string(), Self::github_light());
        themes.insert("everforest".to_string(), Self::everforest());
        themes.insert("gruvbox".to_string(), Self::gruvbox());
        themes.insert("catppuccin".to_string(), Self::catppuccin());
        themes.insert("rose_pine".to_string(), Self::rose_pine());
        themes.insert("matte_black".to_string(), Self::matte_black());

        let Some(dir) = Self::themes_dir() else { return themes };
        if !dir.exists() {
            let _ = std::fs::create_dir_all(&dir);
            // Export default themes as files for user convenience and Omarchy integration
            let _ = Self::export_defaults(&dir);
        }

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        // This will overwrite built-ins with file versions if they exist,
                        // allowing user customization.
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(palette) = toml::from_str::<Palette>(&content) {
                                themes.insert(name.to_string(), palette);
                            }
                        }
                    }
                }
            }
        }

        themes
    }

    fn export_defaults(dir: &Path) -> std::io::Result<()> {
        let defaults = [
            ("dracula", Self::dracula()),
            ("solarized_dark", Self::solarized_dark()),
            ("nord", Self::nord()),
            ("monokai", Self::monokai()),
            ("github_light", Self::github_light()),
            ("everforest", Self::everforest()),
            ("gruvbox", Self::gruvbox()),
            ("catppuccin", Self::catppuccin()),
            ("rose_pine", Self::rose_pine()),
            ("matte_black", Self::matte_black()),
        ];

        for (name, pal) in defaults {
            let path = dir.join(format!("{}.toml", name));
            if !path.exists() {
                let content = toml::to_string_pretty(&pal).unwrap();
                std::fs::write(path, content)?;
            }
        }
        Ok(())
    }

    pub fn solarized_dark() -> Self {
        Self {
            insert:   Color::Rgb(133, 153, 0),
            edit:     Color::Rgb(181, 137, 0),
            reparent: Color::Rgb(38,  139, 210),
            canvas:   Color::Rgb(101, 123, 131),
            pick:     Color::Rgb(211, 54,  130),
            link:     Color::Rgb(108, 113, 196),
            link_arrow: Color::Rgb(133, 153, 0),
            arrow_dim:  Color::Rgb(7,   54,  66),
            node:     Color::Rgb(38,  139, 210),
            selected: Color::Rgb(181, 137, 0),
            prefix:   Color::Rgb(101, 123, 131),
            dim:      Color::Rgb(88,  110, 117),
            status_todo:     Color::Rgb(131, 148, 150),
            status_progress: Color::Rgb(181, 137, 0),
            status_done:     Color::Rgb(133, 153, 0),
            status_blocked:  Color::Rgb(220, 50,  47),
        }
    }

    pub fn nord() -> Self {
        Self {
            insert:   Color::Rgb(163, 190, 140),
            edit:     Color::Rgb(235, 203, 139),
            reparent: Color::Rgb(136, 192, 208),
            canvas:   Color::Rgb(129, 161, 193),
            pick:     Color::Rgb(180, 142, 173),
            link:     Color::Rgb(143, 188, 187),
            link_arrow: Color::Rgb(163, 190, 140),
            arrow_dim:  Color::Rgb(76,  86,  106),
            node:     Color::Rgb(136, 192, 208),
            selected: Color::Rgb(235, 203, 139),
            prefix:   Color::Rgb(129, 161, 193),
            dim:      Color::Rgb(76,  86,  106),
            status_todo:     Color::Rgb(216, 222, 233),
            status_progress: Color::Rgb(235, 203, 139),
            status_done:     Color::Rgb(163, 190, 140),
            status_blocked:  Color::Rgb(191, 97,  106),
        }
    }

    pub fn monokai() -> Self {
        Self {
            insert:   Color::Rgb(166, 226, 46),
            edit:     Color::Rgb(230, 219, 116),
            reparent: Color::Rgb(102, 217, 239),
            canvas:   Color::Rgb(174, 129, 255),
            pick:     Color::Rgb(249, 38,  114),
            link:     Color::Rgb(253, 151, 31),
            link_arrow: Color::Rgb(166, 226, 46),
            arrow_dim:  Color::Rgb(117, 113, 94),
            node:     Color::Rgb(102, 217, 239),
            selected: Color::Rgb(230, 219, 116),
            prefix:   Color::Rgb(174, 129, 255),
            dim:      Color::Rgb(117, 113, 94),
            status_todo:     Color::Rgb(248, 248, 242),
            status_progress: Color::Rgb(230, 219, 116),
            status_done:     Color::Rgb(166, 226, 46),
            status_blocked:  Color::Rgb(249, 38,  114),
        }
    }

    pub fn github_light() -> Self {
        Self {
            insert:   Color::Rgb(40,  167, 69),
            edit:     Color::Rgb(255, 166, 0),
            reparent: Color::Rgb(3,   102, 214),
            canvas:   Color::Rgb(36,  41,  46),
            pick:     Color::Rgb(234, 74,  170),
            link:     Color::Rgb(111, 66,  193),
            link_arrow: Color::Rgb(40, 167, 69),
            arrow_dim:  Color::Rgb(225, 228, 232),
            node:     Color::Rgb(3,   102, 214),
            selected: Color::Rgb(106, 115, 125),
            prefix:   Color::Rgb(209, 213, 218),
            dim:      Color::Rgb(149, 157, 165),
            status_todo:     Color::Rgb(36,  41,  46),
            status_progress: Color::Rgb(255, 166, 0),
            status_done:     Color::Rgb(40,  167, 69),
            status_blocked:  Color::Rgb(215, 58,  73),
        }
    }

    pub fn everforest() -> Self {
        Self {
            insert:   Color::Rgb(167, 192, 128),
            edit:     Color::Rgb(219, 188, 127),
            reparent: Color::Rgb(127, 187, 179),
            canvas:   Color::Rgb(133, 155, 125),
            pick:     Color::Rgb(214, 123, 145),
            link:     Color::Rgb(211, 134, 155),
            link_arrow: Color::Rgb(167, 192, 128),
            arrow_dim:  Color::Rgb(75,  81,  82),
            node:     Color::Rgb(127, 187, 179),
            selected: Color::Rgb(219, 188, 127),
            prefix:   Color::Rgb(133, 155, 125),
            dim:      Color::Rgb(122, 132, 122),
            status_todo:     Color::Rgb(211, 190, 153),
            status_progress: Color::Rgb(219, 188, 127),
            status_done:     Color::Rgb(167, 192, 128),
            status_blocked:  Color::Rgb(230, 126, 128),
        }
    }

    pub fn gruvbox() -> Self {
        Self {
            insert:   Color::Rgb(184, 187, 38),
            edit:     Color::Rgb(250, 189, 47),
            reparent: Color::Rgb(131, 165, 152),
            canvas:   Color::Rgb(146, 131, 116),
            pick:     Color::Rgb(251, 73,  52),
            link:     Color::Rgb(211, 134, 155),
            link_arrow: Color::Rgb(184, 187, 38),
            arrow_dim:  Color::Rgb(60,  56,  54),
            node:     Color::Rgb(131, 165, 152),
            selected: Color::Rgb(250, 189, 47),
            prefix:   Color::Rgb(146, 131, 116),
            dim:      Color::Rgb(168, 153, 132),
            status_todo:     Color::Rgb(235, 219, 178),
            status_progress: Color::Rgb(250, 189, 47),
            status_done:     Color::Rgb(184, 187, 38),
            status_blocked:  Color::Rgb(251, 73,  52),
        }
    }

    pub fn catppuccin() -> Self {
        Self {
            insert:   Color::Rgb(166, 227, 161),
            edit:     Color::Rgb(249, 226, 175),
            reparent: Color::Rgb(137, 180, 250),
            canvas:   Color::Rgb(148, 156, 187),
            pick:     Color::Rgb(245, 194, 231),
            link:     Color::Rgb(203, 166, 247),
            link_arrow: Color::Rgb(166, 227, 161),
            arrow_dim:  Color::Rgb(88,  91,  112),
            node:     Color::Rgb(137, 180, 250),
            selected: Color::Rgb(249, 226, 175),
            prefix:   Color::Rgb(148, 156, 187),
            dim:      Color::Rgb(108, 112, 134),
            status_todo:     Color::Rgb(205, 214, 244),
            status_progress: Color::Rgb(249, 226, 175),
            status_done:     Color::Rgb(166, 227, 161),
            status_blocked:  Color::Rgb(243, 139, 168),
        }
    }

    pub fn rose_pine() -> Self {
        Self {
            insert:   Color::Rgb(156, 207, 216),
            edit:     Color::Rgb(246, 193, 119),
            reparent: Color::Rgb(49,  116, 143),
            canvas:   Color::Rgb(110, 106, 134),
            pick:     Color::Rgb(235, 111, 145),
            link:     Color::Rgb(196, 167, 231),
            link_arrow: Color::Rgb(156, 207, 216),
            arrow_dim:  Color::Rgb(31,  29,  46),
            node:     Color::Rgb(156, 207, 216),
            selected: Color::Rgb(246, 193, 119),
            prefix:   Color::Rgb(110, 106, 134),
            dim:      Color::Rgb(144, 140, 170),
            status_todo:     Color::Rgb(224, 222, 244),
            status_progress: Color::Rgb(246, 193, 119),
            status_done:     Color::Rgb(156, 207, 216),
            status_blocked:  Color::Rgb(235, 111, 145),
        }
    }

    pub fn matte_black() -> Self {
        Self {
            insert:   Color::Rgb(255, 193, 7),   // Amber (#FFC107) - using for insert
            edit:     Color::Rgb(230, 142, 13),  // Blue/Orange (#e68e0d)
            reparent: Color::Rgb(190, 190, 190), // Cyan/Gray (#bebebe)
            canvas:   Color::Rgb(138, 138, 141), // Bright Black (#8a8a8d)
            pick:     Color::Rgb(211, 95,  95),  // Red/Magenta (#D35F5F)
            link:     Color::Rgb(234, 234, 234), // Bright Cyan (#eaeaea)

            link_arrow: Color::Rgb(255, 193, 7),
            arrow_dim:  Color::Rgb(51,  51,  51),  // Black (#333333)

            node:     Color::Rgb(190, 190, 190), // Foreground (#bebebe)
            selected: Color::Rgb(255, 193, 7),   // Amber accent
            prefix:   Color::Rgb(138, 138, 141),
            dim:      Color::Rgb(51,  51,  51),

            status_todo:     Color::Rgb(190, 190, 190),
            status_progress: Color::Rgb(230, 142, 13),
            status_done:     Color::Rgb(255, 193, 7),
            status_blocked:  Color::Rgb(211, 95,  95),
        }
    }

    pub fn get_palette(name: &str) -> Self {
        let all = Self::load_all();
        all.get(name).cloned().unwrap_or_else(Self::dracula)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

pub fn solid(color: Color) -> Style {
    let fg = match color {
        Color::Rgb(r, g, b) => {
            let lum = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
            if lum > 150.0 { Color::Black } else { Color::White }
        }
        Color::Blue | Color::Magenta | Color::Red | Color::DarkGray | Color::Black => Color::White,
        _ => Color::Black,
    };
    Style::default().fg(fg).bg(color).add_modifier(Modifier::BOLD)
}

pub fn tinted(color: Color) -> Style {
    Style::default().fg(color)
}

pub fn tinted_bold(color: Color) -> Style {
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}
