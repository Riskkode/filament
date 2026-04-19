use crate::app::{App, ArrowFidelity, CanvasState, InputAction, Mode, StartMenuState};
use crate::ui::palette::Palette;
use crate::ui::prefix::{box_prefix, compute_insert_trail, compute_is_last};
use crate::ui::titlebar;
use crate::ui::widgets::{route_link_into_buf, ArrowBuf, put_char};
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Terminal,
};
use std::collections::HashSet;
use std::io;
use chrono::{Local, TimeZone};

pub fn draw(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    if matches!(&app.mode, Mode::StartMenu { .. }) {
        return draw_start_menu(terminal, app);
    }

    let order   = app.recompute_layout();
    let is_last = compute_is_last(&order);

    // Visible-node bounds for obstacle-aware routing and incoming arrow scans.
    let visible_nodes: Vec<(i32, i32, i32)> = app.nodes.iter()
        .filter(|n| n.row != usize::MAX)
        .map(|n| (n.world_x, n.world_y, n.world_x_end))
        .collect();

    terminal.draw(|frame| {
        let area = frame.area();

        // ── Outer frame ───────────────────────────────────────────────────────
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(titlebar::border_style(&app.palette, &app.mode))
                .title(titlebar::build_title(&app.palette, &app.mode)),
            area,
        );

        let canvas = Rect {
            x:      area.x + 1,
            y:      area.y + 1,
            width:  area.width.saturating_sub(2),
            height: area.height.saturating_sub(3),
        };

        // ── Global arrow layer ────────────────────────────────────────────────
        if app.arrow.global {
            let mut global_buf = ArrowBuf::new();
            for node in app.nodes.iter() {
                if node.row == usize::MAX { continue; }
                for &tgt_id in &node.links {
                    if tgt_id < app.nodes.len() && app.nodes[tgt_id].row != usize::MAX {
                        route_link_into_buf(
                            &mut global_buf, canvas, app.camera_x, app.camera_y,
                            &visible_nodes,
                            node.world_x, node.world_y, node.world_x_end,
                            app.nodes[tgt_id].world_x, app.nodes[tgt_id].world_y,
                            app.nodes[tgt_id].world_x_end,
                        );
                    }
                }
            }
            global_buf.flush(frame, canvas, app.palette.tinted(app.palette.arrow_dim));
        }

        // ── Highlighted arrow layer ───────────────────────────────────────────
        if app.has_selection() {
            let mut root = app.selected;
            while let Some(p) = app.nodes[root].parent { root = p; }
            let tree: Vec<usize> = app.collect_subtree(root);
            let tree_set: HashSet<usize> = tree.iter().copied().collect();

            let out_sources: HashSet<usize> = match app.arrow.outgoing {
                ArrowFidelity::Tree     => tree_set.clone(),
                ArrowFidelity::Selected => HashSet::from([app.selected]),
            };
            let in_targets: HashSet<usize> = match app.arrow.incoming {
                ArrowFidelity::Tree     => tree_set.clone(),
                ArrowFidelity::Selected => HashSet::from([app.selected]),
            };

            let mut pairs: HashSet<(usize, usize)> = HashSet::new();
            for &src_id in &out_sources {
                for &tgt_id in &app.nodes[src_id].links {
                    if tgt_id < app.nodes.len() && app.nodes[tgt_id].row != usize::MAX {
                        pairs.insert((src_id, tgt_id));
                    }
                }
            }
            for (src_id, node) in app.nodes.iter().enumerate() {
                if node.row == usize::MAX { continue; }
                for &tgt_id in &node.links {
                    if in_targets.contains(&tgt_id)
                        && tgt_id < app.nodes.len()
                        && app.nodes[tgt_id].row != usize::MAX
                    {
                        pairs.insert((src_id, tgt_id));
                    }
                }
            }

            let mut arrow_buf = ArrowBuf::new();
            for (src_id, tgt_id) in pairs {
                route_link_into_buf(
                    &mut arrow_buf, canvas, app.camera_x, app.camera_y,
                    &visible_nodes,
                    app.nodes[src_id].world_x, app.nodes[src_id].world_y,
                    app.nodes[src_id].world_x_end,
                    app.nodes[tgt_id].world_x, app.nodes[tgt_id].world_y,
                    app.nodes[tgt_id].world_x_end,
                );
            }
            arrow_buf.flush(frame, canvas, app.palette.tinted(app.palette.link_arrow));
        }

        // ── Link mode preview arrow ───────────────────────────────────────────
        if let Mode::Canvas { state: CanvasState::Link { origin_id } } = app.mode {
            let mut prev = ArrowBuf::new();
            route_link_into_buf(
                &mut prev, canvas, app.camera_x, app.camera_y,
                &visible_nodes,
                app.nodes[origin_id].world_x,
                app.nodes[origin_id].world_y,
                app.nodes[origin_id].world_x_end,
                app.cursor_x, app.cursor_y, app.cursor_x,
            );
            prev.flush(frame, canvas, app.palette.tinted(app.palette.link));
        }

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

            let status_span = match node.tags.get("status").map(|s| s.as_str()) {
                Some("todo")        => Span::styled("○ ", Style::default().fg(app.palette.status_todo)),
                Some("in_progress") => Span::styled("◑ ", Style::default().fg(app.palette.status_progress)),
                Some("completed")   => Span::styled("● ", Style::default().fg(app.palette.status_done)),
                Some("blocked")     => Span::styled("⊘ ", Style::default().fg(app.palette.status_blocked)),
                _ => Span::raw(""),
            };

            let is_reparent_subj = matches!(&app.mode, Mode::Reparent { subject, .. } if *subject == id);
            let is_reparent_cur  = matches!(&app.mode, Mode::Reparent { cursor,  .. } if *cursor  == id);
            let is_insert_parent = matches!(&app.mode,
                Mode::Input { action: InputAction::InsertChild { parent }, .. } if *parent == id);
            let is_pick_origin   = matches!(&app.mode,
                Mode::Canvas { state: CanvasState::Pick { origin_id, .. } } if *origin_id == id);
            let is_link_origin   = matches!(&app.mode,
                Mode::Canvas { state: CanvasState::Link { origin_id } } if *origin_id == id);

            let label_style = if is_reparent_subj || is_pick_origin {
                app.palette.solid(app.palette.pick)
            } else if is_link_origin {
                app.palette.solid(app.palette.link)
            } else if is_reparent_cur {
                app.palette.solid(app.palette.reparent)
            } else if is_insert_parent {
                app.palette.solid(app.palette.insert)
            } else if app.has_selection() && id == app.selected {
                app.palette.solid(app.palette.selected)
            } else {
                app.palette.tinted(app.palette.node)
            };

            let indicator = if app.selected == id { ">" } else { " " };

            let mut spans = vec![
                Span::styled(prefix,             Style::default().fg(app.palette.prefix)),
                Span::styled(format!("{} ", indicator), Style::default().fg(app.palette.dim)),
                status_span,
                Span::styled(node.label.clone(), label_style),
                Span::styled(collapse_suffix,    Style::default().fg(app.palette.dim)),
            ];

            // Render time tags
            let mut time_keys: Vec<_> = node.times.keys().cloned().collect();
            time_keys.sort();
            for key in time_keys {
                if let Some(&ts) = node.times.get(&key) {
                    let dt = Local.timestamp_opt(ts, 0).unwrap();
                    let date_str = dt.format("%Y-%m-%d").to_string();
                    spans.push(Span::styled(format!(" [{}: {}]", key, date_str), Style::default().fg(app.palette.dim)));
                }
            }

            frame.render_widget(
                Paragraph::new(Line::from(spans)),
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
                        CanvasState::Pick { .. } => ('⊕', app.palette.solid(app.palette.pick)),
                        _                        => ('╋', app.palette.tinted_bold(app.palette.canvas)),
                    };
                    put_char(frame, canvas, sx, sy, ch, style);
                }
            }
        }

        // ── Canvas pick arrow ─────────────────────────────────────────────────
        if let Mode::Canvas {
            state: CanvasState::Pick { origin_id, origin_x, origin_y, .. } } = app.mode
        {
            let ox_end = app.nodes[origin_id].world_x_end;
            let mut pick_buf = ArrowBuf::new();
            route_link_into_buf(
                &mut pick_buf, canvas, app.camera_x, app.camera_y,
                &visible_nodes,
                origin_x, origin_y, ox_end,
                app.cursor_x, app.cursor_y, app.cursor_x,
            );
            pick_buf.flush(frame, canvas, app.palette.tinted(app.palette.pick));
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
                    render_input_line(frame, canvas, sx, sy, "> ", buf, text_cursor, &app.palette, app.palette.insert);
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
                    render_input_line(frame, canvas, sx, sy, &prefix, buf, cursor, &app.palette, app.palette.insert);
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
                    render_input_line(frame, canvas, sx, sy, &prefix, buf, cursor, &app.palette, app.palette.edit);
                }
            }
        }

        // ── Canvas new-node inline prompt ─────────────────────────────────────
        if let Mode::Canvas { state: CanvasState::Pick { origin_id, origin_x, origin_y, .. } } = app.mode {
            let sx = origin_x - app.camera_x;
            let sy = origin_y - app.camera_y;
            if sx >= 0 && sy >= 0 {
                let (sx, sy) = (sx as u16, sy as u16);
                if sx < canvas.width && sy < canvas.height {
                    let label = &app.nodes[origin_id].label;
                    frame.render_widget(
                        Paragraph::new(Span::styled(label.clone(), app.palette.solid(app.palette.pick))),
                        Rect { x: canvas.x + sx, y: canvas.y + sy, width: label.chars().count() as u16, height: 1 }
                    );
                }
            }
        }

        // ── Goto modal prompt ────────────────────────────────────────────────
        if let Mode::Canvas { state: CanvasState::Goto { ref buf, cursor, ref matches, match_idx, .. } } = app.mode {
            let matches_info = if matches.is_empty() {
                if buf.is_empty() { String::new() } else { "(no matches)".to_string() }
            } else {
                format!("({}/{})", match_idx + 1, matches.len())
            };
            let prefix = format!("jump to: {} ", matches_info);
            render_input_line(frame, canvas, 0, canvas.height.saturating_sub(1), &prefix, buf, cursor, &app.palette, app.palette.edit);
        }

        // ── Pick modal prompt ────────────────────────────────────────────────
        if let Mode::Canvas { state: CanvasState::Pick { ref buf, cursor, .. } } = app.mode {
            if !buf.is_empty() {
                let prefix = "move to: ";
                render_input_line(frame, canvas, 0, canvas.height.saturating_sub(1), prefix, buf, cursor, &app.palette, app.palette.insert);
            }
        }

        // ── Arrow settings menu overlay ───────────────────────────────────────
        draw_arrow_menu(frame, canvas, app);

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

fn draw_arrow_menu(frame: &mut ratatui::Frame, canvas: Rect, app: &App) {
    let in_menu = matches!(&app.mode,
        Mode::Canvas { state: CanvasState::Menu | CanvasState::MenuIncoming | CanvasState::MenuOutgoing });
    if !in_menu { return; }

    let fid = |f: ArrowFidelity| match f {
        ArrowFidelity::Tree     => "Tree    ",
        ArrowFidelity::Selected => "Selected",
    };
    let tog = |b: bool| if b { "ON " } else { "off" };

    let hint = match &app.mode {
        Mode::Canvas { state: CanvasState::MenuIncoming } =>
            "  Set: [T]ree  [S]elected  Esc=back",
        Mode::Canvas { state: CanvasState::MenuOutgoing } =>
            "  Set: [T]ree  [S]elected  Esc=back",
        _ =>
            "  [i]ncoming  [o]utgoing  [g]lobal  [F]/Esc=close",
    };

    let incoming_style = if matches!(&app.mode, Mode::Canvas { state: CanvasState::MenuIncoming }) {
        app.palette.solid(app.palette.link_arrow)
    } else {
        app.palette.tinted(app.palette.link_arrow)
    };
    let outgoing_style = if matches!(&app.mode, Mode::Canvas { state: CanvasState::MenuOutgoing }) {
        app.palette.solid(app.palette.link_arrow)
    } else {
        app.palette.tinted(app.palette.link_arrow)
    };

    let lines = vec![
        Line::from(vec![
            Span::raw(" [g] Global   "),
            Span::styled(tog(app.arrow.global), app.palette.tinted(app.palette.link_arrow)),
        ]),
        Line::from(vec![
            Span::raw(" [i] Incoming "),
            Span::styled(fid(app.arrow.incoming), incoming_style),
        ]),
        Line::from(vec![
            Span::raw(" [o] Outgoing "),
            Span::styled(fid(app.arrow.outgoing), outgoing_style),
        ]),
        Line::from(Span::styled(hint, Style::default().fg(app.palette.dim))),
    ];

    let w: u16 = 52;
    let h: u16 = lines.len() as u16 + 2;
    let x = canvas.x + canvas.width.saturating_sub(w + 1);
    let y = canvas.y;

    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Arrow Display ")
                .border_style(app.palette.tinted(app.palette.canvas))),
        Rect { x, y, width: w, height: h },
    );
}

fn render_input_line(
    frame: &mut ratatui::Frame,
    canvas: Rect, sx: u16, sy: u16,
    prefix: &str, buf: &str, cursor: usize,
    pal: &Palette,
    color: Color,
) {
    let before    = &buf[..cursor];
    let cur_char  = buf[cursor..].chars().next().unwrap_or(' ');
    let after_off = cursor + cur_char.len_utf8();
    let after     = if after_off <= buf.len() { &buf[after_off..] } else { "" };

    let reset_bg = Style::default().bg(Color::Reset);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(prefix.to_string(),      reset_bg.patch(Style::default().fg(pal.dim))),
            Span::styled(before.to_string(),       reset_bg.patch(pal.tinted(color))),
            Span::styled(cur_char.to_string(),     pal.solid(color)),
            Span::styled(after.to_string(),        reset_bg.patch(pal.tinted(color))),
        ])),
        Rect { x: canvas.x + sx, y: canvas.y + sy, width: canvas.width.saturating_sub(sx), height: 1 },
    );
}

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
        Mode::StartMenu { state: StartMenuState::Main { selected } } => {
            if let Some(n) = app.nodes.get(*selected) {
                let mut base = format!(" {} │ Enter:select  e:edit  o/f/s/n/?:jump  j/k:nav  q/Q:quit ", n.label);
                if let Some(parent) = n.parent {
                    if app.nodes[parent].label.to_lowercase().contains("open") {
                        base = format!(" {} │ Enter:select  e:name  p:path  d:remove  o/f/s/n/?:jump  j/k:nav  q/Q:quit ", n.label);
                    }
                }
                base
            } else {
                " FILAMENT │ Enter:select  e:edit  o/f/s/n/?:jump  j/k:nav  q/Q:quit ".to_string()
            }
        }
        Mode::StartMenu { state: StartMenuState::NewPath { buf, .. } } =>
            format!(" new project location: \"{}\" ", buf),
        Mode::StartMenu { state: StartMenuState::NewName { path, buf, .. } } =>
            format!(" new project \"{}\" at {} ", buf, path),
        Mode::StartMenu { state: StartMenuState::EditSetting { key, buf, .. } } =>
            format!(" editing {}: \"{}\" ", key.replace('_', " "), buf),
        Mode::StartMenu { state: StartMenuState::Import { buf, matches, root, editing_root, .. } } => {
            let mode = if *editing_root { "editing location" } else { "fuzzy search" };
            format!(" import from {} │ {}: \"{}\" │ {} matches │ up/down:nav  Tab:change location  Enter:import  Esc:cancel ", root, mode, buf, matches.len())
        }
        Mode::Input { action: InputAction::InsertChild { parent }, buf, .. } =>
            format!(" inserting under \"{}\" │ \"{}\" ", app.nodes[*parent].label, buf),
        Mode::Input { action: InputAction::EditLabel { node } | InputAction::Overwrite { node }, buf, .. } =>
            format!(" editing \"{}\" → \"{}\" ", app.nodes[*node].label, buf),
        Mode::Reparent { subject, cursor, .. } =>
            format!(" reparenting \"{}\" → child of \"{}\" ", app.nodes[*subject].label, app.nodes[*cursor].label),
        Mode::Canvas { state: CanvasState::Menu | CanvasState::MenuIncoming | CanvasState::MenuOutgoing } =>
            format!(" arrow display │ global {} │ incoming {} │ outgoing {} ",
                if app.arrow.global { "ON" } else { "off" },
                match app.arrow.incoming { ArrowFidelity::Tree => "tree", ArrowFidelity::Selected => "selected" },
                match app.arrow.outgoing { ArrowFidelity::Tree => "tree", ArrowFidelity::Selected => "selected" },
            ),
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
        Mode::Canvas { state: CanvasState::Link { origin_id } } => {
            let target = app.node_near_cursor(app.cursor_x, app.cursor_y)
                .filter(|&t| t != *origin_id)
                .map(|t| format!("\"{}\"", app.nodes[t].label))
                .unwrap_or_else(|| "(none)".into());
            let existing = app.nodes[*origin_id].links.len();
            format!(" linking from \"{}\"  →  {}  │  {} outgoing links ",
                app.nodes[*origin_id].label, target, existing)
        }
        Mode::Canvas { state: CanvasState::Goto { buf, .. } } =>
            format!(" finding node: \"{}\" ", buf),
        Mode::Canvas { state: CanvasState::TagStatus } =>
            " status tag │ t:todo  p:progress  c:done  b:blocked  x:clear  Esc:cancel ".to_string(),
        Mode::Canvas { state: CanvasState::TagTime } =>
            " time tag │ d:deadline  s:start  e:end  c:checkpoint  u:duration  x:clear  Esc:cancel ".to_string(),
        Mode::Canvas { state: CanvasState::TimeInput { time_type, buf, .. } } =>
            format!(" enter {} │ \"{}\" ", time_type, buf),
        Mode::Help =>
            " Browsing Help tree │ hjkl:navigate  z:toggle  Esc/?:close ".to_string(),
    }
}

// ── Start menu screen ─────────────────────────────────────────────────────────

const BANNER: &[&str] = &[
    "███████╗██╗██╗      █████╗ ███╗   ███╗███████╗███╗   ██╗████████╗",
    "██╔════╝██║██║     ██╔══██╗████╗ ████║██╔════╝████╗  ██║╚══██╔══╝",
    "█████╗  ██║██║     ███████║██╔████╔██║█████╗  ██╔██╗ ██║   ██║   ",
    "██╔══╝  ██║██║     ██╔══██║██║╚██╔╝██║██╔══╝  ██║╚██╗██║   ██║   ",
    "██║     ██║███████╗██║  ██║██║ ╚═╝ ██║███████╗██║ ╚████║   ██║   ",
    "╚═╝     ╚═╝╚══════╝╚═╝  ╚═╝╚═╝     ╚═╝╚══════╝╚═╝  ╚═══╝   ╚═╝   ",
];

fn draw_start_menu(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    let order   = app.recompute_layout();
    let is_last = compute_is_last(&order);

    terminal.draw(|frame| {
        let area = frame.area();

        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(titlebar::border_style(&app.palette, &app.mode))
                .title(titlebar::build_title(&app.palette, &app.mode)),
            area,
        );

        let inner = Rect {
            x:      area.x + 1,
            y:      area.y + 1,
            width:  area.width.saturating_sub(2),
            height: area.height.saturating_sub(3),
        };

        // ── Render Banner (Centered) ──────────────────────────────────────────
        let banner_w = BANNER[0].chars().count() as u16;
        let banner_h = BANNER.len() as u16;
        let banner_x = inner.x + (inner.width.saturating_sub(banner_w)) / 2;
        let banner_y = inner.y + (inner.height.saturating_sub(banner_h + 12)) / 2;
        let banner_right_x = banner_x + banner_w;

        for (i, line) in BANNER.iter().enumerate() {
            frame.render_widget(
                Paragraph::new(*line).style(Style::default().fg(app.palette.selected)),
                Rect { x: banner_x, y: banner_y + i as u16, width: banner_w, height: 1 },
            );
        }

        // ── Render Nodes (Left-aligned with Banner) ──────────────────────────
        let mut max_node_w = 40u16;
        for (id, depth) in &order {
            let w = app.nodes[*id].label.chars().count() as u16 + (*depth as u16 * 2) + 6;
            max_node_w = max_node_w.max(w);
        }
        let menu_w = max_node_w.min(inner.width.saturating_sub(4));
        let menu_x = banner_x;
        let menu_y = banner_y + banner_h + 2;
        let visible_height = inner.height.saturating_sub(banner_h + 4);

        let mut trail: Vec<bool> = Vec::new();
        for (idx, &(id, depth)) in order.iter().enumerate() {
            let node = &app.nodes[id];

            trail.truncate(depth);
            if depth > 0 {
                if trail.len() < depth { trail.push(is_last[idx]); }
                else { *trail.last_mut().unwrap() = is_last[idx]; }
            }

            // Scrolling logic
            let wy = idx as i32;
            let sy = wy - app.camera_y;
            if sy < 0 || sy >= visible_height as i32 {
                continue;
            }
            let sy = sy as u16;

            let prefix = box_prefix(&trail[..depth.min(trail.len())]);
            let collapse_suffix = if node.children.is_empty() { String::new() }
                else if node.collapsed { format!("  [+{}]", node.children.len()) }
                else { String::new() };

            let (indicator, style) = if app.selected == id {
                (">", app.palette.solid(app.palette.selected))
            } else {
                (" ", app.palette.tinted(app.palette.node))
            };

            let label_lower = node.label.to_lowercase();
            let shortcut = if node.parent.is_none() {
                if label_lower.contains("open")     { Some("o") }
                else if label_lower.contains("find") { Some("f") }
                else if label_lower.contains("import") { Some("i") }
                else if label_lower.contains("settings") { Some("s") }
                else if label_lower.contains("+ new")  { Some("n") }
                else if label_lower.contains("help")   { Some("?") }
                else { None }
            } else { None };

            // Check if this node is currently being edited in-line
            let mut prompt_info = None;
            if app.selected == id {
                prompt_info = match &app.mode {
                    Mode::StartMenu { state: StartMenuState::NewPath { buf, cursor } } =>
                        Some(("location: ".to_string(), buf.as_str(), *cursor, app.palette.insert)),
                    Mode::StartMenu { state: StartMenuState::NewName { buf, cursor, .. } } =>
                        Some(("project: ".to_string(), buf.as_str(), *cursor, app.palette.insert)),
                    Mode::StartMenu { state: StartMenuState::EditSetting { key, buf, cursor } } => {
                        let prefix = if key.starts_with("rename_project:") {
                            "rename: ".to_string()
                        } else if key.starts_with("repath_project:") {
                            "repath: ".to_string()
                        } else {
                            format!("{}: ", key.replace('_', " "))
                        };
                        Some((prefix, buf.as_str(), *cursor, app.palette.edit))
                    }
                    Mode::StartMenu { state: StartMenuState::Import { buf, cursor, editing_root, .. } } => {
                        let prefix = if *editing_root { "location: ".to_string() } else { "search: ".to_string() };
                        Some((prefix, buf.as_str(), *cursor, app.palette.insert))
                    }
                    _ => None,
                };
            }

            if let Some((input_prefix, buf, cursor, color)) = prompt_info {
                let full_prefix = format!("{}{}{} ", prefix, indicator, input_prefix);
                render_input_line(
                    frame,
                    Rect { x: menu_x, y: menu_y + sy, width: menu_w, height: 1 },
                    0, 0,
                    &full_prefix, buf, cursor, &app.palette, color
                );
            } else {
                frame.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::styled(prefix,             Style::default().fg(app.palette.prefix)),
                        Span::styled(format!("{} ", indicator), Style::default().fg(app.palette.dim)),
                        Span::styled(node.label.clone(), style),
                        Span::styled(collapse_suffix,    Style::default().fg(app.palette.dim)),
                    ])),
                    Rect {
                        x: menu_x,
                        y: menu_y + sy,
                        width: menu_w,
                        height: 1
                    },
                );
            }

            if let Some(key) = shortcut {
                let shortcut_text = format!("[{}]", key);
                let shortcut_w = shortcut_text.chars().count() as u16;
                frame.render_widget(
                    Paragraph::new(Span::styled(shortcut_text, Style::default().fg(app.palette.insert))),
                    Rect {
                        x: banner_right_x.saturating_sub(shortcut_w),
                        y: menu_y + sy,
                        width: shortcut_w,
                        height: 1,
                    },
                );
            }
        }

        // ── Render Import Results ────────────────────────────────────────────
        if let Mode::StartMenu { state: StartMenuState::Import { matches, match_idx, .. } } = &app.mode {
            let result_y = menu_y + order.len() as u16 + 1;
            for (i, path) in matches.iter().enumerate() {
                let style = if i == *match_idx { app.palette.solid(app.palette.selected) } else { Style::default().fg(app.palette.dim) };
                frame.render_widget(
                    Paragraph::new(format!("  {} {}", if i == *match_idx { ">" } else { " " }, path)).style(style),
                    Rect { x: menu_x, y: result_y + i as u16, width: inner.width.saturating_sub(menu_x - inner.x), height: 1 }
                );
            }
        }

        // Status bar.
        let status = build_status(app);
        frame.render_widget(
            Paragraph::new(status).style(Style::default().fg(Color::DarkGray)),
            Rect {
                x:      inner.x,
                y:      area.y + area.height.saturating_sub(2),
                width:  inner.width,
                height: 1,
            },
        );
    })?;
    Ok(())
}
