use crate::app::{CanvasState, InputAction, Mode};
use crate::ui::palette as pal;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Border style for the outer frame — coloured to match the active mode.
pub fn border_style(mode: &Mode) -> Style {
    Style::default().fg(mode_color(mode))
}

fn mode_color(mode: &Mode) -> Color {
    match mode {
        Mode::Input { action: InputAction::InsertChild { .. }, .. }    => pal::INSERT,
        Mode::Input { .. }                                             => pal::EDIT,
        Mode::Reparent { .. }                                          => pal::REPARENT,
        Mode::Canvas { state: CanvasState::Pick { .. }, .. }          => pal::PICK,
        Mode::Canvas { .. }                                            => pal::CANVAS,
    }
}

/// Title line rendered inside the top border of the outer frame.
pub fn build_title(mode: &Mode) -> Line<'static> {
    match mode {
        Mode::Canvas { state: CanvasState::Browse } => canvas_title(),

        Mode::Input { action: InputAction::InsertChild { .. }, .. } => modal_title(
            "INSERT", pal::INSERT,
            "Enter:add  Tab:indent  ⇧Tab:dedent  ←→:cursor  Esc:done",
        ),
        Mode::Input { .. } => modal_title(
            "EDIT", pal::EDIT,
            "←→:cursor  Enter:confirm  Esc:cancel",
        ),
        Mode::Reparent { .. } => modal_title(
            "REPARENT", pal::REPARENT,
            "hjkl:navigate  v/↵:confirm  Esc:cancel",
        ),
        Mode::Canvas { state: CanvasState::New { .. } } => modal_title(
            "NEW NODE", pal::INSERT,
            "type name  Enter:confirm  Esc:cancel",
        ),
        Mode::Canvas { state: CanvasState::Pick { .. } } => modal_title(
            "PICK", pal::PICK,
            "hjkl:move  p:place  Esc:cancel",
        ),
    }
}

// ── Builders ──────────────────────────────────────────────────────────────────

/// Canvas (ground state): mode triggers + command keys.
fn canvas_title() -> Line<'static> {
    Line::from(vec![
        Span::styled(" filament", Style::default().add_modifier(Modifier::BOLD)),
        sep(),
        // ── Mode triggers ────────────────────────────────────────────────────
        bracket("i",   pal::INSERT),   Span::styled(" insert  ",  pal::tinted(pal::INSERT)),
        bracket("e/E", pal::EDIT),     Span::styled(" edit  ",    pal::tinted(pal::EDIT)),
        bracket("v",   pal::REPARENT), Span::styled(" reparent  ",pal::tinted(pal::REPARENT)),
        bracket("n",   pal::INSERT),   Span::styled(" new  ",     pal::tinted(pal::INSERT)),
        bracket("p",   pal::PICK),     Span::styled(" pick",      pal::tinted(pal::PICK)),
        sep(),
        // ── Immediate commands ────────────────────────────────────────────────
        Span::styled("[x] delete  [d/D] depth  [z] collapse  [c] center  [HJKL] warp  [q] quit ",
            Style::default().fg(Color::DarkGray)),
    ])
}

/// Modal mode: coloured mode name + muted hint string.
fn modal_title(name: &'static str, color: Color, hints: &'static str) -> Line<'static> {
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
