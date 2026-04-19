use ratatui::style::{Color, Modifier, Style};
use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Palette {
    pub name:           String,
    pub insert:         Color,
    pub edit:           Color,
    pub reparent:       Color,
    pub canvas:         Color,
    pub pick:           Color,
    pub link:           Color,
    pub link_arrow:     Color,
    pub arrow_dim:      Color,
    pub node:           Color,
    pub selected:       Color,
    pub prefix:         Color,
    pub dim:            Color,
    pub status_todo:    Color,
    pub status_progress: Color,
    pub status_done:    Color,
    pub status_blocked: Color,
}

impl Palette {
    pub fn solid(&self, color: Color) -> Style {
        let fg = match color {
            Color::Blue | Color::Magenta | Color::Red | Color::DarkGray => Color::White,
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

pub fn load_all() -> Vec<Palette> {
    vec![
        Palette {
            name:           "dracula".into(),
            insert:         Color::Green,
            edit:           Color::Yellow,
            reparent:       Color::Blue,
            canvas:         Color::Cyan,
            pick:           Color::Magenta,
            link:           Color::LightGreen,
            link_arrow:     Color::Green,
            arrow_dim:      Color::DarkGray,
            node:           Color::Cyan,
            selected:       Color::Yellow,
            prefix:         Color::DarkGray,
            dim:            Color::DarkGray,
            status_todo:    Color::Rgb(142, 142, 142), // Gray
            status_progress:Color::Rgb(230, 142, 13),  // Orange/Amber
            status_done:    Color::Rgb(80,  250, 123), // Green
            status_blocked: Color::Rgb(255, 85,  85),  // Red
            },
            Palette {
            name:           "solarized dark".into(),
            insert:         Color::Rgb(133, 153, 0),   // Green
            edit:           Color::Rgb(181, 137, 0),   // Yellow
            reparent:       Color::Rgb(38, 139, 210),  // Blue
            canvas:         Color::Rgb(42, 161, 152),  // Cyan
            pick:           Color::Rgb(211, 54, 130),  // Magenta
            link:           Color::Rgb(133, 153, 0),   // Green
            link_arrow:     Color::Rgb(133, 153, 0),
            arrow_dim:      Color::Rgb(88, 110, 117),  // Base01
            node:           Color::Rgb(42, 161, 152),
            selected:       Color::Rgb(181, 137, 0),
            prefix:         Color::Rgb(101, 123, 131), // Base00
            dim:            Color::Rgb(88, 110, 117),
            status_todo:    Color::Rgb(142, 142, 142), // Gray
            status_progress:Color::Rgb(230, 142, 13),  // Orange/Amber
            status_done:    Color::Rgb(80,  250, 123), // Green
            status_blocked: Color::Rgb(255, 85,  85),  // Red
            },
            Palette {
            name:           "nord".into(),
            insert:         Color::Rgb(163, 190, 140), // Green
            edit:           Color::Rgb(235, 203, 139), // Yellow
            reparent:       Color::Rgb(129, 161, 193), // Blue
            canvas:         Color::Rgb(136, 192, 208), // Cyan
            pick:           Color::Rgb(180, 142, 173), // Magenta
            link:           Color::Rgb(163, 190, 140),
            link_arrow:     Color::Rgb(163, 190, 140),
            arrow_dim:      Color::Rgb(76, 86, 106),   // Polar Night
            node:           Color::Rgb(136, 192, 208),
            selected:       Color::Rgb(235, 203, 139),
            prefix:         Color::Rgb(76, 86, 106),
            dim:            Color::Rgb(76, 86, 106),
            status_todo:    Color::Rgb(142, 142, 142), // Gray
            status_progress:Color::Rgb(230, 142, 13),  // Orange/Amber
            status_done:    Color::Rgb(80,  250, 123), // Green
            status_blocked: Color::Rgb(255, 85,  85),  // Red
            },
            Palette {
            name:           "monokai".into(),
            insert:         Color::Rgb(166, 226, 46),  // Green
            edit:           Color::Rgb(230, 219, 116), // Yellow
            reparent:       Color::Rgb(102, 217, 239), // Blue
            canvas:         Color::Rgb(174, 129, 255), // Purple/Cyan-ish
            pick:           Color::Rgb(249, 38,  114),  // Magenta
            link:           Color::Rgb(166, 226, 46),
            link_arrow:     Color::Rgb(166, 226, 46),
            arrow_dim:      Color::Rgb(117, 113, 94),  // Gray
            node:           Color::Rgb(102, 217, 239),
            selected:       Color::Rgb(230, 219, 116),
            prefix:         Color::Rgb(117, 113, 94),
            dim:            Color::Rgb(117, 113, 94),
            status_todo:    Color::Rgb(142, 142, 142), // Gray
            status_progress:Color::Rgb(230, 142, 13),  // Orange/Amber
            status_done:    Color::Rgb(80,  250, 123), // Green
            status_blocked: Color::Rgb(255, 85,  85),  // Red
            },
            Palette {
            name:           "github light".into(),
            insert:         Color::Rgb(40, 167, 69),   // Green
            edit:           Color::Rgb(255, 211, 61),  // Yellow
            reparent:       Color::Rgb(3, 102, 214),   // Blue
            canvas:         Color::Rgb(5, 255, 225),   // Cyan
            pick:           Color::Rgb(234, 74, 170),  // Pink
            link:           Color::Rgb(40, 167, 69),
            link_arrow:     Color::Rgb(40, 167, 69),
            arrow_dim:      Color::Rgb(209, 213, 218), // Gray
            node:           Color::Rgb(3, 102, 214),
            selected:       Color::Rgb(255, 211, 61),
            prefix:         Color::Rgb(149, 157, 165),
            dim:            Color::Rgb(209, 213, 218),
            status_todo:    Color::Rgb(142, 142, 142), // Gray
            status_progress:Color::Rgb(230, 142, 13),  // Orange/Amber
            status_done:    Color::Rgb(80,  250, 123), // Green
            status_blocked: Color::Rgb(255, 85,  85),  // Red
            },
            Palette {
            name:           "everforest".into(),
            insert:         Color::Rgb(167, 192, 128), // Green
            edit:           Color::Rgb(219, 188, 127), // Yellow
            reparent:       Color::Rgb(127, 187, 179), // Blue
            canvas:         Color::Rgb(127, 187, 179),
            pick:           Color::Rgb(214, 153, 182), // Purple
            link:           Color::Rgb(167, 192, 128),
            link_arrow:     Color::Rgb(167, 192, 128),
            arrow_dim:      Color::Rgb(133, 146, 137), // Gray
            node:           Color::Rgb(127, 187, 179),
            selected:       Color::Rgb(219, 188, 127),
            prefix:         Color::Rgb(147, 160, 150),
            dim:            Color::Rgb(133, 146, 137),
            status_todo:    Color::Rgb(142, 142, 142), // Gray
            status_progress:Color::Rgb(230, 142, 13),  // Orange/Amber
            status_done:    Color::Rgb(80,  250, 123), // Green
            status_blocked: Color::Rgb(255, 85,  85),  // Red
            },
            Palette {
            name:           "gruvbox".into(),
            insert:         Color::Rgb(184, 187, 38),  // Green
            edit:           Color::Rgb(250, 189, 47),  // Yellow
            reparent:       Color::Rgb(131, 165, 152), // Blue
            canvas:         Color::Rgb(142, 192, 124), // Aqua
            pick:           Color::Rgb(211, 134, 155), // Purple
            link:           Color::Rgb(184, 187, 38),
            link_arrow:     Color::Rgb(184, 187, 38),
            arrow_dim:      Color::Rgb(146, 131, 116), // Gray
            node:           Color::Rgb(142, 192, 124),
            selected:       Color::Rgb(250, 189, 47),
            prefix:         Color::Rgb(168, 153, 132),
            dim:            Color::Rgb(146, 131, 116),
            status_todo:    Color::Rgb(142, 142, 142), // Gray
            status_progress:Color::Rgb(230, 142, 13),  // Orange/Amber
            status_done:    Color::Rgb(80,  250, 123), // Green
            status_blocked: Color::Rgb(255, 85,  85),  // Red
            },
            Palette {
            name:           "catppuccin".into(),
            insert:         Color::Rgb(166, 227, 161), // Green
            edit:           Color::Rgb(249, 226, 175), // Yellow
            reparent:       Color::Rgb(137, 180, 250), // Blue
            canvas:         Color::Rgb(148, 226, 213), // Teal
            pick:           Color::Rgb(203, 166, 247), // Mauve
            link:           Color::Rgb(166, 227, 161),
            link_arrow:     Color::Rgb(166, 227, 161),
            arrow_dim:      Color::Rgb(108, 112, 134), // Gray
            node:           Color::Rgb(148, 226, 213),
            selected:       Color::Rgb(249, 226, 175),
            prefix:         Color::Rgb(166, 173, 200),
            dim:            Color::Rgb(108, 112, 134),
            status_todo:    Color::Rgb(142, 142, 142), // Gray
            status_progress:Color::Rgb(230, 142, 13),  // Orange/Amber
            status_done:    Color::Rgb(80,  250, 123), // Green
            status_blocked: Color::Rgb(255, 85,  85),  // Red
            },
            Palette {
            name:           "rose pine".into(),
            insert:         Color::Rgb(49, 116, 143),  // Pine (using as Green)
            edit:           Color::Rgb(246, 193, 119), // Gold (using as Yellow)
            reparent:       Color::Rgb(156, 207, 216), // Foam (using as Blue)
            canvas:         Color::Rgb(235, 188, 186), // Rose
            pick:           Color::Rgb(196, 167, 231), // Iris
            link:           Color::Rgb(49, 116, 143),
            link_arrow:     Color::Rgb(49, 116, 143),
            arrow_dim:      Color::Rgb(110, 106, 134), // Gray
            node:           Color::Rgb(235, 188, 186),
            selected:       Color::Rgb(246, 193, 119),
            prefix:         Color::Rgb(144, 140, 170),
            dim:            Color::Rgb(110, 106, 134),
            status_todo:    Color::Rgb(142, 142, 142), // Gray
            status_progress:Color::Rgb(230, 142, 13),  // Orange/Amber
            status_done:    Color::Rgb(80,  250, 123), // Green
            status_blocked: Color::Rgb(255, 85,  85),  // Red
            },
            Palette {
            name:           "matte black".into(),
            insert:         Color::Rgb(100, 100, 100),
            edit:           Color::Rgb(180, 180, 180),
            reparent:       Color::Rgb(120, 120, 120),
            canvas:         Color::Rgb(140, 140, 140),
            pick:           Color::Rgb(160, 160, 160),
            link:           Color::Rgb(100, 100, 100),
            link_arrow:     Color::Rgb(100, 100, 100),
            arrow_dim:      Color::Rgb(50, 50, 50),
            node:           Color::Rgb(140, 140, 140),
            selected:       Color::Rgb(255, 255, 255),
            prefix:         Color::Rgb(80, 80, 80),
            dim:            Color::Rgb(60, 60, 60),
            status_todo:    Color::Rgb(142, 142, 142), // Gray
            status_progress:Color::Rgb(230, 142, 13),  // Orange/Amber
            status_done:    Color::Rgb(80,  250, 123), // Green
            status_blocked: Color::Rgb(255, 85,  85),  // Red
            },
    ]
}

pub fn get_palette(name: &str) -> Palette {
    load_all().into_iter()
        .find(|p| p.name == name)
        .unwrap_or_else(|| load_all().remove(0))
}
