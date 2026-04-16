use crate::app::{App, ArrowFidelity, CanvasState, InputAction, Mode, ProjectListState};
use crate::ui::palette as pal;
use crate::ui::prefix::{box_prefix, compute_insert_trail, compute_is_last};
use crate::ui::titlebar;
use crate::ui::widgets::{route_link_into_buf, ArrowBuf, put_char};
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::collections::HashSet;
use std::io;

pub fn draw(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    if matches!(&app.mode, Mode::ProjectList { .. }) {
        return draw_project_list(terminal, app);
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

        // ── Global arrow layer ────────────────────────────────────────────────
        // When Global is on, render every visible link in a dim colour first so
        // the graph structure is always visible behind the highlighted subset.
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
            global_buf.flush(frame, canvas, pal::tinted(pal::ARROW_DIM));
        }

        // ── Highlighted arrow layer ───────────────────────────────────────────
        // Draws the subset of arrows determined by incoming/outgoing fidelity.
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

            // Collect (src, tgt) pairs, deduplicated.
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
            arrow_buf.flush(frame, canvas, pal::tinted(pal::LINK_ARROW));
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
            prev.flush(frame, canvas, pal::tinted(pal::LINK));
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

            let is_reparent_subj = matches!(&app.mode, Mode::Reparent { subject, .. } if *subject == id);
            let is_reparent_cur  = matches!(&app.mode, Mode::Reparent { cursor,  .. } if *cursor  == id);
            let is_insert_parent = matches!(&app.mode,
                Mode::Input { action: InputAction::InsertChild { parent }, .. } if *parent == id);
            let is_pick_origin   = matches!(&app.mode,
                Mode::Canvas { state: CanvasState::Pick { origin_id, .. } } if *origin_id == id);
            let is_link_origin   = matches!(&app.mode,
                Mode::Canvas { state: CanvasState::Link { origin_id } } if *origin_id == id);

            let label_style = if is_reparent_subj || is_pick_origin {
                pal::solid(pal::PICK)
            } else if is_link_origin {
                pal::solid(pal::LINK)
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
            state: CanvasState::Pick { origin_id, origin_x, origin_y } } = app.mode
        {
            let ox_end = app.nodes[origin_id].world_x_end;
            let mut pick_buf = ArrowBuf::new();
            route_link_into_buf(
                &mut pick_buf, canvas, app.camera_x, app.camera_y,
                &visible_nodes,
                origin_x, origin_y, ox_end,
                app.cursor_x, app.cursor_y, app.cursor_x,
            );
            pick_buf.flush(frame, canvas, pal::tinted(pal::PICK));
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
        pal::solid(pal::LINK_ARROW)
    } else {
        pal::tinted(pal::LINK_ARROW)
    };
    let outgoing_style = if matches!(&app.mode, Mode::Canvas { state: CanvasState::MenuOutgoing }) {
        pal::solid(pal::LINK_ARROW)
    } else {
        pal::tinted(pal::LINK_ARROW)
    };

    let lines = vec![
        Line::from(vec![
            Span::raw(" [g] Global   "),
            Span::styled(tog(app.arrow.global), pal::tinted(pal::LINK_ARROW)),
        ]),
        Line::from(vec![
            Span::raw(" [i] Incoming "),
            Span::styled(fid(app.arrow.incoming), incoming_style),
        ]),
        Line::from(vec![
            Span::raw(" [o] Outgoing "),
            Span::styled(fid(app.arrow.outgoing), outgoing_style),
        ]),
        Line::from(Span::styled(hint, Style::default().fg(pal::DIM))),
    ];

    let w: u16 = 52;
    let h: u16 = lines.len() as u16 + 2;
    let x = canvas.x + canvas.width.saturating_sub(w + 1);
    let y = canvas.y;

    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(" Arrow Display ")
                .border_style(pal::tinted(pal::CANVAS))),
        Rect { x, y, width: w, height: h },
    );
}

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
        Mode::ProjectList { state: ProjectListState::Browse { selected } } => {
            if let Some(e) = app.registry.projects.get(*selected) {
                format!(" {} │ {}  │  Enter:open  n:new  d:remove  q:quit ", e.name, e.path)
            } else if let Some(msg) = &app.status_message {
                format!(" {} ", msg)
            } else {
                String::from(" no projects — press [n] to create one ")
            }
        }
        Mode::ProjectList { state: ProjectListState::NewPath { buf, .. } } =>
            format!(" new project path: \"{}\" ", buf),
        Mode::ProjectList { state: ProjectListState::NewName { path, buf, .. } } =>
            format!(" new project \"{}\" at {} ", buf, path),
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
    }
}

// ── Project list screen ───────────────────────────────────────────────────────

fn draw_project_list(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &App,
) -> io::Result<()> {
    terminal.draw(|frame| {
        let area = frame.area();

        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_style(titlebar::border_style(&app.mode))
                .title(titlebar::build_title(&app.mode)),
            area,
        );

        let inner = Rect {
            x:      area.x + 1,
            y:      area.y + 1,
            width:  area.width.saturating_sub(2),
            height: area.height.saturating_sub(3),
        };

        let selected_idx = match &app.mode {
            Mode::ProjectList { state: ProjectListState::Browse { selected } } => Some(*selected),
            _ => None,
        };

        if app.registry.projects.is_empty() {
            frame.render_widget(
                Paragraph::new("  No projects yet. Press [n] to create one.")
                    .style(Style::default().fg(pal::DIM)),
                Rect { x: inner.x, y: inner.y, width: inner.width, height: 1 },
            );
        } else {
            for (i, entry) in app.registry.projects.iter().enumerate() {
                if i as u16 >= inner.height.saturating_sub(1) { break; }
                let is_sel = selected_idx == Some(i);
                let (indicator, name_style) = if is_sel {
                    (">", pal::solid(pal::SELECTED))
                } else {
                    (" ", pal::tinted(pal::NODE))
                };
                frame.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::styled(format!(" {} ", indicator), name_style),
                        Span::styled(entry.name.clone(), name_style),
                        Span::styled(format!("  {}", entry.path), Style::default().fg(pal::DIM)),
                    ])),
                    Rect { x: inner.x, y: inner.y + i as u16, width: inner.width, height: 1 },
                );
            }
        }

        // Inline input prompt for new-project path / name.
        let prompt = match &app.mode {
            Mode::ProjectList { state: ProjectListState::NewPath { buf, cursor } } =>
                Some(("path: ", buf.as_str(), *cursor, pal::INSERT)),
            Mode::ProjectList { state: ProjectListState::NewName { buf, cursor, .. } } =>
                Some(("name: ", buf.as_str(), *cursor, pal::INSERT)),
            _ => None,
        };
        if let Some((prefix, buf, cursor, color)) = prompt {
            let sy = inner.height.saturating_sub(1);
            render_input_line(frame, inner, 0, sy, prefix, buf, cursor, color);
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
