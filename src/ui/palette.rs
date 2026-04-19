use ratatui::style::{Color, Modifier, Style};

// ── Mode colours ──────────────────────────────────────────────────────────────
// Each mode that the user can enter has an associated colour used consistently
// across the border, title, inline prompts and node highlights.

pub const INSERT:   Color = Color::Rgb(80,  250, 123); // Green
pub const EDIT:     Color = Color::Rgb(241, 250, 140); // Yellow
pub const REPARENT: Color = Color::Rgb(139, 233, 253); // Cyan
pub const CANVAS:   Color = Color::Rgb(98,  114, 164); // Blue/Slate
pub const PICK:     Color = Color::Rgb(255, 121, 198); // Pink/Magenta
pub const LINK:     Color = Color::Rgb(189, 147, 249); // Purple

/// Dimmer shade used for persistent link arrows in browse mode.
pub const LINK_ARROW: Color = Color::Rgb(80, 250, 123);
/// Very dim colour for global-mode background arrows (all links, unselected).
pub const ARROW_DIM:  Color = Color::Rgb(68, 71, 90);

// ── Node / tree colours ───────────────────────────────────────────────────────

pub const NODE:     Color = Color::Rgb(139, 233, 253);
pub const SELECTED: Color = Color::Rgb(241, 250, 140);
pub const PREFIX:   Color = Color::Rgb(98,  114, 164);
pub const DIM:      Color = Color::Rgb(98,  114, 164);

// ── Status Tag colours ────────────────────────────────────────────────────────

pub const STATUS_TODO:     Color = Color::Rgb(248, 248, 242); // White-ish
pub const STATUS_PROGRESS: Color = Color::Rgb(241, 250, 140); // Yellow
pub const STATUS_DONE:     Color = Color::Rgb(80,  250, 123); // Green
pub const STATUS_BLOCKED:  Color = Color::Rgb(255, 85,  85);  // Red

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Solid highlight: black text on a mode-colour background.
/// Uses a simple luminance check to decide between black and white text.
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

/// Tinted: text in the mode colour, no background.
pub fn tinted(color: Color) -> Style {
    Style::default().fg(color)
}

/// Tinted + bold.
pub fn tinted_bold(color: Color) -> Style {
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}
