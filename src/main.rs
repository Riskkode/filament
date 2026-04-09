mod app;
mod db;
mod models;
mod repositories;
mod ui;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use app::{App, CanvasState, InputAction, Mode};
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
                        (KeyModifiers::NONE, KeyCode::Char('v')) | (_, KeyCode::Enter) => app.confirm_reparent(),
                        (KeyModifiers::NONE, KeyCode::Char('j')) | (_, KeyCode::Down)  => { app.reparent_nav_vertical(1);  app.scroll_to_selected(canvas_h); }
                        (KeyModifiers::NONE, KeyCode::Char('k')) | (_, KeyCode::Up)    => { app.reparent_nav_vertical(-1); app.scroll_to_selected(canvas_h); }
                        (KeyModifiers::NONE, KeyCode::Char('h')) | (_, KeyCode::Left)  => { app.reparent_nav_parent();     app.scroll_to_selected(canvas_h); }
                        (KeyModifiers::NONE, KeyCode::Char('l')) | (_, KeyCode::Right) => { app.reparent_nav_child();      app.scroll_to_selected(canvas_h); }
                        _ => {}
                    }
                }

                // ── Canvas ───────────────────────────────────────────────────
                Mode::Canvas { .. } => {
                    // New sub-state: all keystrokes go to the text buffer.
                    if matches!(&app.mode, Mode::Canvas { state: CanvasState::New { .. } }) {
                        match key.code {
                            KeyCode::Enter     => app.canvas_confirm_new(),
                            KeyCode::Esc       => app.canvas_cancel_sub(),
                            KeyCode::Backspace => app.canvas_new_backspace(),
                            KeyCode::Left      => app.canvas_new_move_cursor(-1),
                            KeyCode::Right     => app.canvas_new_move_cursor(1),
                            KeyCode::Char(c)   => app.canvas_new_char(c),
                            _ => {}
                        }
                    } else {
                        // Browse + Pick: cursor movement and all top-level actions.
                        match (key.modifiers, key.code) {
                            (_, KeyCode::Char('q')) => break,

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
                            (KeyModifiers::NONE, KeyCode::Char('i'))  => app.enter_insert(),
                            (KeyModifiers::NONE, KeyCode::Char('e'))  => app.enter_edit(),
                            (KeyModifiers::SHIFT, KeyCode::Char('E')) => app.enter_overwrite(),
                            (KeyModifiers::NONE, KeyCode::Char('v'))  => app.enter_reparent(),
                            (KeyModifiers::NONE, KeyCode::Char('x'))  => app.delete_selected(),
                            (KeyModifiers::NONE, KeyCode::Char('n'))  => app.canvas_start_new(),
                            (KeyModifiers::NONE, KeyCode::Char('p'))  => app.canvas_pick_or_place(),

                            // ── Structure ─────────────────────────────────────
                            (KeyModifiers::NONE, KeyCode::Char('d'))  => { app.indent_increase(); app.recompute_layout(); }
                            (KeyModifiers::SHIFT, KeyCode::Char('D')) => { app.indent_decrease(); app.recompute_layout(); }
                            (KeyModifiers::NONE, KeyCode::Char('z')) | (_, KeyCode::Char(' ')) => {
                                app.toggle_collapse(); app.recompute_layout();
                            }

                            // ── Camera ────────────────────────────────────────
                            (_, KeyCode::Char('c')) => app.center_on_selected(canvas_w, canvas_h as u16),

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
