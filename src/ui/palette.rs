use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Serialize, Deserialize)]
pub struct Palette {
    pub name:           String,
    pub insert:         Color,
    pub edit:           Color,
    pub reparent:       Color,
    pub canvas:         Color,
    pub pick:           Color,
    pub link:           Color,

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
    pub fn solid(&self, color: Color) -> Style {
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

    pub fn tinted(&self, color: Color) -> Style {
        Style::default().fg(color)
    }

    pub fn tinted_bold(&self, color: Color) -> Style {
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    }
}

pub fn builtins() -> Vec<Palette> {
    vec![
        Palette {
            name:           "dracula".into(),
            insert:         Color::Rgb(80,  250, 123),
            edit:           Color::Rgb(241, 250, 140),
            reparent:       Color::Rgb(139, 233, 253),
            canvas:         Color::Rgb(98,  114, 164),
            pick:           Color::Rgb(255, 121, 198),
            link:           Color::Rgb(189, 147, 249),
            link_arrow:     Color::Rgb(80,  250, 123),
            arrow_dim:      Color::Rgb(68,  71,  90),
            node:           Color::Rgb(139, 233, 253),
            selected:       Color::Rgb(241, 250, 140),
            prefix:         Color::Rgb(98,  114, 164),
            dim:            Color::Rgb(98,  114, 164),
            status_todo:    Color::Rgb(142, 142, 142), // Gray
            status_progress:Color::Rgb(230, 142, 13),  // Orange/Amber
            status_done:    Color::Rgb(80,  250, 123), // Green
            status_blocked: Color::Rgb(255, 85,  85),  // Red
        },
        Palette {
            name:           "solarized dark".into(),
            insert:         Color::Rgb(133, 153, 0),
            edit:           Color::Rgb(181, 137, 0),
            reparent:       Color::Rgb(38,  139, 210),
            canvas:         Color::Rgb(101, 123, 131),
            pick:           Color::Rgb(211, 54,  130),
            link:           Color::Rgb(108, 113, 196),
            link_arrow:     Color::Rgb(133, 153, 0),
            arrow_dim:      Color::Rgb(7,   54,  66),
            node:           Color::Rgb(38,  139, 210),
            selected:       Color::Rgb(181, 137, 0),
            prefix:         Color::Rgb(101, 123, 131),
            dim:            Color::Rgb(88,  110, 117),
            status_todo:    Color::Rgb(142, 142, 142),
            status_progress:Color::Rgb(230, 142, 13),
            status_done:    Color::Rgb(80,  250, 123),
            status_blocked: Color::Rgb(255, 85,  85),
        },
        Palette {
            name:           "nord".into(),
            insert:         Color::Rgb(163, 190, 140),
            edit:           Color::Rgb(235, 203, 139),
            reparent:       Color::Rgb(136, 192, 208),
            canvas:         Color::Rgb(129, 161, 193),
            pick:           Color::Rgb(180, 142, 173),
            link:           Color::Rgb(143, 188, 187),
            link_arrow:     Color::Rgb(163, 190, 140),
            arrow_dim:      Color::Rgb(76,  86,  106),
            node:           Color::Rgb(136, 192, 208),
            selected:       Color::Rgb(235, 203, 139),
            prefix:         Color::Rgb(129, 161, 193),
            dim:            Color::Rgb(76,  86,  106),
            status_todo:    Color::Rgb(142, 142, 142),
            status_progress:Color::Rgb(230, 142, 13),
            status_done:    Color::Rgb(80,  250, 123),
            status_blocked: Color::Rgb(255, 85,  85),
        },
        Palette {
            name:           "monokai".into(),
            insert:         Color::Rgb(166, 226, 46),
            edit:           Color::Rgb(230, 219, 116),
            reparent:       Color::Rgb(102, 217, 239),
            canvas:         Color::Rgb(174, 129, 255),
            pick:           Color::Rgb(249, 38,  114),
            link:           Color::Rgb(253, 151, 31),
            link_arrow:     Color::Rgb(166, 226, 46),
            arrow_dim:      Color::Rgb(117, 113, 94),
            node:           Color::Rgb(102, 217, 239),
            selected:       Color::Rgb(230, 219, 116),
            prefix:         Color::Rgb(174, 129, 255),
            dim:            Color::Rgb(117, 113, 94),
            status_todo:    Color::Rgb(142, 142, 142),
            status_progress:Color::Rgb(230, 142, 13),
            status_done:    Color::Rgb(80,  250, 123),
            status_blocked: Color::Rgb(255, 85,  85),
        },
        Palette {
            name:           "github light".into(),
            insert:         Color::Rgb(40,  167, 69),
            edit:           Color::Rgb(255, 166, 0),
            reparent:       Color::Rgb(3,   102, 214),
            canvas:         Color::Rgb(36,  41,  46),
            pick:           Color::Rgb(234, 74,  170),
            link:           Color::Rgb(111, 66,  193),
            link_arrow:     Color::Rgb(40,  167, 69),
            arrow_dim:      Color::Rgb(225, 228, 232),
            node:           Color::Rgb(3,   102, 214),
            selected:       Color::Rgb(106, 115, 125),
            prefix:         Color::Rgb(209, 213, 218),
            dim:            Color::Rgb(149, 157, 165),
            status_todo:    Color::Rgb(142, 142, 142),
            status_progress:Color::Rgb(230, 142, 13),
            status_done:    Color::Rgb(80,  250, 123),
            status_blocked: Color::Rgb(255, 85,  85),
        },
        Palette {
            name:           "everforest".into(),
            insert:         Color::Rgb(167, 192, 128),
            edit:           Color::Rgb(219, 188, 127),
            reparent:       Color::Rgb(127, 187, 179),
            canvas:         Color::Rgb(133, 155, 125),
            pick:           Color::Rgb(214, 123, 145),
            link:           Color::Rgb(211, 134, 155),
            link_arrow:     Color::Rgb(167, 192, 128),
            arrow_dim:      Color::Rgb(75,  81,  82),
            node:           Color::Rgb(127, 187, 179),
            selected:       Color::Rgb(219, 188, 127),
            prefix:         Color::Rgb(133, 155, 125),
            dim:            Color::Rgb(122, 132, 122),
            status_todo:    Color::Rgb(142, 142, 142),
            status_progress:Color::Rgb(230, 142, 13),
            status_done:    Color::Rgb(80,  250, 123),
            status_blocked: Color::Rgb(255, 85,  85),
        },
        Palette {
            name:           "gruvbox".into(),
            insert:         Color::Rgb(184, 187, 38),
            edit:           Color::Rgb(250, 189, 47),
            reparent:       Color::Rgb(131, 165, 152),
            canvas:         Color::Rgb(146, 131, 116),
            pick:           Color::Rgb(251, 73,  52),
            link:           Color::Rgb(211, 134, 155),
            link_arrow:     Color::Rgb(184, 187, 38),
            arrow_dim:      Color::Rgb(60,  56,  54),
            node:           Color::Rgb(131, 165, 152),
            selected:       Color::Rgb(250, 189, 47),
            prefix:         Color::Rgb(146, 131, 116),
            dim:            Color::Rgb(168, 153, 132),
            status_todo:    Color::Rgb(142, 142, 142),
            status_progress:Color::Rgb(230, 142, 13),
            status_done:    Color::Rgb(80,  250, 123),
            status_blocked: Color::Rgb(255, 85,  85),
        },
        Palette {
            name:           "catppuccin".into(),
            insert:         Color::Rgb(166, 227, 161),
            edit:           Color::Rgb(249, 226, 175),
            reparent:       Color::Rgb(137, 180, 250),
            canvas:         Color::Rgb(148, 156, 187),
            pick:           Color::Rgb(245, 194, 231),
            link:           Color::Rgb(203, 166, 247),
            link_arrow:     Color::Rgb(166, 227, 161),
            arrow_dim:      Color::Rgb(88,  91,  112),
            node:           Color::Rgb(137, 180, 250),
            selected:       Color::Rgb(249, 226, 175),
            prefix:         Color::Rgb(148, 156, 187),
            dim:            Color::Rgb(108, 112, 134),
            status_todo:    Color::Rgb(142, 142, 142),
            status_progress:Color::Rgb(230, 142, 13),
            status_done:    Color::Rgb(80,  250, 123),
            status_blocked: Color::Rgb(255, 85,  85),
        },
        Palette {
            name:           "rose pine".into(),
            insert:         Color::Rgb(156, 207, 216),
            edit:           Color::Rgb(246, 193, 119),
            reparent:       Color::Rgb(49,  116, 143),
            canvas:         Color::Rgb(110, 106, 134),
            pick:           Color::Rgb(235, 111, 145),
            link:           Color::Rgb(196, 167, 231),
            link_arrow:     Color::Rgb(156, 207, 216),
            arrow_dim:      Color::Rgb(31,  29,  46),
            node:           Color::Rgb(156, 207, 216),
            selected:       Color::Rgb(246, 193, 119),
            prefix:         Color::Rgb(110, 106, 134),
            dim:            Color::Rgb(144, 140, 170),
            status_todo:    Color::Rgb(142, 142, 142),
            status_progress:Color::Rgb(230, 142, 13),
            status_done:    Color::Rgb(80,  250, 123),
            status_blocked: Color::Rgb(255, 85,  85),
        },
        Palette {
            name:           "matte black".into(),
            insert:         Color::Rgb(255, 193, 7),   // Amber (#FFC107)
            edit:           Color::Rgb(230, 142, 13),  // Orange (#e68e0d)
            reparent:       Color::Rgb(190, 190, 190), // Foreground (#bebebe)
            canvas:         Color::Rgb(138, 138, 141), // Bright Black (#8a8a8d)
            pick:           Color::Rgb(211, 95,  95),  // Red (#D35F5F)
            link:           Color::Rgb(234, 234, 234), // Cursor/Cyan (#eaeaea)
            link_arrow:     Color::Rgb(255, 193, 7),
            arrow_dim:      Color::Rgb(51,  51,  51),  // Normal Black (#333333)
            node:           Color::Rgb(190, 190, 190), // Foreground (#bebebe)
            selected:       Color::Rgb(255, 193, 7),   // Amber accent
            prefix:         Color::Rgb(138, 138, 141),
            dim:            Color::Rgb(51,  51,  51),
            status_todo:    Color::Rgb(142, 142, 142), // Gray
            status_progress:Color::Rgb(230, 142, 13),  // Amber
            status_done:    Color::Rgb(80,  250, 123), // Green
            status_blocked: Color::Rgb(255, 85,  85),  // Red
        },
    ]
}

pub fn themes_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("filament").join("themes"))
}

pub fn load_all() -> Vec<Palette> {
    let mut themes = builtins();
    
    let Some(dir) = themes_dir() else { return themes };
    if !dir.exists() {
        let _ = std::fs::create_dir_all(&dir);
        // Export default themes as files
        for theme in &themes {
            let path = dir.join(format!("{}.toml", theme.name.replace(' ', "_")));
            if let Ok(content) = toml::to_string_pretty(theme) {
                let _ = std::fs::write(path, content);
            }
        }
    }

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(mut palette) = toml::from_str::<Palette>(&content) {
                        if palette.name.is_empty() {
                            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                palette.name = stem.to_string();
                            }
                        }
                        // Replace builtin if name matches
                        if let Some(existing) = themes.iter_mut().find(|p| p.name == palette.name) {
                            *existing = palette;
                        } else {
                            themes.push(palette);
                        }
                    }
                }
            }
        }
    }

    themes
}

pub fn get_palette(name: &str) -> Palette {
    load_all().into_iter()
        .find(|p| p.name == name)
        .unwrap_or_else(|| builtins().remove(0))
}
