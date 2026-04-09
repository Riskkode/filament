use ratatui::{
    layout::Rect,
    style::Style,
    text::Span,
    widgets::Paragraph,
    Frame,
};
use std::collections::HashMap;

pub fn put_char(frame: &mut Frame, canvas: Rect, sx: u16, sy: u16, ch: char, style: Style) {
    if sx >= canvas.width || sy >= canvas.height { return; }
    frame.render_widget(
        Paragraph::new(Span::styled(ch.to_string(), style)),
        Rect { x: canvas.x + sx, y: canvas.y + sy, width: 1, height: 1 },
    );
}

// ── Connection-set arrow buffer ───────────────────────────────────────────────
//
// Each cell tracks which cardinal directions it has line-segments going out in
// (bitmask) plus an optional "terminal" flag for lone arrowheads.  When two
// routes share a cell their direction bits are OR-merged; this naturally
// produces T-junctions (├ ┤ ┬ ┴) where paths diverge and crossroads (┼) at
// true intersections — no special-casing required.
//
// Direction bits:  N=0001  S=0010  E=0100  W=1000
//   Each bit means "this cell has a line-segment extending in that direction".
//
// Character table:
//   E|W      → ╌      N|S      → ╎
//   S|E      → ╭      S|W      → ╮
//   N|E      → ╰      N|W      → ╯
//   N|S|E    → ├      N|S|W    → ┤
//   E|W|S    → ┬      E|W|N    → ┴
//   N|S|E|W  → ┼
//   W only (terminal) → →      E only (terminal) → ←
//   N only (terminal) → ↓      S only (terminal) → ↑

type Conn = u8;
const CN: Conn = 0b0001;
const CS: Conn = 0b0010;
const CE: Conn = 0b0100;
const CW: Conn = 0b1000;

pub struct ArrowBuf {
    cells: HashMap<(u16, u16), (Conn, Option<Conn>)>,
}

impl ArrowBuf {
    pub fn new() -> Self { Self { cells: HashMap::new() } }

    /// Add a through-segment bit-pair (e.g. `CN|CS` for a vertical cell).
    pub fn add_seg(&mut self, x: u16, y: u16, bits: Conn) {
        let cell = self.cells.entry((x, y)).or_insert((0, None));
        cell.0 |= bits;
        // A lone arrowhead absorbed by a multi-direction cell becomes a junction.
        if cell.1.is_some() && cell.0.count_ones() > 1 { cell.1 = None; }
    }

    /// Add a terminal arrowhead.  `entry_bit` is the direction FROM which the
    /// path arrived (CW → came from the west → character is →).
    pub fn add_terminal(&mut self, x: u16, y: u16, entry_bit: Conn) {
        let cell = self.cells.entry((x, y)).or_insert((0, None));
        cell.0 |= entry_bit;
        if cell.0.count_ones() == 1 { cell.1 = Some(entry_bit); }
        else                        { cell.1 = None; }
    }

    fn char_for(conn: Conn, terminal: Option<Conn>) -> char {
        if terminal.is_some() {
            return match conn {
                c if c == CW => '→',
                c if c == CE => '←',
                c if c == CN => '↓',
                c if c == CS => '↑',
                _ => ' ',
            };
        }
        match conn {
            c if c == CE | CW           => '╌',
            c if c == CN | CS           => '╎',
            c if c == CS | CE           => '╭',
            c if c == CS | CW           => '╮',
            c if c == CN | CE           => '╰',
            c if c == CN | CW           => '╯',
            c if c == CN | CS | CE      => '├',
            c if c == CN | CS | CW      => '┤',
            c if c == CE | CW | CS      => '┬',
            c if c == CE | CW | CN      => '┴',
            c if c == CN | CS | CE | CW => '┼',
            _ => ' ',
        }
    }

    pub fn flush(&self, frame: &mut Frame, canvas: Rect, style: Style) {
        for (&(x, y), &(conn, term)) in &self.cells {
            let ch = Self::char_for(conn, term);
            if ch != ' ' { put_char(frame, canvas, x, y, ch, style); }
        }
    }

}

// ── Obstacle-aware vertical column search ─────────────────────────────────────
//
// Returns true when world column `cx` does not intersect any visible node's
// text span on the rows strictly between `y0` and `y1` (interior only; the
// endpoints are corners/arrowheads and are intentionally excluded).
fn col_clear_vert(
    nodes: &[(i32, i32, i32)],  // visible nodes: (world_x, world_y, world_x_end)
    cx: i32, y0: i32, y1: i32,
) -> bool {
    let lo = y0.min(y1);
    let hi = y0.max(y1);
    if lo + 1 >= hi { return true; } // no interior rows
    for &(wx, wy, wx_end) in nodes {
        if wy > lo && wy < hi && cx >= wx && cx <= wx_end {
            return false;
        }
    }
    true
}

/// Scan columns from `preferred` toward `limit`, returning the first that is
/// clear of node text in the vertical range [y0, y1].  Falls back to
/// `preferred` if nothing is found within the scan range.
fn find_clear_col(
    nodes: &[(i32, i32, i32)],
    preferred: i32, limit: i32,
    y0: i32, y1: i32,
) -> i32 {
    let step = if limit >= preferred { 1i32 } else { -1i32 };
    let mut cx = preferred;
    loop {
        if col_clear_vert(nodes, cx, y0, y1) { return cx; }
        if cx == limit { break; }
        cx += step;
    }
    preferred // fallback
}

// ── Manhattan link routing ────────────────────────────────────────────────────
//
// Selects exit/entry anchor sides based on the relative column positions of
// source and target, then routes with a Z-bend (two corners) so the arrowhead
// is always on a dedicated horizontal cell — never conflated with a corner.
//
// Vertical column selection is obstacle-aware: it scans the candidate range
// and picks a column that does not pass through any visible node's text.
//
// Z-shape (target right of source):
//   [Source text] ╌╌╌╮        corner 1  CW|CS / CW|CN
//                    ╎        vertical  (clear of text)
//                    ╰╌→      corner 2  CE|CN / CE|CS  +  arrowhead →
//
// Z-shape (target left of source):
//   ←╯╌╌ [Source text]        arrowhead ←  +  corner 2  CW|CN / CW|CS
//    ╎
//    ╭╌╌╌                     corner 1  CE|CS / CE|CN
//
// Overlapping columns → right-hook routing clear of both nodes.

pub fn route_link_into_buf(
    buf: &mut ArrowBuf,
    canvas: Rect,
    cam_x: i32, cam_y: i32,
    nodes: &[(i32, i32, i32)],   // visible nodes: (world_x, world_y, world_x_end)
    ox: i32, oy: i32, ox_end: i32,
    tx: i32, ty: i32, tx_end: i32,
) {
    if ox == tx && ox_end == tx_end && oy == ty { return; }

    macro_rules! sc {
        ($wx:expr, $wy:expr) => {{
            let sx = $wx - cam_x; let sy = $wy - cam_y;
            if sx < 0 || sy < 0 || sx as u16 >= canvas.width || sy as u16 >= canvas.height {
                None
            } else { Some((sx as u16, sy as u16)) }
        }};
    }
    macro_rules! seg {
        ($wx:expr, $wy:expr, $bits:expr) => {
            if let Some((sx, sy)) = sc!($wx, $wy) { buf.add_seg(sx, sy, $bits); }
        };
    }
    macro_rules! term {
        ($wx:expr, $wy:expr, $entry:expr) => {
            if let Some((sx, sy)) = sc!($wx, $wy) { buf.add_terminal(sx, sy, $entry); }
        };
    }

    // Emit the interior vertical segment between two rows at world column wx.
    // Endpoints (corners/arrowheads) are handled by the caller.
    let vert = |buf: &mut ArrowBuf, wx: i32, r0: i32, r1: i32| {
        let (lo, hi) = if r0 < r1 { (r0 + 1, r1) } else { (r1 + 1, r0) };
        for wy in lo..hi {
            if let Some((sx, sy)) = sc!(wx, wy) { buf.add_seg(sx, sy, CN | CS); }
        }
    };

    // ── Case D: column ranges overlap → right-hook ───────────────────────────
    //
    // Routes to the right of both nodes, then curves down/up and returns with
    // a ← arrowhead entering the target's right side.
    //
    //   [Source]╌╌╌╮        corner 1: CW|CS  or  CW|CN
    //              ╎        vertical  (hook pushed right until clear of text)
    //              ╯╌╌╌←   corner 2: CW|CN  or  CW|CS  +  arrowhead ←
    let col_overlap = tx <= ox_end && tx_end >= ox;
    if col_overlap {
        let base_hook = ox_end.max(tx_end) + 4;
        // Scan rightward until the vertical column is clear, cap at +30.
        let hook = (base_hook..=base_hook + 30)
            .find(|&h| col_clear_vert(nodes, h, oy, ty))
            .unwrap_or(base_hook);

        for wx in (ox_end + 1)..hook { seg!(wx, oy, CE | CW); }
        // Corner 1: incoming from west, outgoing south or north
        seg!(hook, oy, if ty > oy { CW | CS } else { CW | CN });
        vert(buf, hook, oy, ty);
        // Corner 2: incoming from north or south, outgoing west
        seg!(hook, ty, if ty > oy { CW | CN } else { CW | CS });
        for wx in (tx_end + 2)..hook { seg!(wx, ty, CE | CW); }
        term!(tx_end + 1, ty, CE);
        return;
    }

    // ── Case B: target strictly right (tx > ox_end) ──────────────────────────
    //
    // Z-shape:  exit right of source → vertical (clear of text) → enter left of target.
    //
    //   horiz  ox_end+1 .. cx-1      CE|CW
    //   corner1 (cx, oy)             CW|CS  or  CW|CN
    //   vertical cx interior         CN|CS
    //   corner2 (cx, ty)             CE|CN  or  CE|CS
    //   horiz  cx+1 .. tx-2          CE|CW     (may be empty when cx = tx-2)
    //   arrowhead at tx-1            entry CW → char →
    //
    // L-shape fallback when gap < 3 (corner and arrowhead share the same column).
    if tx > ox_end {
        let use_z  = tx - ox_end >= 3;
        // Preferred cx is as far right as possible (tx-2 for Z, tx-1 for L),
        // but scan leftward to find a column clear of intermediate node text.
        let (cx, arrowhead_wx) = if use_z {
            let min_cx = ox_end + 1;
            let pref   = tx - 2;
            let cx = find_clear_col(nodes, pref, min_cx, oy, ty);
            (cx, cx + 1) // arrowhead one step right of corner 2
        } else {
            (tx - 1, tx - 1) // L-shape: corner and arrowhead share column
        };

        // Horizontal from source right edge up to (not including) corner 1
        for wx in (ox_end + 1)..cx { seg!(wx, oy, CE | CW); }

        if oy == ty {
            // Same row — no vertical or second corner needed.
            for wx in cx..arrowhead_wx { seg!(wx, oy, CE | CW); }
            term!(arrowhead_wx, ty, CW);
            return;
        }

        // Corner 1: incoming from west, outgoing down/up
        seg!(cx, oy, if ty > oy { CW | CS } else { CW | CN });
        vert(buf, cx, oy, ty);

        if use_z {
            // Corner 2: incoming from vertical, outgoing east
            seg!(cx, ty, if ty > oy { CE | CN } else { CE | CS });
            // Short horizontal at target row from cx+1 to arrowhead_wx-1
            for wx in (cx + 1)..arrowhead_wx { seg!(wx, ty, CE | CW); }
            term!(arrowhead_wx, ty, CW);
        } else {
            term!(cx, ty, CW); // L-shape: arrowhead at corner column
        }
        return;
    }

    // ── Case C: target strictly left (tx_end < ox) ───────────────────────────
    //
    // Mirror of Case B.
    //
    //   horiz  cx+1 .. ox-1          CE|CW
    //   corner1 (cx, oy)             CE|CS  or  CE|CN
    //   vertical cx interior         CN|CS
    //   corner2 (cx, ty)             CW|CN  or  CW|CS
    //   horiz  arrowhead_wx+1 .. cx-1  CE|CW   (may be empty)
    //   arrowhead at tx_end+1        entry CE → char ←
    {
        let use_z  = ox - tx_end >= 3;
        let (cx, arrowhead_wx) = if use_z {
            let max_cx = ox - 1;
            let pref   = tx_end + 2;
            let cx = find_clear_col(nodes, pref, max_cx, oy, ty);
            (cx, cx - 1) // arrowhead one step left of corner 2
        } else {
            (tx_end + 1, tx_end + 1) // L-shape
        };

        // Horizontal from corner 1 column toward source left edge
        for wx in (cx + 1)..ox { seg!(wx, oy, CE | CW); }

        if oy == ty {
            for wx in (arrowhead_wx + 1)..=cx { seg!(wx, oy, CE | CW); }
            term!(arrowhead_wx, ty, CE);
            return;
        }

        // Corner 1: incoming from east, outgoing down/up
        seg!(cx, oy, if ty > oy { CE | CS } else { CE | CN });
        vert(buf, cx, oy, ty);

        if use_z {
            // Corner 2: incoming from vertical, outgoing west
            seg!(cx, ty, if ty > oy { CW | CN } else { CW | CS });
            for wx in (arrowhead_wx + 1)..cx { seg!(wx, ty, CE | CW); }
            term!(arrowhead_wx, ty, CE);
        } else {
            term!(cx, ty, CE); // L-shape
        }
    }
}

