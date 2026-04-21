use crate::app::{CanvasState, InputAction, Mode, StartMenuState, StatusPageState, ArchiveState};
use crate::ui::palette::Palette;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Border style for the outer frame — coloured to match the active mode.
pub fn border_style(pal: &Palette, mode: &Mode) -> Style {
    Style::default().fg(mode_color(pal, mode))
}

pub fn mode_color(pal: &Palette, mode: &Mode) -> Color {
    match mode {
        Mode::StartMenu { .. }                                         => pal.canvas,
        Mode::Input { action: InputAction::InsertChild { .. }, .. }    => pal.insert,
        Mode::Input { .. }                                             => pal.edit,
        Mode::Reparent { .. }                                          => pal.reparent,
        Mode::Canvas { state: CanvasState::Pick { .. } }              => pal.pick,
        Mode::Canvas { state: CanvasState::Link { .. } }              => pal.link,
        Mode::TagStatus { .. }                => pal.edit,
        Mode::TagTime { .. }                  => pal.edit,
        Mode::TagTimeClear { .. }             => pal.edit,
        Mode::TimeInput { .. }                => pal.edit,
        Mode::NoteInput { .. }                => pal.insert,
        Mode::Canvas { .. }                                            => pal.canvas,
        Mode::ContextSwitcher { .. }                                   => pal.edit,
        Mode::StatusPage { .. }                                        => pal.canvas,
        Mode::ArchivePage { .. }                                       => pal.canvas,
        Mode::Help                                                     => pal.pick,
    }
}

/// Title line rendered inside the top border of the outer frame.
pub fn build_title<'a>(pal: &'a Palette, mode: &'a Mode) -> Line<'a> {
    match mode {
        Mode::StartMenu { state: StartMenuState::Main { .. } } => start_menu_title(pal),
        Mode::StartMenu { state: StartMenuState::NewPath { .. } } => modal_title(
            pal, "NEW PROJECT", pal.insert,
            "enter directory path  Enter:confirm  Esc:cancel".to_string(),
        ),
        Mode::StartMenu { state: StartMenuState::NewName { .. } } => modal_title(
            pal, "NEW PROJECT", pal.insert,
            "enter project name  Enter:confirm  Esc:cancel".to_string(),
        ),
        Mode::StartMenu { state: StartMenuState::EditSetting { key, .. } } => modal_title(
            pal, "SETTINGS", pal.edit,
            format!("edit {}  Enter:confirm  Esc:cancel", key.replace('_', " ")),
        ),
        Mode::StartMenu { state: StartMenuState::Import { .. } } => modal_title(
            pal, "IMPORT", pal.insert,
            "type to fuzzy search  ↑↓:nav  Tab:change location  Enter:import  Esc:cancel".to_string(),
        ),
        Mode::StatusPage { state: sps } => {
            match sps {
                StatusPageState::NewQueryName { .. } => modal_title(
                    pal, "NEW GROUP", pal.insert,
                    "enter group name  Enter:next  Esc:cancel".to_string(),
                ),
                StatusPageState::NewQueryLogic { .. } => modal_title(
                    pal, "QUERY LOGIC", pal.insert,
                    "enter logic (e.g. status:todo)  Enter:save  Esc:cancel".to_string(),
                ),
                _ => status_page_title(pal),
            }
        }
        Mode::ArchivePage { state, .. } => {
            match state {
                ArchiveState::EditTitle { .. } => modal_title(
                    pal, "EDIT TITLE", pal.edit,
                    "enter title  Enter:save  Esc:cancel".to_string(),
                ),
                ArchiveState::EditContent { .. } => modal_title(
                    pal, "EDIT CONTENT", pal.edit,
                    "enter content  Enter:newline  Esc:done".to_string(),
                ),
                _ => archive_page_title(pal),
            }
        }
        Mode::TagStatus { .. } => modal_title(
            pal, "STATUS", pal.edit,
            "t:todo  p:in progress  c:done  b:blocked  x:clear  Esc:cancel".to_string(),
        ),
        Mode::TagTime { .. } => modal_title(
            pal, "TIME", pal.edit,
            "d:deadline  s:start  e:end  c:checkpoint  u:duration  r:recurring  x:clear...  Esc:cancel".to_string(),
        ),
        Mode::TagTimeClear { .. } => modal_title(
            pal, "TIME › CLEAR", pal.edit,
            "d:deadline  s:start  e:end  c:checkpoint  u:duration  r:recurring  a/x:all  Esc:back".to_string(),
        ),
        Mode::TimeInput { time_type, .. } => modal_title(
            pal, "TIME INPUT", pal.edit,
            format!("enter {}: tomorrow, friday, 2024-12-01  Enter:confirm  Esc:cancel", time_type),
        ),
        Mode::NoteInput { .. } => modal_title(
            pal, "NOTE", pal.insert,
            "enter note title  Enter:confirm  Esc:cancel".to_string(),
        ),
        Mode::Input { action: InputAction::InsertChild { .. }, .. } => modal_title(
            pal, "INSERT", pal.insert,
            "Enter:add  Tab:indent  ⇧Tab:dedent  ←→:cursor  Esc:done".to_string(),
        ),
        Mode::Input { .. } => modal_title(
            pal, "EDIT", pal.edit,
            "←→:cursor  Enter:confirm  Esc:cancel".to_string(),
        ),
        Mode::Reparent { .. } => modal_title(
            pal, "REPARENT", pal.reparent,
            "hjkl:navigate  v/↵:confirm  Esc:cancel".to_string(),
        ),
        Mode::Canvas { state: CanvasState::Browse } => canvas_title(pal),
        Mode::Canvas { state: CanvasState::New { .. } } => modal_title(
            pal, "NEW NODE", pal.insert,
            "type name  Enter:confirm  Esc:cancel".to_string(),
        ),
        Mode::Canvas { state: CanvasState::Pick { .. } } => modal_title(
            pal, "PICK", pal.pick,
            "hjkl:move  p:place  Esc:cancel".to_string(),
        ),
        Mode::Canvas { state: CanvasState::Link { .. } } => modal_title(
            pal, "LINK", pal.link,
            "hjkl:navigate  Enter:toggle link  Esc:cancel".to_string(),
        ),
        Mode::Canvas { state: CanvasState::Goto { .. } } => modal_title(
            pal, "GOTO", pal.edit,
            "type to search  Tab:next match  Enter:jump  Esc:cancel".to_string(),
        ),
        Mode::Canvas { state: CanvasState::Menu } => modal_title(
            pal, "ARROWS", pal.canvas,
            "i:incoming  o:outgoing  g:global  F/Esc:close".to_string(),
        ),
        Mode::Canvas { state: CanvasState::MenuIncoming } => modal_title(
            pal, "ARROWS › INCOMING", pal.canvas,
            "T:tree  S:selected  Esc:back".to_string(),
        ),
        Mode::Canvas { state: CanvasState::MenuOutgoing } => modal_title(
            pal, "ARROWS › OUTGOING", pal.canvas,
            "T:tree  S:selected  Esc:back".to_string(),
        ),
        Mode::Help => modal_title(
            pal, "HELP", pal.pick,
            "hjkl:navigate  Esc:close".to_string(),
        ),
        Mode::ContextSwitcher { .. } => modal_title(
            pal, "CONTEXT", pal.edit,
            "hjkl:nav  Enter:confirm  shortcuts: f s a  Space:cancel".to_string(),
        ),
    }
}

// ── Builders ──────────────────────────────────────────────────────────────────

fn start_menu_title(pal: &Palette) -> Line<'static> {
    Line::from(vec![
        Span::styled(" filament", Style::default().add_modifier(Modifier::BOLD)),
        sep(),
        Span::styled("Welcome ", Style::default().fg(pal.canvas)),
    ])
}

fn status_page_title(pal: &Palette) -> Line<'static> {
    Line::from(vec![
        Span::styled(" filament", Style::default().add_modifier(Modifier::BOLD)),
        sep(),
        Span::styled("Dashboard ", Style::default().fg(pal.canvas)),
        sep(),
        bracket(pal, "n", pal.insert), Span::styled(" new group  ", pal.tinted(pal.insert)),
        bracket(pal, "x", pal.pick),   Span::styled(" delete node  ", pal.tinted(pal.pick)),
        bracket(pal, "X", pal.pick),   Span::styled(" delete group  ", pal.tinted(pal.pick)),
        bracket(pal, "s", pal.edit),   Span::styled(" status  ", pal.tinted(pal.edit)),
        bracket(pal, "t", pal.edit),   Span::styled(" time  ", pal.tinted(pal.edit)),
    ])
}

fn archive_page_title(pal: &Palette) -> Line<'static> {
    Line::from(vec![
        Span::styled(" filament", Style::default().add_modifier(Modifier::BOLD)),
        sep(),
        Span::styled("Archive ", Style::default().fg(pal.canvas)),
        sep(),
        bracket(pal, "n", pal.insert), Span::styled(" new note  ",  pal.tinted(pal.insert)),
        bracket(pal, "Enter", pal.edit), Span::styled(" edit  ",     pal.tinted(pal.edit)),
        bracket(pal, "x", pal.pick),   Span::styled(" delete note  ", pal.tinted(pal.pick)),
        bracket(pal, "j/k", pal.canvas), Span::styled(" navigate",   pal.tinted(pal.canvas)),
    ])
}

/// Canvas (ground state): mode triggers + command keys.
fn canvas_title(pal: &Palette) -> Line<'static> {
    Line::from(vec![
        Span::styled(" filament", Style::default().add_modifier(Modifier::BOLD)),
        sep(),
        // ── Mode triggers ────────────────────────────────────────────────────
        bracket(pal, "i",   pal.insert),   Span::styled(" ins  ",     pal.tinted(pal.insert)),
        bracket(pal, "e/E", pal.edit),     Span::styled(" edit  ",    pal.tinted(pal.edit)),
        bracket(pal, "v",   pal.reparent), Span::styled(" move  ",    pal.tinted(pal.reparent)),
        bracket(pal, "p",   pal.pick),     Span::styled(" pick  ",    pal.tinted(pal.pick)),
        bracket(pal, "f",   pal.link),     Span::styled(" link  ",    pal.tinted(pal.link)),
        bracket(pal, "s",   pal.edit),     Span::styled(" status  ",  pal.tinted(pal.edit)),
        bracket(pal, "t",   pal.edit),     Span::styled(" time  ",    pal.tinted(pal.edit)),
        bracket(pal, "n",   pal.insert),   Span::styled(" note  ",    pal.tinted(pal.insert)),
        bracket(pal, "g",   pal.edit),     Span::styled(" goto  ",    pal.tinted(pal.edit)),
        bracket(pal, "?",   pal.pick),     Span::styled(" help",      pal.tinted(pal.pick)),
    ])
}

/// Modal mode: coloured mode name + muted hint string.
fn modal_title(pal: &Palette, name: &'static str, color: Color, hints: String) -> Line<'static> {
    Line::from(vec![
        Span::raw(" "),
        Span::styled(name, pal.tinted_bold(color)),
        sep(),
        Span::styled(hints, Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
    ])
}

/// Coloured `[key]` indicator.
fn bracket(pal: &Palette, key: &'static str, color: Color) -> Span<'static> {
    Span::styled(
        format!("[{key}]"),
        pal.tinted_bold(color),
    )
}

fn sep() -> Span<'static> {
    Span::styled("  │  ", Style::default().fg(Color::DarkGray))
}
