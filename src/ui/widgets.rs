use ratatui::{
    layout::Rect,
    style::Style,
    text::Span,
    widgets::Paragraph,
    Frame,
};

pub fn put_char(frame: &mut Frame, canvas: Rect, sx: u16, sy: u16, ch: char, style: Style) {
    if sx >= canvas.width || sy >= canvas.height { return; }
    frame.render_widget(
        Paragraph::new(Span::styled(ch.to_string(), style)),
        Rect { x: canvas.x + sx, y: canvas.y + sy, width: 1, height: 1 },
    );
}

pub fn draw_elbow_arrow(
    frame: &mut Frame, canvas: Rect,
    cam_x: i32, cam_y: i32,
    ox: i32, oy: i32, cx: i32, cy: i32,
    style: Style,
) {
    macro_rules! put {
        ($wx:expr, $wy:expr, $ch:expr) => {{
            let sx = $wx - cam_x; let sy = $wy - cam_y;
            if sx >= 0 && sy >= 0 { put_char(frame, canvas, sx as u16, sy as u16, $ch, style); }
        }};
    }
    if ox == cx && oy == cy { return; }
    if oy == cy { for x in ox.min(cx)..=ox.max(cx) { put!(x, oy, '─'); } return; }
    if ox == cx { for y in oy.min(cy)..=oy.max(cy) { put!(ox, y, '│'); } return; }

    if ox < cx { for x in ox..cx { put!(x, oy, '─'); } }
    else        { for x in (cx + 1)..=ox { put!(x, oy, '─'); } }

    let corner = match (ox < cx, cy > oy) {
        (true,  true)  => '╮',
        (true,  false) => '╯',
        (false, true)  => '╭',
        (false, false) => '╰',
    };
    put!(cx, oy, corner);
    let v_lo = oy.min(cy) + 1;
    let v_hi = oy.max(cy);
    for y in v_lo..v_hi { put!(cx, y, '│'); }
}
