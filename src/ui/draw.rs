use crate::app::{App, Mode};
use crate::ui::prefix::{box_prefix, compute_insert_trail, compute_is_last};
use crate::ui::widgets::{draw_elbow_arrow, put_char};
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Terminal,
};
use std::io;

pub fn draw(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    let order   = app.recompute_layout();
    let is_last = compute_is_last(&order);

    terminal.draw(|frame| {
        let area = frame.area();

        // ── Border / title ────────────────────────────────────────────────────
        let (title, title_style) = match &app.mode {
            Mode::Normal => (
                " filament  [i insert | x delete | d/D depth | v reparent | n nodes | hjkl nav | HJKL pan | c center | z collapse | q quit] ",
                Style::default(),
            ),
            Mode::Insert { .. } => (
                " filament — INSERT  [Enter add | Tab indent | Shift+Tab dedent | ←/→ cursor | Esc done] ",
                Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD),
            ),
            Mode::Confirm { .. } => (
                " filament — DELETE  [y confirm | n/Esc cancel] ",
                Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Mode::Reparent { .. } => (
                " filament — REPARENT  [hjkl navigate | v/Enter confirm | Esc cancel] ",
                Style::default().fg(Color::Black).bg(Color::Blue).add_modifier(Modifier::BOLD),
            ),
            Mode::Nodes { .. } => (
                " filament — NODES  [hjkl move cursor | n new node | p pick | Esc exit] ",
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Mode::NodeNew { .. } => (
                " filament — NEW NODE  [type name | Enter confirm | Esc cancel] ",
                Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD),
            ),
            Mode::NodePick { .. } => (
                " filament — PICK  [hjkl move | p place | Esc cancel] ",
                Style::default().fg(Color::Black).bg(Color::Magenta).add_modifier(Modifier::BOLD),
            ),
        };

        frame.render_widget(
            Block::default().borders(Borders::ALL).title(Span::styled(title, title_style)),
            area,
        );

        let canvas = Rect {
            x:      area.x + 1,
            y:      area.y + 1,
            width:  area.width.saturating_sub(2),
            height: area.height.saturating_sub(3),
        };

        // ── Bullet lists ──────────────────────────────────────────────────────
        let prefix_style = Style::default().fg(Color::DarkGray);
        let arrow_style  = Style::default().fg(Color::DarkGray);
        let dim_style    = Style::default().fg(Color::DarkGray);
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

            let is_reparent_subj  = matches!(&app.mode, Mode::Reparent { subject, .. } if *subject == id);
            let is_reparent_cur   = matches!(&app.mode, Mode::Reparent { cursor,  .. } if *cursor  == id);
            let is_insert_parent  = matches!(&app.mode, Mode::Insert   { parent,  .. } if *parent  == id);
            let is_confirm_target = matches!(&app.mode, Mode::Confirm  { target      } if *target  == id);
            let is_pick_origin    = matches!(&app.mode, Mode::NodePick { origin_id, .. } if *origin_id == id);

            let label_style = if is_confirm_target {
                Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD)
            } else if is_reparent_subj || is_pick_origin {
                Style::default().fg(Color::Black).bg(Color::Magenta).add_modifier(Modifier::BOLD)
            } else if is_reparent_cur {
                Style::default().fg(Color::Black).bg(Color::Blue).add_modifier(Modifier::BOLD)
            } else if is_insert_parent {
                Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)
            } else if app.has_selection() && id == app.selected {
                Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };

            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(prefix,             prefix_style),
                    Span::styled("> ",               arrow_style),
                    Span::styled(node.label.clone(), label_style),
                    Span::styled(collapse_suffix,    dim_style),
                ])),
                Rect { x: canvas.x + sx, y: canvas.y + sy, width: canvas.width.saturating_sub(sx), height: 1 },
            );
        }

        // ── Nodes mode cursor ─────────────────────────────────────────────────
        if let Mode::Nodes { cursor_x, cursor_y } = app.mode {
            let sx = cursor_x - app.camera_x;
            let sy = cursor_y - app.camera_y;
            if sx >= 0 && sy >= 0 {
                let (sx, sy) = (sx as u16, sy as u16);
                if sx < canvas.width && sy < canvas.height {
                    put_char(frame, canvas, sx, sy, '╋',
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
                }
            }
        }

        // ── NodeNew inline prompt ─────────────────────────────────────────────
        if let Mode::NodeNew { cursor_x, cursor_y, buf, text_cursor } = &app.mode {
            let sx = cursor_x - app.camera_x;
            let sy = cursor_y - app.camera_y;
            if sx >= 0 && sy >= 0 {
                let (sx, sy) = (sx as u16, sy as u16);
                if sx < canvas.width && sy < canvas.height {
                    let before = &buf[..*text_cursor];
                    let cur_char = buf[*text_cursor..].chars().next().unwrap_or(' ');
                    let after_start = *text_cursor + cur_char.len_utf8();
                    let after = if after_start <= buf.len() { &buf[after_start..] } else { "" };
                    frame.render_widget(
                        Paragraph::new(Line::from(vec![
                            Span::styled("> ",                 Style::default().fg(Color::Green)),
                            Span::styled(before.to_string(),   Style::default().fg(Color::Green)),
                            Span::styled(cur_char.to_string(), Style::default().fg(Color::Black).bg(Color::Green)),
                            Span::styled(after.to_string(),    Style::default().fg(Color::Green)),
                        ])),
                        Rect { x: canvas.x + sx, y: canvas.y + sy, width: canvas.width.saturating_sub(sx), height: 1 },
                    );
                }
            }
        }

        // ── NodePick arrow + cursor ───────────────────────────────────────────
        if let Mode::NodePick { cursor_x, cursor_y, origin_x, origin_y, .. } = app.mode {
            let arrow_style = Style::default().fg(Color::Magenta);
            draw_elbow_arrow(frame, canvas, app.camera_x, app.camera_y,
                origin_x, origin_y, cursor_x, cursor_y, arrow_style);
            let sx = cursor_x - app.camera_x;
            let sy = cursor_y - app.camera_y;
            if sx >= 0 && sy >= 0 {
                let (sx, sy) = (sx as u16, sy as u16);
                if sx < canvas.width && sy < canvas.height {
                    put_char(frame, canvas, sx, sy, '⊕',
                        Style::default().fg(Color::Black).bg(Color::Magenta).add_modifier(Modifier::BOLD));
                }
            }
        }

        // ── Inline insert prompt ──────────────────────────────────────────────
        if let Mode::Insert { parent, buf, cursor } = &app.mode {
            let parent = *parent;
            let subtree = app.collect_subtree(parent);
            let visible = subtree.iter().filter(|&&id| app.nodes[id].row != usize::MAX).count();
            let insert_wy = app.nodes[parent].world_y + visible as i32;
            let insert_wx = app.nodes[parent].world_x;
            let sx = insert_wx - app.camera_x;
            let sy = insert_wy  - app.camera_y;
            if sx >= 0 && sy >= 0 {
                let (sx, sy) = (sx as u16, sy as u16);
                if sx < canvas.width && sy < canvas.height {
                    let trail  = compute_insert_trail(&app.nodes, parent);
                    let prefix = box_prefix(&trail);
                    let before = &buf[..*cursor];
                    let cur_char = buf[*cursor..].chars().next().unwrap_or(' ');
                    let after_start = *cursor + cur_char.len_utf8();
                    let after = if after_start <= buf.len() { &buf[after_start..] } else { "" };
                    frame.render_widget(
                        Paragraph::new(Line::from(vec![
                            Span::styled(prefix,                  Style::default().fg(Color::DarkGray)),
                            Span::styled("> ",                    Style::default().fg(Color::Green)),
                            Span::styled(before.to_string(),      Style::default().fg(Color::Green)),
                            Span::styled(cur_char.to_string(),    Style::default().fg(Color::Black).bg(Color::Green)),
                            Span::styled(after.to_string(),       Style::default().fg(Color::Green)),
                        ])),
                        Rect { x: canvas.x + sx, y: canvas.y + sy, width: canvas.width.saturating_sub(sx), height: 1 },
                    );
                }
            }
        }

        // ── Confirm delete overlay ─────────────────────────────────────────────
        if let Mode::Confirm { target } = &app.mode {
            let label   = &app.nodes[*target].label;
            let subtree = app.collect_subtree(*target);
            let desc    = subtree.len() - 1;
            let body = if desc == 0 { format!(" Delete \"{}\"? [y]es / [n]o ", label) }
                       else { format!(" Delete \"{}\" + {} descendant(s)? [y]es / [n]o ", label, desc) };
            let dlg_w = (body.len() as u16 + 2).min(canvas.width);
            let dlg   = Rect {
                x: canvas.x + canvas.width.saturating_sub(dlg_w) / 2,
                y: canvas.y + canvas.height.saturating_sub(3) / 2,
                width: dlg_w, height: 3,
            };
            frame.render_widget(Clear, dlg);
            frame.render_widget(
                Block::default().borders(Borders::ALL)
                    .style(Style::default().fg(Color::Red))
                    .title(Span::styled(" Confirm Delete ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))),
                dlg,
            );
            frame.render_widget(
                Paragraph::new(body.trim().to_string()).style(Style::default().fg(Color::White)),
                Rect { x: dlg.x + 1, y: dlg.y + 1, width: dlg.width.saturating_sub(2), height: 1 },
            );
        }

        // ── Status bar ────────────────────────────────────────────────────────
        let status = match &app.mode {
            Mode::Normal => {
                if app.has_selection() {
                    let node  = &app.nodes[app.selected];
                    let depth = { let mut d = 0u32; let mut cur = app.selected;
                        while let Some(p) = app.nodes[cur].parent { cur = p; d += 1; } d };
                    format!(" [{}] \"{}\"  depth:{}  world({},{})  camera({},{})  {} nodes ",
                        app.selected, node.label, depth, node.world_x, node.world_y,
                        app.camera_x, app.camera_y, app.nodes.len())
                } else {
                    format!(" camera({},{})  {} nodes  — press n to enter Nodes mode ",
                        app.camera_x, app.camera_y, app.nodes.len())
                }
            }
            Mode::Nodes { cursor_x, cursor_y } =>
                format!(" NODES  cursor({},{})  camera({},{}) ", cursor_x, cursor_y, app.camera_x, app.camera_y),
            Mode::NodeNew { cursor_x, cursor_y, .. } =>
                format!(" NEW NODE  at({},{}) ", cursor_x, cursor_y),
            Mode::NodePick { origin_id, cursor_x, cursor_y, .. } =>
                format!(" PICK  \"{}\"  → ({},{}) ", app.nodes[*origin_id].label, cursor_x, cursor_y),
            Mode::Reparent { subject, cursor, .. } =>
                format!(" REPARENT  \"{}\"  →  child of \"{}\" ", app.nodes[*subject].label, app.nodes[*cursor].label),
            Mode::Insert { parent, .. } =>
                format!(" INSERT  child of \"{}\" ", app.nodes[*parent].label),
            Mode::Confirm { target } => {
                let count = app.collect_subtree(*target).len();
                format!(" DELETE  \"{}\" and {} descendant(s) ", app.nodes[*target].label, count - 1)
            }
        };

        frame.render_widget(
            Paragraph::new(status).style(Style::default().fg(Color::DarkGray)),
            Rect { x: canvas.x, y: area.y + area.height.saturating_sub(2), width: canvas.width, height: 1 },
        );
    })?;
    Ok(())
}
