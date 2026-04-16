mod app;
mod db;
mod models;
mod persistence;
mod repositories;
mod ui;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use app::{App, ArrowFidelity, CanvasState, InputAction, Mode, StartMenuState};
use ui::draw::draw;

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new();

    loop {
        draw(&mut terminal, &mut app)?;

        if let Event::Key(key) = event::read()? {
            let (tw, th) = crossterm::terminal::size().unwrap_or((80, 24));
            let canvas_w = tw.saturating_sub(2);
            let canvas_h = th.saturating_sub(3) as usize;

            match app.mode {
                // ── Start menu ────────────────────────────────────────────────
                Mode::StartMenu { .. } => {
                    let is_browse = matches!(&app.mode,
                        Mode::StartMenu { state: StartMenuState::Main { .. } }
                    );

                    if is_browse {
                        match (key.modifiers, key.code) {
                            (_, KeyCode::Char('q')) | (KeyModifiers::SHIFT, KeyCode::Char('Q')) => break,
                            (_, KeyCode::Enter) | (KeyModifiers::NONE, KeyCode::Char('l')) => app.start_menu_confirm(),
                            (KeyModifiers::NONE, KeyCode::Char('e')) | (KeyModifiers::SHIFT, KeyCode::Char('E')) => app.start_menu_edit(),
                            (KeyModifiers::NONE, KeyCode::Char('n'))                       => app.start_menu_go_to_label("new"),
                            (KeyModifiers::NONE, KeyCode::Char('o'))                       => app.start_menu_go_to_label("open"),
                            (KeyModifiers::NONE, KeyCode::Char('f'))                       => app.start_menu_go_to_label("find"),
                            (KeyModifiers::NONE, KeyCode::Char('s'))                       => app.start_menu_go_to_label("settings"),
                            (KeyModifiers::NONE, KeyCode::Char('?'))                       => app.start_menu_go_to_label("help"),
                            (KeyModifiers::NONE, KeyCode::Char('d'))                       => app.start_menu_remove_selected(),
                            (KeyModifiers::NONE, KeyCode::Char('j')) | (_, KeyCode::Down)  => app.start_menu_nav(1),
                            (KeyModifiers::NONE, KeyCode::Char('k')) | (_, KeyCode::Up)    => app.start_menu_nav(-1),
                            (KeyModifiers::NONE, KeyCode::Char('h')) | (_, KeyCode::Esc)   => app.start_menu_cancel(),
                            _ => {}
                        }
                    } else {
                        // NewPath / NewName input.
                        match key.code {
                            KeyCode::Enter     => app.start_menu_confirm(),
                            KeyCode::Esc       => app.start_menu_cancel(),
                            KeyCode::Backspace => app.start_menu_backspace(),
                            KeyCode::Left      => app.start_menu_move_cursor(-1),
                            KeyCode::Right     => app.start_menu_move_cursor(1),
                            KeyCode::Char(c)   => app.start_menu_input_char(c),
                            _ => {}
                        }
                    }
                }

                // ── Input (insert + edit) ─────────────────────────────────────
                Mode::Input { .. } => match key.code {
                    KeyCode::Enter => {
                        let is_insert = matches!(&app.mode,
                            Mode::Input { action: InputAction::InsertChild { .. }, .. });
                        app.confirm_input();
                        if is_insert { app.scroll_to_input(canvas_h); }
                    }
                    KeyCode::Esc       => app.cancel_input(),
                    KeyCode::Backspace => app.input_backspace(),
                    KeyCode::Left      => app.input_move_cursor(-1),
                    KeyCode::Right     => app.input_move_cursor(1),
                    KeyCode::Tab       => { app.input_indent();  app.scroll_to_input(canvas_h); }
                    KeyCode::BackTab   => { app.input_dedent();  app.scroll_to_input(canvas_h); }
                    KeyCode::Char(c)   => app.input_char(c),
                    _ => {}
                },

                // ── Reparent ─────────────────────────────────────────────────
                Mode::Reparent { .. } => {
                    app.recompute_layout();
                    match (key.modifiers, key.code) {
                        (_, KeyCode::Esc)                                              => app.cancel_reparent(),
                        (KeyModifiers::NONE, KeyCode::Char('v')) | (_, KeyCode::Enter) => { app.confirm_reparent(); }
                        (KeyModifiers::NONE, KeyCode::Char('j')) | (_, KeyCode::Down)  => { app.reparent_nav_vertical(1);  app.scroll_to_selected(canvas_h); }
                        (KeyModifiers::NONE, KeyCode::Char('k')) | (_, KeyCode::Up)    => { app.reparent_nav_vertical(-1); app.scroll_to_selected(canvas_h); }
                        (KeyModifiers::NONE, KeyCode::Char('h')) | (_, KeyCode::Left)  => { app.reparent_nav_parent();     app.scroll_to_selected(canvas_h); }
                        (KeyModifiers::NONE, KeyCode::Char('l')) | (_, KeyCode::Right) => { app.reparent_nav_child();      app.scroll_to_selected(canvas_h); }
                        _ => {}
                    }
                }

                // ── Help ─────────────────────────────────────────────────────
                Mode::Help => {
                    match (key.modifiers, key.code) {
                        (_, KeyCode::Esc) | (KeyModifiers::NONE, KeyCode::Char('?')) => app.canvas_close_help(),
                        (KeyModifiers::NONE, KeyCode::Char('j')) | (_, KeyCode::Down) => app.cursor_move(0, 1, canvas_w, canvas_h as u16),
                        (KeyModifiers::NONE, KeyCode::Char('k')) | (_, KeyCode::Up)   => app.cursor_move(0, -1, canvas_w, canvas_h as u16),
                        (KeyModifiers::NONE, KeyCode::Char('h')) | (_, KeyCode::Left) => app.cursor_move(-1, 0, canvas_w, canvas_h as u16),
                        (KeyModifiers::NONE, KeyCode::Char('l')) | (_, KeyCode::Right) => app.cursor_move(1, 0, canvas_w, canvas_h as u16),
                        (KeyModifiers::NONE, KeyCode::Enter) | (KeyModifiers::NONE, KeyCode::Char('z')) | (_, KeyCode::Char(' ')) => {
                            app.toggle_collapse();
                            app.recompute_layout();
                        }
                        _ => {}
                    }
                }

                // ── Canvas ───────────────────────────────────────────────────
                Mode::Canvas { state: ref cs } => {
                    // Goto sub-state: search and jump to node.
                    if let CanvasState::Goto { .. } = cs {
                        match key.code {
                            KeyCode::Enter     => app.canvas_goto_confirm(canvas_w, canvas_h as u16),
                            KeyCode::Esc       => app.canvas_cancel_sub(),
                            KeyCode::Tab       => app.canvas_goto_next(),
                            KeyCode::Backspace => app.canvas_goto_backspace(),
                            KeyCode::Left      => app.canvas_goto_move_cursor(-1),
                            KeyCode::Right     => app.canvas_goto_move_cursor(1),
                            KeyCode::Char(c)   => app.canvas_goto_input_char(c),
                            _ => {}
                        }
                    }
                    // Pick sub-state: allow typing coordinates
                    else if let CanvasState::Pick { .. } = cs {
                        match (key.modifiers, key.code) {
                            (_, KeyCode::Enter)     => app.canvas_pick_or_place(),
                            (_, KeyCode::Esc)       => app.canvas_cancel_sub(),
                            (_, KeyCode::Backspace) => app.canvas_pick_backspace(),
                            (KeyModifiers::NONE, KeyCode::Char(c)) if c.is_ascii_digit() || c == ',' || c == '-' => app.canvas_pick_char(c),
                            
                            // Allow movement while picking
                            (KeyModifiers::NONE, KeyCode::Char('h')) | (_, KeyCode::Left)  => app.cursor_move(-1, 0, canvas_w, canvas_h as u16),
                            (KeyModifiers::NONE, KeyCode::Char('l')) | (_, KeyCode::Right) => app.cursor_move(1, 0, canvas_w, canvas_h as u16),
                            (KeyModifiers::NONE, KeyCode::Char('k')) | (_, KeyCode::Up)    => app.cursor_move(0, -1, canvas_w, canvas_h as u16),
                            (KeyModifiers::NONE, KeyCode::Char('j')) | (_, KeyCode::Down)  => app.cursor_move(0, 1, canvas_w, canvas_h as u16),
                            
                            _ => {}
                        }
                    }
                    // New sub-state: all keystrokes go to the text buffer.
                    else if let CanvasState::New { .. } = cs {
                        match key.code {
                            KeyCode::Enter     => {
                                app.canvas_confirm_new();
                                // If a root node was created we're now in InsertChild — scroll to it.
                                if matches!(&app.mode, Mode::Input { action: InputAction::InsertChild { .. }, .. }) {
                                    app.scroll_to_input(canvas_h);
                                }
                            }
                            KeyCode::Esc       => app.canvas_cancel_sub(),
                            KeyCode::Backspace => app.canvas_new_backspace(),
                            KeyCode::Left      => app.canvas_new_move_cursor(-1),
                            KeyCode::Right     => app.canvas_new_move_cursor(1),
                            KeyCode::Char(c)   => app.canvas_new_char(c),
                            _ => {}
                        }
                    } else if matches!(&app.mode,
                        Mode::Canvas { state: CanvasState::Menu }
                        | Mode::Canvas { state: CanvasState::MenuIncoming }
                        | Mode::Canvas { state: CanvasState::MenuOutgoing })
                    {
                        match &app.mode {
                            Mode::Canvas { state: CanvasState::Menu } => match key.code {
                                KeyCode::Char('i') => app.mode = Mode::Canvas { state: CanvasState::MenuIncoming },
                                KeyCode::Char('o') => app.mode = Mode::Canvas { state: CanvasState::MenuOutgoing },
                                KeyCode::Char('g') => { app.arrow.global = !app.arrow.global; app.save_project(); }
                                KeyCode::Char('F') | KeyCode::Esc => {
                                    app.mode = Mode::Canvas { state: CanvasState::Browse };
                                }
                                _ => {}
                            },
                            Mode::Canvas { state: CanvasState::MenuIncoming } => match key.code {
                                KeyCode::Char('T') | KeyCode::Char('t') => {
                                    app.arrow.incoming = ArrowFidelity::Tree;
                                    app.mode = Mode::Canvas { state: CanvasState::Menu };
                                    app.save_project();
                                }
                                KeyCode::Char('S') | KeyCode::Char('s') => {
                                    app.arrow.incoming = ArrowFidelity::Selected;
                                    app.mode = Mode::Canvas { state: CanvasState::Menu };
                                    app.save_project();
                                }
                                KeyCode::Esc => app.mode = Mode::Canvas { state: CanvasState::Menu },
                                _ => {}
                            },
                            Mode::Canvas { state: CanvasState::MenuOutgoing } => match key.code {
                                KeyCode::Char('T') | KeyCode::Char('t') => {
                                    app.arrow.outgoing = ArrowFidelity::Tree;
                                    app.mode = Mode::Canvas { state: CanvasState::Menu };
                                    app.save_project();
                                }
                                KeyCode::Char('S') | KeyCode::Char('s') => {
                                    app.arrow.outgoing = ArrowFidelity::Selected;
                                    app.mode = Mode::Canvas { state: CanvasState::Menu };
                                    app.save_project();
                                }
                                KeyCode::Esc => app.mode = Mode::Canvas { state: CanvasState::Menu },
                                _ => {}
                            },
                            _ => {}
                        }
                    } else {
                        // Browse + Pick + Link: cursor movement and all top-level actions.
                        match (key.modifiers, key.code) {
                            (KeyModifiers::NONE, KeyCode::Char('q')) => {
                                app.quit_to_main_menu();
                            }
                            (KeyModifiers::SHIFT, KeyCode::Char('Q')) => {
                                app.save_project();
                                break;
                            }

                            // ── Link confirmation / cancellation ──────────────
                            (_, KeyCode::Enter) => { app.canvas_confirm_link(); }
                            (_, KeyCode::Esc)   => {
                                if matches!(app.mode, Mode::Canvas { state: CanvasState::Browse }) {
                                    app.quit_to_main_menu();
                                } else {
                                    app.canvas_cancel_sub();
                                }
                            }

                            // ── Cursor movement ───────────────────────────────
                            (KeyModifiers::NONE, KeyCode::Char('h')) | (_, KeyCode::Left)  => app.cursor_move(-1, 0, canvas_w, canvas_h as u16),
                            (KeyModifiers::NONE, KeyCode::Char('l')) | (_, KeyCode::Right) => app.cursor_move(1, 0, canvas_w, canvas_h as u16),
                            (KeyModifiers::NONE, KeyCode::Char('k')) | (_, KeyCode::Up)    => app.cursor_move(0, -1, canvas_w, canvas_h as u16),
                            (KeyModifiers::NONE, KeyCode::Char('j')) | (_, KeyCode::Down)  => app.cursor_move(0, 1, canvas_w, canvas_h as u16),
                            // ── Cardinal warp: jump to next occupied row/column.
                            (KeyModifiers::SHIFT, KeyCode::Char('H')) => app.cursor_warp(-1,  0, canvas_w, canvas_h as u16),
                            (KeyModifiers::SHIFT, KeyCode::Char('L')) => app.cursor_warp( 1,  0, canvas_w, canvas_h as u16),
                            (KeyModifiers::SHIFT, KeyCode::Char('K')) => app.cursor_warp( 0, -1, canvas_w, canvas_h as u16),
                            (KeyModifiers::SHIFT, KeyCode::Char('J')) => app.cursor_warp( 0,  1, canvas_w, canvas_h as u16),

                            // ── Node operations ───────────────────────────────
                            (KeyModifiers::NONE, KeyCode::Char('i'))  => {
                                if app.has_selection() { app.enter_insert(); }
                                else                   { app.canvas_start_new(); }
                            }
                            (KeyModifiers::NONE, KeyCode::Char('e'))  => app.enter_edit(),
                            (KeyModifiers::SHIFT, KeyCode::Char('E')) => app.enter_overwrite(),
                            (KeyModifiers::NONE, KeyCode::Char('v'))  => app.enter_reparent(),
                            (KeyModifiers::NONE, KeyCode::Char('x'))  => { app.delete_selected(); }
                            (KeyModifiers::NONE, KeyCode::Char('p'))  => { app.canvas_pick_or_place(); }
                            (KeyModifiers::NONE, KeyCode::Char('f'))  => app.canvas_start_link(),
                            (KeyModifiers::NONE, KeyCode::Char('g'))  => app.canvas_start_goto(),
                            (KeyModifiers::NONE, KeyCode::Char('u'))  => app.undo(),
                            (KeyModifiers::NONE, KeyCode::Char('?'))  => app.canvas_start_help(),
                            (KeyModifiers::SHIFT, KeyCode::Char('F')) => {

                                app.mode = Mode::Canvas { state: CanvasState::Menu };
                            }

                            // ── Structure ─────────────────────────────────────
                            (KeyModifiers::NONE, KeyCode::Char('d'))  => { app.indent_increase(); app.recompute_layout(); }
                            (KeyModifiers::SHIFT, KeyCode::Char('D')) => { app.indent_decrease(); app.recompute_layout(); }
                            (KeyModifiers::NONE, KeyCode::Char('z')) | (_, KeyCode::Char(' ')) => {
                                app.toggle_collapse(); app.recompute_layout(); app.save_project();
                            }

                            // ── Camera ────────────────────────────────────────
                            (_, KeyCode::Char('c')) => app.center_on_selected(canvas_w, canvas_h as u16),

                            (KeyModifiers::NONE, KeyCode::Tab)   => app.canvas_jump_link(1, canvas_w, canvas_h as u16),
                            (KeyModifiers::SHIFT, KeyCode::BackTab) => app.canvas_jump_link(-1, canvas_w, canvas_h as u16),

                            _ => {}

                        }
                    }
                }
            }

        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
