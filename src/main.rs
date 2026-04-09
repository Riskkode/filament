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

use app::{App, Mode};
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
                // ── Insert ────────────────────────────────────────────────────
                Mode::Insert { .. } => match key.code {
                    KeyCode::Enter     => { app.confirm_insert(); app.scroll_to_insert(canvas_h); }
                    KeyCode::Esc       => app.cancel_insert(),
                    KeyCode::Backspace => app.insert_backspace(),
                    KeyCode::Left      => app.insert_move_cursor(-1),
                    KeyCode::Right     => app.insert_move_cursor(1),
                    KeyCode::Tab       => { app.insert_indent();  app.scroll_to_insert(canvas_h); }
                    KeyCode::BackTab   => { app.insert_dedent();  app.scroll_to_insert(canvas_h); }
                    KeyCode::Char(c)   => app.insert_char(c),
                    _ => {}
                },

                // ── Confirm delete ────────────────────────────────────────────
                Mode::Confirm { .. } => match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_delete(),
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_confirm(),
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
                        (KeyModifiers::SHIFT, KeyCode::Char('H')) => app.pan(-4, 0),
                        (KeyModifiers::SHIFT, KeyCode::Char('L')) => app.pan(4, 0),
                        (KeyModifiers::SHIFT, KeyCode::Char('K')) => app.pan(0, -2),
                        (KeyModifiers::SHIFT, KeyCode::Char('J')) => app.pan(0, 2),
                        _ => {}
                    }
                }

                // ── Nodes mode ────────────────────────────────────────────────
                Mode::Nodes { .. } => match (key.modifiers, key.code) {
                    (_, KeyCode::Esc)                                              => app.cancel_nodes(),
                    (KeyModifiers::NONE, KeyCode::Char('n'))                       => app.nodes_start_new(),
                    (KeyModifiers::NONE, KeyCode::Char('p'))                       => app.nodes_pick_or_place(),
                    (KeyModifiers::NONE, KeyCode::Char('h')) | (_, KeyCode::Left)  => app.nodes_move(-1, 0, canvas_w, canvas_h as u16),
                    (KeyModifiers::NONE, KeyCode::Char('l')) | (_, KeyCode::Right) => app.nodes_move(1, 0, canvas_w, canvas_h as u16),
                    (KeyModifiers::NONE, KeyCode::Char('k')) | (_, KeyCode::Up)    => app.nodes_move(0, -1, canvas_w, canvas_h as u16),
                    (KeyModifiers::NONE, KeyCode::Char('j')) | (_, KeyCode::Down)  => app.nodes_move(0, 1, canvas_w, canvas_h as u16),
                    _ => {}
                },

                // ── NodeNew ───────────────────────────────────────────────────
                Mode::NodeNew { .. } => match key.code {
                    KeyCode::Enter     => app.confirm_node_new(),
                    KeyCode::Esc       => app.cancel_node_new(),
                    KeyCode::Backspace => app.node_new_backspace(),
                    KeyCode::Left      => app.node_new_move_cursor(-1),
                    KeyCode::Right     => app.node_new_move_cursor(1),
                    KeyCode::Char(c)   => app.node_new_char(c),
                    _ => {}
                },

                // ── NodePick ─────────────────────────────────────────────────
                Mode::NodePick { .. } => match (key.modifiers, key.code) {
                    (_, KeyCode::Esc)                                              => app.cancel_nodes(),
                    (KeyModifiers::NONE, KeyCode::Char('p'))                       => app.nodes_pick_or_place(),
                    (KeyModifiers::NONE, KeyCode::Char('h')) | (_, KeyCode::Left)  => app.nodes_pick_move(-1, 0, canvas_w, canvas_h as u16),
                    (KeyModifiers::NONE, KeyCode::Char('l')) | (_, KeyCode::Right) => app.nodes_pick_move(1, 0, canvas_w, canvas_h as u16),
                    (KeyModifiers::NONE, KeyCode::Char('k')) | (_, KeyCode::Up)    => app.nodes_pick_move(0, -1, canvas_w, canvas_h as u16),
                    (KeyModifiers::NONE, KeyCode::Char('j')) | (_, KeyCode::Down)  => app.nodes_pick_move(0, 1, canvas_w, canvas_h as u16),
                    _ => {}
                },

                // ── Normal ────────────────────────────────────────────────────
                Mode::Normal => match (key.modifiers, key.code) {
                    (_, KeyCode::Char('q')) => break,

                    (KeyModifiers::NONE, KeyCode::Char('j')) | (_, KeyCode::Down)  => { app.nav_down();   app.scroll_to_selected(canvas_h); }
                    (KeyModifiers::NONE, KeyCode::Char('k')) | (_, KeyCode::Up)    => { app.nav_up();     app.scroll_to_selected(canvas_h); }
                    (KeyModifiers::NONE, KeyCode::Char('h')) | (_, KeyCode::Left)  => { app.nav_parent(); app.scroll_to_selected(canvas_h); }
                    (KeyModifiers::NONE, KeyCode::Char('l')) | (_, KeyCode::Right) => { app.nav_child();  app.scroll_to_selected(canvas_h); }

                    (KeyModifiers::SHIFT, KeyCode::Char('H')) => app.pan(-4, 0),
                    (KeyModifiers::SHIFT, KeyCode::Char('L')) => app.pan(4, 0),
                    (KeyModifiers::SHIFT, KeyCode::Char('K')) => app.pan(0, -2),
                    (KeyModifiers::SHIFT, KeyCode::Char('J')) => app.pan(0, 2),

                    (_, KeyCode::Char('c')) => app.center_on_selected(canvas_w, canvas_h as u16),

                    (KeyModifiers::NONE, KeyCode::Char('z')) | (_, KeyCode::Char(' ')) => {
                        app.toggle_collapse(); app.recompute_layout(); app.scroll_to_selected(canvas_h);
                    }

                    (KeyModifiers::NONE, KeyCode::Char('d')) => { app.indent_increase(); app.recompute_layout(); app.scroll_to_selected(canvas_h); }
                    (KeyModifiers::SHIFT, KeyCode::Char('D')) => { app.indent_decrease(); app.recompute_layout(); app.scroll_to_selected(canvas_h); }

                    (KeyModifiers::NONE, KeyCode::Char('v')) => app.enter_reparent(),
                    (KeyModifiers::NONE, KeyCode::Char('i')) => app.enter_insert(),
                    (KeyModifiers::NONE, KeyCode::Char('x')) => app.enter_confirm_delete(),
                    (KeyModifiers::NONE, KeyCode::Char('n')) => app.enter_nodes(canvas_w, canvas_h as u16),

                    _ => {}
                },
            }
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
