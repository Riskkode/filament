use ratatui::style::{Color, Modifier, Style};

// ── Mode colours ──────────────────────────────────────────────────────────────
// Each mode that the user can enter has an associated colour used consistently
// across the border, title, inline prompts and node highlights.

pub const INSERT:   Color = Color::Green;
pub const EDIT:     Color = Color::Yellow;
pub const REPARENT: Color = Color::Blue;
pub const CANVAS:   Color = Color::Cyan;
pub const PICK:     Color = Color::Magenta;

// ── Node / tree colours ───────────────────────────────────────────────────────

pub const NODE:     Color = Color::Cyan;
pub const SELECTED: Color = Color::Yellow;
pub const PREFIX:   Color = Color::DarkGray;
pub const DIM:      Color = Color::DarkGray;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Solid highlight: black text on a mode-colour background.
/// Use for light-bg colours (Yellow, Cyan, Green); the contrast is fine.
/// For dark-bg colours (Blue, Magenta) white fg is readable instead.
pub fn solid(color: Color) -> Style {
    let fg = match color {
        Color::Blue | Color::Magenta | Color::Red | Color::DarkGray => Color::White,
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
