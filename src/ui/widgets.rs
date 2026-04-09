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

/// Dashed link arrow with a directional arrowhead.
///
/// - Uses `╌`/`╎` (double-dash box-drawing) for the line segments.
/// - Terminates one column before the target node to avoid overlapping tree
///   formatting, with `→`/`←` pointing at the target.
/// - Same-column links (both nodes share `world_x`) loop out to the right
///   and approach the target from the right side.
pub fn draw_link_arrow(
    frame: &mut Frame, canvas: Rect,
    cam_x: i32, cam_y: i32,
    ox: i32, oy: i32,
    tx: i32, ty: i32,
    style: Style,
) {
    const HOOK: i32 = 16; // columns to the right for same-column routing

    macro_rules! put {
        ($wx:expr, $wy:expr, $ch:expr) => {{
            let sx = $wx - cam_x; let sy = $wy - cam_y;
            if sx >= 0 && sy >= 0 { put_char(frame, canvas, sx as u16, sy as u16, $ch, style); }
        }};
    }

    if ox == tx && oy == ty { return; }

    // ── Same-column: right hook ───────────────────────────────────────────────
    if ox == tx {
        let turn = ox + HOOK;
        for x in (ox + 1)..turn { put!(x, oy, '╌'); }
        put!(turn, oy, if ty > oy { '╮' } else { '╯' });
        let v_lo = oy.min(ty) + 1;
        let v_hi = oy.max(ty);
        for y in v_lo..v_hi { put!(turn, y, '╎'); }
        put!(turn, ty, if ty > oy { '╯' } else { '╮' });
        for x in (ox + 2)..turn { put!(x, ty, '╌'); }
        put!(ox + 1, ty, '←');
        return;
    }

    // ── Different columns: offset-elbow ───────────────────────────────────────
    // Corner x lands one column before the target; approach arrow points inward.
    let (cx, arrow) = if ox < tx { (tx - 1, '→') } else { (tx + 1, '←') };
    if cx == ox { return; } // source and target are adjacent — nothing to draw

    // Horizontal segment: one column past source up to (but not including) corner.
    let (h_lo, h_hi) = if ox < cx { (ox + 1, cx) } else { (cx + 1, ox) };
    for x in h_lo..h_hi { put!(x, oy, '╌'); }

    if oy == ty {
        // Purely horizontal — just place the arrowhead at the corner.
        put!(cx, oy, arrow);
        return;
    }

    // Elbow: corner → vertical leg → arrowhead.
    let corner = match (ox < tx, ty > oy) {
        (true,  true)  => '╮',
        (true,  false) => '╯',
        (false, true)  => '╭',
        (false, false) => '╰',
    };
    put!(cx, oy, corner);
    let v_lo = oy.min(ty) + 1;
    let v_hi = oy.max(ty);
    for y in v_lo..v_hi { put!(cx, y, '╎'); }
    put!(cx, ty, arrow);
}
