use crate::app::{App, CanvasState, InputAction, Mode};
use crate::ui::palette as pal;
use crate::ui::prefix::{box_prefix, compute_insert_trail, compute_is_last};
use crate::ui::titlebar;
use crate::ui::widgets::{draw_elbow_arrow, put_char};
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;

pub fn draw(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    let order   = app.recompute_layout();
    let is_last = compute_is_last(&order);

    terminal.draw(|frame| {
        let area = frame.area();

        // ── Outer frame ───────────────────────────────────────────────────────
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_style(titlebar::border_style(&app.mode))
                .title(titlebar::build_title(&app.mode)),
            area,
        );

        let canvas = Rect {
            x:      area.x + 1,
            y:      area.y + 1,
            width:  area.width.saturating_sub(2),
            height: area.height.saturating_sub(3),
        };

        // ── Bullet lists ──────────────────────────────────────────────────────
        let mut trail: Vec<bool> = Vec::new();

        for (idx, &(id, depth)) in order.iter().enumerate() {
            let node = &app.nodes[id];

            trail.truncate(depth);
            if depth > 0 {
                if trail.len() < depth { trail.push(is_last[idx]); }
                else { *trail.last_mut().unwrap() = is_last[idx]; }
            }

            let sx = node.world_x - app.camera_x;
            let sy = node.world_y - app.camera_y;
            if sx < 0 || sy < 0 { continue; }
            let (sx, sy) = (sx as u16, sy as u16);
            if sx >= canvas.width || sy >= canvas.height { continue; }

            let prefix = box_prefix(&trail[..depth.min(trail.len())]);
            let collapse_suffix = if node.children.is_empty() { String::new() }
                else if node.collapsed { format!("  [+{}]", node.children.len()) }
                else { String::new() };

            let is_reparent_subj = matches!(&app.mode, Mode::Reparent { subject, .. } if *subject == id);
            let is_reparent_cur  = matches!(&app.mode, Mode::Reparent { cursor,  .. } if *cursor  == id);
            let is_insert_parent = matches!(&app.mode,
                Mode::Input { action: InputAction::InsertChild { parent }, .. } if *parent == id);
            let is_pick_origin   = matches!(&app.mode,
                Mode::Canvas { state: CanvasState::Pick { origin_id, .. }, .. } if *origin_id == id);

            let label_style = if is_reparent_subj || is_pick_origin {
                pal::solid(pal::PICK)
            } else if is_reparent_cur {
                pal::solid(pal::REPARENT)
            } else if is_insert_parent {
                pal::solid(pal::INSERT)
            } else if app.has_selection() && id == app.selected {
                pal::solid(pal::SELECTED)
            } else {
                pal::tinted(pal::NODE)
            };

            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(prefix,             Style::default().fg(pal::PREFIX)),
                    Span::styled("> ",               Style::default().fg(pal::DIM)),
                    Span::styled(node.label.clone(), label_style),
                    Span::styled(collapse_suffix,    Style::default().fg(pal::DIM)),
                ])),
                Rect { x: canvas.x + sx, y: canvas.y + sy, width: canvas.width.saturating_sub(sx), height: 1 },
            );
        }

        // ── Canvas cursor ─────────────────────────────────────────────────────
        if let Mode::Canvas { ref state } = app.mode {
            let sx = app.cursor_x - app.camera_x;
            let sy = app.cursor_y - app.camera_y;
            if sx >= 0 && sy >= 0 {
                let (sx, sy) = (sx as u16, sy as u16);
                if sx < canvas.width && sy < canvas.height {
                    let (ch, style) = match state {
                        CanvasState::Pick { .. } => ('⊕', pal::solid(pal::PICK)),
                        _                        => ('╋', pal::tinted_bold(pal::CANVAS)),
                    };
                    put_char(frame, canvas, sx, sy, ch, style);
                }
            }
        }

        // ── Canvas pick arrow ─────────────────────────────────────────────────
        if let Mode::Canvas {
            state: CanvasState::Pick { origin_x, origin_y, .. } } = app.mode
        {
            draw_elbow_arrow(frame, canvas, app.camera_x, app.camera_y,
                origin_x, origin_y, app.cursor_x, app.cursor_y,
                pal::tinted(pal::PICK));
        }

        // ── Canvas new-node inline prompt ─────────────────────────────────────
        if let Mode::Canvas {
            state: CanvasState::New { ref buf, text_cursor } } = app.mode
        {
            let sx = app.cursor_x - app.camera_x;
            let sy = app.cursor_y - app.camera_y;
            if sx >= 0 && sy >= 0 {
                let (sx, sy) = (sx as u16, sy as u16);
                if sx < canvas.width && sy < canvas.height {
                    render_input_line(frame, canvas, sx, sy, "> ", buf, text_cursor, pal::INSERT);
                }
            }
        }

        // ── Insert inline prompt ──────────────────────────────────────────────
        if let Mode::Input { action: InputAction::InsertChild { parent }, ref buf, cursor } = app.mode {
            let subtree = app.collect_subtree(parent);
            let visible = subtree.iter().filter(|&&id| app.nodes[id].row != usize::MAX).count();
            let wy = app.nodes[parent].world_y + visible as i32;
            let wx = app.nodes[parent].world_x;
            let sx = wx - app.camera_x;
            let sy = wy - app.camera_y;
            if sx >= 0 && sy >= 0 {
                let (sx, sy) = (sx as u16, sy as u16);
                if sx < canvas.width && sy < canvas.height {
                    let trail  = compute_insert_trail(&app.nodes, parent);
                    let prefix = format!("{}>  ", box_prefix(&trail));
                    render_input_line(frame, canvas, sx, sy, &prefix, buf, cursor, pal::INSERT);
                }
            }
        }

        // ── Edit inline prompt ────────────────────────────────────────────────
        if let Mode::Input {
            action: InputAction::EditLabel { node } | InputAction::Overwrite { node },
            ref buf, cursor
        } = app.mode {
            let sx = app.nodes[node].world_x - app.camera_x;
            let sy = app.nodes[node].world_y - app.camera_y;
            if sx >= 0 && sy >= 0 {
                let (sx, sy) = (sx as u16, sy as u16);
                if sx < canvas.width && sy < canvas.height {
                    let idx   = order.iter().position(|&(id, _)| id == node).unwrap_or(0);
                    let depth = order.get(idx).map(|&(_, d)| d).unwrap_or(0);
                    let trail = build_trail_for(&order, &is_last, idx, depth);
                    let prefix = format!("{}>  ", box_prefix(&trail));
                    render_input_line(frame, canvas, sx, sy, &prefix, buf, cursor, pal::EDIT);
                }
            }
        }

        // ── Status bar ────────────────────────────────────────────────────────
        let status = build_status(app);
        frame.render_widget(
            Paragraph::new(status).style(Style::default().fg(Color::DarkGray)),
            Rect { x: canvas.x, y: area.y + area.height.saturating_sub(2), width: canvas.width, height: 1 },
        );
    })?;
    Ok(())
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Render a text-input line with a block cursor at `cursor` byte offset.
fn render_input_line(
    frame: &mut ratatui::Frame,
    canvas: Rect, sx: u16, sy: u16,
    prefix: &str, buf: &str, cursor: usize,
    color: Color,
) {
    let before    = &buf[..cursor];
    let cur_char  = buf[cursor..].chars().next().unwrap_or(' ');
    let after_off = cursor + cur_char.len_utf8();
    let after     = if after_off <= buf.len() { &buf[after_off..] } else { "" };

    let reset_bg = Style::default().bg(Color::Reset);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(prefix.to_string(),      reset_bg.patch(Style::default().fg(pal::DIM))),
            Span::styled(before.to_string(),       reset_bg.patch(pal::tinted(color))),
            Span::styled(cur_char.to_string(),     pal::solid(color)),
            Span::styled(after.to_string(),        reset_bg.patch(pal::tinted(color))),
        ])),
        Rect { x: canvas.x + sx, y: canvas.y + sy, width: canvas.width.saturating_sub(sx), height: 1 },
    );
}

/// Reconstruct the box-drawing trail for a node at `idx`/`depth` from the layout order.
fn build_trail_for(order: &[(usize, usize)], is_last: &[bool], idx: usize, depth: usize) -> Vec<bool> {
    if depth == 0 { return vec![]; }
    let mut trail = Vec::with_capacity(depth);
    for target_depth in 1..=depth {
        let ancestor_idx = order[..idx].iter().rposition(|&(_, d)| d == target_depth - 1);
        trail.push(ancestor_idx.map(|ai| is_last[ai]).unwrap_or(true));
    }
    if let Some(last) = trail.last_mut() { *last = is_last[idx]; }
    trail
}

fn build_status(app: &App) -> String {
    match &app.mode {
        Mode::Input { action: InputAction::InsertChild { parent }, buf, .. } =>
            format!(" inserting under \"{}\" │ \"{}\" ", app.nodes[*parent].label, buf),
        Mode::Input { action: InputAction::EditLabel { node } | InputAction::Overwrite { node }, buf, .. } =>
            format!(" editing \"{}\" → \"{}\" ", app.nodes[*node].label, buf),
        Mode::Reparent { subject, cursor, .. } =>
            format!(" reparenting \"{}\" → child of \"{}\" ", app.nodes[*subject].label, app.nodes[*cursor].label),
        Mode::Canvas { state: CanvasState::Browse } => {
            if app.has_selection() {
                let node  = &app.nodes[app.selected];
                let depth = { let mut d = 0u32; let mut c = app.selected;
                    while let Some(p) = app.nodes[c].parent { c = p; d += 1; } d };
                format!(" {} │ depth {} │ cursor ({},{}) │ camera ({},{}) │ {} nodes ",
                    node.label, depth, app.cursor_x, app.cursor_y,
                    app.camera_x, app.camera_y, app.nodes.len())
            } else {
                format!(" cursor ({},{}) │ camera ({},{}) │ {} nodes ",
                    app.cursor_x, app.cursor_y, app.camera_x, app.camera_y, app.nodes.len())
            }
        }
        Mode::Canvas { state: CanvasState::New { buf, .. } } =>
            format!(" new node at ({},{}) │ \"{}\" ", app.cursor_x, app.cursor_y, buf),
        Mode::Canvas { state: CanvasState::Pick { origin_id, .. } } =>
            format!(" picking \"{}\" → ({},{}) ", app.nodes[*origin_id].label, app.cursor_x, app.cursor_y),
    }
}
