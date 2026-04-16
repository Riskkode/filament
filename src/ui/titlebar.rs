use crate::app::{CanvasState, InputAction, Mode, StartMenuState};
use crate::ui::palette as pal;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Border style for the outer frame — coloured to match the active mode.
pub fn border_style(mode: &Mode) -> Style {
    Style::default().fg(mode_color(mode))
}

fn mode_color(mode: &Mode) -> Color {
    match mode {
        Mode::StartMenu { .. }                                         => pal::CANVAS,
        Mode::Input { action: InputAction::InsertChild { .. }, .. }    => pal::INSERT,
        Mode::Input { .. }                                             => pal::EDIT,
        Mode::Reparent { .. }                                          => pal::REPARENT,
        Mode::Canvas { state: CanvasState::Pick { .. } }              => pal::PICK,
        Mode::Canvas { state: CanvasState::Link { .. } }              => pal::LINK,
        Mode::Canvas { .. }                                            => pal::CANVAS,
        Mode::Help                                                     => pal::PICK,
    }
}

/// Title line rendered inside the top border of the outer frame.
pub fn build_title(mode: &Mode) -> Line<'_> {
    match mode {
        Mode::StartMenu { state: StartMenuState::Main { .. } } => start_menu_title(),
        Mode::StartMenu { state: StartMenuState::NewPath { .. } } => modal_title(
            "NEW PROJECT", pal::INSERT,
            "enter directory path  Enter:confirm  Esc:cancel".to_string(),
        ),
        Mode::StartMenu { state: StartMenuState::NewName { .. } } => modal_title(
            "NEW PROJECT", pal::INSERT,
            "enter project name  Enter:confirm  Esc:cancel".to_string(),
        ),
        Mode::StartMenu { state: StartMenuState::EditSetting { key, .. } } => modal_title(
            "SETTINGS", pal::EDIT,
            format!("edit {}  Enter:confirm  Esc:cancel", key.replace('_', " ")),
        ),
        Mode::Canvas { state: CanvasState::Browse } => canvas_title(),

        Mode::Input { action: InputAction::InsertChild { .. }, .. } => modal_title(
            "INSERT", pal::INSERT,
            "Enter:add  Tab:indent  ⇧Tab:dedent  ←→:cursor  Esc:done".to_string(),
        ),
        Mode::Input { .. } => modal_title(
            "EDIT", pal::EDIT,
            "←→:cursor  Enter:confirm  Esc:cancel".to_string(),
        ),
        Mode::Reparent { .. } => modal_title(
            "REPARENT", pal::REPARENT,
            "hjkl:navigate  v/↵:confirm  Esc:cancel".to_string(),
        ),
        Mode::Canvas { state: CanvasState::New { .. } } => modal_title(
            "NEW NODE", pal::INSERT,
            "type name  Enter:confirm  Esc:cancel".to_string(),
        ),
        Mode::Canvas { state: CanvasState::Pick { .. } } => modal_title(
            "PICK", pal::PICK,
            "hjkl:move  p:place  Esc:cancel".to_string(),
        ),
        Mode::Canvas { state: CanvasState::Link { .. } } => modal_title(
            "LINK", pal::LINK,
            "hjkl:navigate  Enter:toggle link  Esc:cancel".to_string(),
        ),
        Mode::Canvas { state: CanvasState::Goto { .. } } => modal_title(
            "GOTO", pal::EDIT,
            "type to search  Tab:next match  Enter:jump  Esc:cancel".to_string(),
        ),
        Mode::Canvas { state: CanvasState::Menu } => modal_title(
            "ARROWS", pal::CANVAS,
            "i:incoming  o:outgoing  g:global  F/Esc:close".to_string(),
        ),
        Mode::Canvas { state: CanvasState::MenuIncoming } => modal_title(
            "ARROWS › INCOMING", pal::CANVAS,
            "T:tree  S:selected  Esc:back".to_string(),
        ),
        Mode::Canvas { state: CanvasState::MenuOutgoing } => modal_title(
            "ARROWS › OUTGOING", pal::CANVAS,
            "T:tree  S:selected  Esc:back".to_string(),
        ),
        Mode::Help => modal_title(
            "HELP", pal::PICK,
            "hjkl:navigate  Esc:close".to_string(),
        ),
    }
}

// ── Builders ──────────────────────────────────────────────────────────────────

fn start_menu_title() -> Line<'static> {
    Line::from(vec![
        Span::styled(" filament", Style::default().add_modifier(Modifier::BOLD)),
        sep(),
        Span::styled("Welcome ", Style::default().fg(pal::CANVAS)),
    ])
}

/// Canvas (ground state): mode triggers + command keys.
fn canvas_title() -> Line<'static> {
    Line::from(vec![
        Span::styled(" filament", Style::default().add_modifier(Modifier::BOLD)),
        sep(),
        // ── Mode triggers ────────────────────────────────────────────────────
        bracket("i",   pal::INSERT),   Span::styled(" ins  ",     pal::tinted(pal::INSERT)),
        bracket("e/E", pal::EDIT),     Span::styled(" edit  ",    pal::tinted(pal::EDIT)),
        bracket("v",   pal::REPARENT), Span::styled(" move  ",    pal::tinted(pal::REPARENT)),
        bracket("p",   pal::PICK),     Span::styled(" pick  ",    pal::tinted(pal::PICK)),
        bracket("f",   pal::LINK),     Span::styled(" link  ",    pal::tinted(pal::LINK)),
        bracket("g",   pal::EDIT),     Span::styled(" goto  ",    pal::tinted(pal::EDIT)),
        bracket("?",   pal::PICK),     Span::styled(" help",      pal::tinted(pal::PICK)),
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
