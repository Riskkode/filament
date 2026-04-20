use crate::app::{App, Mode, ArchiveState};
use crate::ui::titlebar;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Rect, Layout, Constraint, Direction},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, List, ListItem},
    Terminal,
};
use std::io;

pub fn draw_archive_page(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
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

        let inner = Rect {
            x:      area.x + 1,
            y:      area.y + 1,
            width:  area.width.saturating_sub(2),
            height: area.height.saturating_sub(3),
        };

        // Split into Left (List) and Right (Editor)
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(70),
            ])
            .split(inner);

        let left_area = chunks[0];
        let right_area = chunks[1];

        // ── Left: Note List ──────────────────────────────────────────────────
        let (selected_idx, is_editing_title, is_editing_content) = match &app.mode {
            Mode::ArchivePage { state, .. } => match state {
                ArchiveState::BrowseList { selected } => (*selected, false, false),
                ArchiveState::EditTitle { idx, .. } => (*idx, true, false),
                ArchiveState::EditContent { idx, .. } => (*idx, false, true),
            },
            _ => (0, false, false),
        };

        let items: Vec<ListItem> = app.notes.iter().enumerate().map(|(i, note)| {
            let style = if i == selected_idx {
                app.palette.solid(app.palette.selected)
            } else {
                app.palette.tinted(app.palette.node)
            };
            let prefix = if i == selected_idx { "> " } else { "  " };
            ListItem::new(Span::styled(format!("{}{}", prefix, note.title), style))
        }).collect();

        let list = List::new(items)
            .block(Block::default()
                .borders(Borders::RIGHT)
                .border_style(Style::default().fg(app.palette.dim))
                .title(" Notes "));
        frame.render_widget(list, left_area);

        // ── Right: Editor ────────────────────────────────────────────────────
        if let Some(note) = app.notes.get(selected_idx) {
            // Further split right area for Title, Content, and Footer
            let editor_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Title
                    Constraint::Min(0),    // Content
                    Constraint::Length(3), // Footer (Referenced nodes)
                ])
                .split(right_area);

            let title_area = editor_chunks[0];
            let content_area = editor_chunks[1];
            let footer_area = editor_chunks[2];

            // Title
            let title_text = if is_editing_title {
                if let Mode::ArchivePage { state: ArchiveState::EditTitle { buf, cursor, .. }, .. } = &app.mode {
                    render_editable_line(buf, *cursor, &app.palette, app.palette.edit)
                } else {
                    Line::from(note.title.as_str())
                }
            } else {
                Line::from(note.title.as_str())
            };

            frame.render_widget(
                Paragraph::new(title_text)
                    .block(Block::default()
                        .borders(Borders::BOTTOM)
                        .border_style(Style::default().fg(app.palette.dim))
                        .title(" Title ")),
                title_area,
            );

            // Content
            let content_lines = if is_editing_content {
                if let Mode::ArchivePage { state: ArchiveState::EditContent { buf, cursor, .. }, .. } = &app.mode {
                    render_editable_content(buf, *cursor, &app.palette, app.palette.edit)
                } else {
                    note.content.lines().map(Line::from).collect()
                }
            } else {
                note.content.lines().map(Line::from).collect()
            };

            frame.render_widget(
                Paragraph::new(content_lines)
                    .block(Block::default()
                        .title(" Content ")),
                content_area,
            );

            // Footer (Referenced nodes)
            let refs = find_references(&note.content);
            let mut ref_spans = vec![Span::raw("References: ")];
            if refs.is_empty() {
                ref_spans.push(Span::styled("none", Style::default().fg(app.palette.dim)));
            } else {
                for (i, r) in refs.iter().enumerate() {
                    if i > 0 { ref_spans.push(Span::raw(", ")); }
                    let node_exists = app.nodes.iter().any(|n| n.label == *r);
                    let style = if node_exists {
                        app.palette.solid(app.palette.link)
                    } else {
                        Style::default().fg(app.palette.status_blocked)
                    };
                    ref_spans.push(Span::styled(format!("@{}", r), style));
                }
            }

            frame.render_widget(
                Paragraph::new(Line::from(ref_spans))
                    .block(Block::default()
                        .borders(Borders::TOP)
                        .border_style(Style::default().fg(app.palette.dim))),
                footer_area,
            );
        } else {
             frame.render_widget(
                Paragraph::new("No note selected")
                    .block(Block::default().title(" Editor ")),
                right_area,
            );
        }

        // Modal overlays
        if let Mode::ContextSwitcher { .. } = &app.mode {
            crate::ui::draw::draw_context_switcher(frame, area, app);
        }

        // Status bar
        let status = crate::ui::draw::build_status(app);
        frame.render_widget(
            Paragraph::new(status).style(Style::default().fg(Color::DarkGray)),
            Rect { x: inner.x, y: area.y + area.height.saturating_sub(2), width: inner.width, height: 1 },
        );
    })?;
    Ok(())
}

fn render_editable_line<'a>(buf: &'a str, cursor: usize, pal: &crate::ui::palette::Palette, color: Color) -> Line<'a> {
    let before = &buf[..cursor];
    let cur_char = buf[cursor..].chars().next().unwrap_or(' ');
    let after_off = cursor + cur_char.len_utf8();
    let after = if after_off <= buf.len() { &buf[after_off..] } else { "" };

    Line::from(vec![
        Span::styled(before.to_string(), pal.tinted(color)),
        Span::styled(cur_char.to_string(), pal.solid(color)),
        Span::styled(after.to_string(), pal.tinted(color)),
    ])
}

fn render_editable_content<'a>(buf: &'a str, cursor: usize, pal: &crate::ui::palette::Palette, color: Color) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    let mut current_pos = 0;
    
    for line_content in buf.split_inclusive('\n') {
        let line_len = line_content.len();
        if cursor >= current_pos && cursor < current_pos + line_len {
            // Cursor is in this line
            let local_cursor = cursor - current_pos;
            let before = &line_content[..local_cursor];
            
            // Check if cursor is at the newline character itself
            if local_cursor < line_content.len() {
                let cur_char = line_content[local_cursor..].chars().next().unwrap_or(' ');
                let after_off = local_cursor + cur_char.len_utf8();
                let after = if after_off <= line_content.len() { &line_content[after_off..] } else { "" };
                
                lines.push(Line::from(vec![
                    Span::styled(before.to_string(), pal.tinted(color)),
                    Span::styled(cur_char.to_string(), pal.solid(color)),
                    Span::styled(after.to_string(), pal.tinted(color)),
                ]));
            } else {
                // Should not happen with split_inclusive unless cursor is at end of buf
                lines.push(Line::from(vec![
                    Span::styled(before.to_string(), pal.tinted(color)),
                    Span::styled(" ", pal.solid(color)),
                ]));
            }
        } else if cursor == current_pos + line_len && !line_content.ends_with('\n') {
             // Cursor is at the very end of the buffer (no trailing newline)
            lines.push(Line::from(vec![
                Span::styled(line_content.to_string(), pal.tinted(color)),
                Span::styled(" ", pal.solid(color)),
            ]));
        } else {
            lines.push(Line::from(vec![Span::styled(line_content.to_string(), pal.tinted(color))]));
        }
        current_pos += line_len;
    }
    
    if cursor == buf.len() && (buf.is_empty() || buf.ends_with('\n')) {
        lines.push(Line::from(vec![Span::styled(" ", pal.solid(color))]));
    }

    lines
}

fn find_references(content: &str) -> Vec<String> {
    let mut refs = Vec::new();
    for word in content.split_whitespace() {
        if word.starts_with('@') && word.len() > 1 {
            let name = word[1..].trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != ' ');
            if !name.is_empty() {
                refs.push(name.to_string());
            }
        }
    }
    refs
}
