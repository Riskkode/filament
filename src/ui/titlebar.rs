use crate::app::{CanvasState, InputAction, Mode, StartMenuState};
use crate::ui::palette::{self as pal, Palette};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Border style for the outer frame — coloured to match the active mode.
pub fn border_style(mode: &Mode, palette: &Palette) -> Style {
    Style::default().fg(mode_color(mode, palette))
}

fn mode_color(mode: &Mode, palette: &Palette) -> Color {
    match mode {
        Mode::StartMenu { .. }                                         => palette.canvas,
        Mode::Input { action: InputAction::InsertChild { .. }, .. }    => palette.insert,
        Mode::Input { .. }                                             => palette.edit,
        Mode::Reparent { .. }                                          => palette.reparent,
        Mode::Canvas { state: CanvasState::Pick { .. } }              => palette.pick,
        Mode::Canvas { state: CanvasState::Link { .. } }              => palette.link,
        Mode::Canvas { .. }                                            => palette.canvas,
        Mode::Help                                                     => palette.pick,
    }
}

/// Title line rendered inside the top border of the outer frame.
pub fn build_title<'a>(mode: &'a Mode, palette: &'a Palette) -> Line<'a> {
    match mode {
        Mode::StartMenu { state: StartMenuState::Main { .. } } => start_menu_title(palette),
        Mode::StartMenu { state: StartMenuState::NewPath { .. } } => modal_title(
            "NEW PROJECT", palette.insert,
            "enter directory path  Enter:confirm  Esc:cancel".to_string(),
        ),
        Mode::StartMenu { state: StartMenuState::NewName { .. } } => modal_title(
            "NEW PROJECT", palette.insert,
            "enter project name  Enter:confirm  Esc:cancel".to_string(),
        ),
        Mode::StartMenu { state: StartMenuState::EditSetting { key, .. } } => modal_title(
            "SETTINGS", palette.edit,
            format!("edit {}  Enter:confirm  Esc:cancel", key.replace('_', " ")),
        ),
        Mode::StartMenu { state: StartMenuState::Import { .. } } => modal_title(
            "IMPORT", palette.insert,
            "type to fuzzy search  ↑↓:nav  Tab:change location  Enter:import  Esc:cancel".to_string(),
        ),
        Mode::Canvas { state: CanvasState::Browse } => canvas_title(palette),

        Mode::Input { action: InputAction::InsertChild { .. }, .. } => modal_title(
            "INSERT", palette.insert,
            "Enter:add  Tab:indent  ⇧Tab:dedent  ←→:cursor  Esc:done".to_string(),
        ),
        Mode::Input { .. } => modal_title(
            "EDIT", palette.edit,
            "←→:cursor  Enter:confirm  Esc:cancel".to_string(),
        ),
        Mode::Reparent { .. } => modal_title(
            "REPARENT", palette.reparent,
            "hjkl:navigate  v/↵:confirm  Esc:cancel".to_string(),
        ),
        Mode::Canvas { state: CanvasState::New { .. } } => modal_title(
            "NEW NODE", palette.insert,
            "type name  Enter:confirm  Esc:cancel".to_string(),
        ),
        Mode::Canvas { state: CanvasState::Pick { .. } } => modal_title(
            "PICK", palette.pick,
            "hjkl:move  p:place  Esc:cancel".to_string(),
        ),
        Mode::Canvas { state: CanvasState::Link { .. } } => modal_title(
            "LINK", palette.link,
            "hjkl:navigate  Enter:toggle link  Esc:cancel".to_string(),
        ),
        Mode::Canvas { state: CanvasState::Goto { .. } } => modal_title(
            "GOTO", palette.edit,
            "type to search  Tab:next match  Enter:jump  Esc:cancel".to_string(),
        ),
        Mode::Canvas { state: CanvasState::Menu } => modal_title(
            "ARROWS", palette.canvas,
            "i:incoming  o:outgoing  g:global  F/Esc:close".to_string(),
        ),
        Mode::Canvas { state: CanvasState::MenuIncoming } => modal_title(
            "ARROWS › INCOMING", palette.canvas,
            "T:tree  S:selected  Esc:back".to_string(),
        ),
        Mode::Canvas { state: CanvasState::MenuOutgoing } => modal_title(
            "ARROWS › OUTGOING", palette.canvas,
            "T:tree  S:selected  Esc:back".to_string(),
        ),
        Mode::Help => modal_title(
            "HELP", palette.pick,
            "hjkl:navigate  Esc:close".to_string(),
        ),
    }
}

// ── Builders ──────────────────────────────────────────────────────────────────

fn start_menu_title<'a>(palette: &'a Palette) -> Line<'a> {
    Line::from(vec![
        Span::styled(" filament", Style::default().add_modifier(Modifier::BOLD)),
        sep(),
        Span::styled("Welcome ", Style::default().fg(palette.canvas)),
    ])
}

/// Canvas (ground state): mode triggers + command keys.
fn canvas_title<'a>(palette: &'a Palette) -> Line<'a> {
    Line::from(vec![
        Span::styled(" filament", Style::default().add_modifier(Modifier::BOLD)),
        sep(),
        // ── Mode triggers ────────────────────────────────────────────────────
        bracket("i",   palette.insert),   Span::styled(" ins  ",     pal::tinted(palette.insert)),
        bracket("e/E", palette.edit),     Span::styled(" edit  ",    pal::tinted(palette.edit)),
        bracket("v",   palette.reparent), Span::styled(" move  ",    pal::tinted(palette.reparent)),
        bracket("p",   palette.pick),     Span::styled(" pick  ",    pal::tinted(palette.pick)),
        bracket("f",   palette.link),     Span::styled(" link  ",    pal::tinted(palette.link)),
        bracket("g",   palette.edit),     Span::styled(" goto  ",    pal::tinted(palette.edit)),
        bracket("?",   palette.pick),     Span::styled(" help",      pal::tinted(palette.pick)),
    ])
}

/// Modal mode: coloured mode name + muted hint string.
fn modal_title(name: &'static str, color: Color, hints: String) -> Line<'static> {
    Line::from(vec![
        Span::raw(" "),
        Span::styled(name, pal::tinted_bold(color)),
        sep(),
        Span::styled(hints, Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
    ])
}

/// Coloured `[key]` indicator.
fn bracket(key: &'static str, color: Color) -> Span<'static> {
    Span::styled(
        format!("[{key}]"),
        pal::tinted_bold(color),
    )
}

fn sep() -> Span<'static> {
    Span::styled("  │  ", Style::default().fg(Color::DarkGray))
}
