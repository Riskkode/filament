use crate::app::{App, Mode, StatusPageState};
use crate::ui::palette::Palette;
use crate::ui::titlebar;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Rect, Layout, Constraint, Direction},
    style::{Color, Style, Modifier},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame, Terminal,
};
use std::io;

pub fn draw_status_page(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
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

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(4), Constraint::Min(0)])
            .split(inner);

        draw_statistics(frame, chunks[0], app);

        let panels_area = chunks[1];

        fn get_status_selection(mode: &Mode) -> (usize, Option<usize>) {
            match mode {
                Mode::StatusPage { state: StatusPageState::Browse { group_idx, item_idx } } => (*group_idx, *item_idx),
                Mode::TagStatus { previous } => get_status_selection(previous),
                Mode::TagTime { previous } => get_status_selection(previous),
                Mode::TagTimeClear { previous } => get_status_selection(previous),
                Mode::TimeInput { previous, .. } => get_status_selection(previous),
                _ => (0, None),
            }
        }

        let (focused_group, selected_item) = get_status_selection(&app.mode);

        if app.queries.is_empty() {
            let msg = "No groups defined yet. Press 'n' to add a new group.";
            frame.render_widget(
                Paragraph::new(msg).style(Style::default().fg(Color::DarkGray)),
                panels_area,
            );
        } else {
            let count = app.queries.len();
            let constraints: Vec<Constraint> = (0..count).map(|_| Constraint::Percentage(100 / count as u16)).collect();
            let group_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(constraints)
                .split(panels_area);

            for (i, query) in app.queries.iter().enumerate() {
                let results = app.get_query_results(i);
                let mut lines = Vec::new();
                
                // Calculate scroll offset for the focused column
                let scroll_offset = if i == focused_group {
                    if let Some(item_idx) = selected_item {
                        let h = group_chunks[i].height.saturating_sub(2) as usize;
                        if item_idx >= h { item_idx - h + 1 } else { 0 }
                    } else { 0 }
                } else { 0 };

                for (j, &node_id) in results.iter().enumerate().skip(scroll_offset) {
                    let node = &app.nodes[node_id];
                    let indicator = if i == focused_group && selected_item == Some(j) { "> " } else { "• " };
                    let style = if i == focused_group && selected_item == Some(j) {
                        app.palette.solid(app.palette.selected)
                    } else {
                        app.palette.tinted(app.palette.node)
                    };

                    lines.push(Line::from(vec![
                        Span::styled(indicator, Style::default().fg(app.palette.dim)),
                        Span::styled(node.label.clone(), style),
                    ]));
                }

                let border_style = if i == focused_group {
                    Style::default().fg(app.palette.edit)
                } else {
                    Style::default().fg(app.palette.dim)
                };

                frame.render_widget(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(" {} ", query.name))
                        .border_style(border_style),
                    group_chunks[i],
                );

                let group_inner = group_chunks[i].inner(ratatui::layout::Margin { horizontal: 1, vertical: 1 });
                frame.render_widget(Paragraph::new(lines), group_inner);
            }
        }

        // ── Status bar ────────────────────────────────────────────────────────
        let status = crate::ui::draw::build_status(app);
        frame.render_widget(
            Paragraph::new(status).style(Style::default().fg(Color::DarkGray)),
            Rect {
                x:      inner.x,
                y:      area.y + area.height.saturating_sub(2),
                width:  inner.width,
                height: 1,
            },
        );

        // ── Modal overlays ────────────────────────────────────────────────────
        if let Mode::ContextSwitcher { .. } = &app.mode {
            crate::ui::draw::draw_context_switcher(frame, area, app);
        }
        
        // Render text input overlays for query editing
        match &app.mode {
            Mode::StatusPage { state: StatusPageState::NewQueryName { buf, cursor } } => {
                draw_query_prompt(frame, inner, " name: ", buf, *cursor, &app.palette);
            }
            Mode::StatusPage { state: StatusPageState::NewQueryLogic { name: _, buf, cursor } } => {
                draw_query_prompt(frame, inner, " query: ", buf, *cursor, &app.palette);
            }
            _ => {}
        }
    })?;
    Ok(())
}

fn draw_query_prompt(frame: &mut Frame, area: Rect, prefix: &str, buf: &str, cursor: usize, pal: &Palette) {
    let width = 60u16;
    let height = 3u16;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect { x, y, width, height };

    let title = if prefix.contains("name") { " NEW GROUP NAME " } else { " QUERY LOGIC " };

    frame.render_widget(ratatui::widgets::Clear, popup_area);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(title)
            .border_style(Style::default().fg(pal.insert)),
        popup_area,
    );

    crate::ui::draw::render_input_line(
        frame,
        popup_area.inner(ratatui::layout::Margin { horizontal: 1, vertical: 1 }),
        0, 0,
        prefix, buf, cursor, pal, pal.insert
    );
}

fn draw_statistics(frame: &mut Frame, area: Rect, app: &App) {
    let total_nodes = app.nodes.len();
    let mut todo = 0;
    let mut in_progress = 0;
    let mut completed = 0;
    let mut blocked = 0;
    let mut total_duration = 0i64;

    for node in &app.nodes {
        match node.tags.get("status").map(|s| s.as_str()) {
            Some("todo") => todo += 1,
            Some("in_progress") => in_progress += 1,
            Some("completed") => completed += 1,
            Some("blocked") => blocked += 1,
            _ => {}
        }
        if let Some(tag) = node.times.get("duration") {
            total_duration += tag.timestamp;
        }
    }

    let stats_block = Block::default()
        .borders(Borders::ALL)
        .title(" Project Statistics ")
        .border_style(Style::default().fg(app.palette.prefix));
    
    let inner = stats_block.inner(area);
    frame.render_widget(stats_block, area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ])
        .split(inner);

    let render_stat = |frame: &mut Frame, area: Rect, label: &str, val: String, color: Color| {
        let text = vec![
            Line::from(Span::styled(label, Style::default().fg(app.palette.dim))),
            Line::from(Span::styled(val, Style::default().fg(color).add_modifier(Modifier::BOLD))),
        ];
        frame.render_widget(Paragraph::new(text), area);
    };

    render_stat(frame, chunks[0], "Total Nodes", total_nodes.to_string(), app.palette.node);
    render_stat(frame, chunks[1], "Statuses", format!("○ {}  ◑ {}  ● {}  ⊘ {}", todo, in_progress, completed, blocked), Color::White);
    render_stat(frame, chunks[2], "Total Effort", format_duration_stat(total_duration), app.palette.insert);
    
    // Deadlines: count within next 7 days
    let now = chrono::Local::now().timestamp();
    let week_later = now + 7 * 24 * 3600;
    let upcoming = app.nodes.iter()
        .filter(|n| n.times.get("deadline").map(|t| t.timestamp > now && t.timestamp < week_later).unwrap_or(false))
        .count();
    render_stat(frame, chunks[3], "Upcoming Deadlines", upcoming.to_string(), app.palette.pick);
    
    // Project Age
    let age_days = (chrono::Utc::now().timestamp() as u64).saturating_sub(app.project_created_at) / 86400;
    render_stat(frame, chunks[4], "Project Age", format!("{} days", age_days), app.palette.edit);
}

fn format_duration_stat(seconds: i64) -> String {
    let mut s = seconds.abs();
    let days = s / 86400;
    s %= 86400;
    let hours = s / 3600;
    s %= 3600;
    let minutes = s / 60;
    
    let mut parts = Vec::new();
    if days > 0 { parts.push(format!("{}d", days)); }
    if hours > 0 { parts.push(format!("{}h", hours)); }
    if minutes > 0 { parts.push(format!("{}m", minutes)); }
    
    if parts.is_empty() { "none".to_string() } else { parts.join(" ") }
}
