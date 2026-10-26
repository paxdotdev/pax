use crate::display::ROWS;
use pax_kit::*;

pub const INK_COLUMNS: usize = 56;
pub const MAX_INK_COLUMNS: usize = INK_COLUMNS * 32;
pub const INK_ROWS: usize = 21;
pub const BRUSH_WIDTH: f64 = 3.;
pub const MAX_POINTS: usize = 8192;

/// A point in the horizontal drawing strip. `start` lifts the pen before it.
#[pax]
pub struct InkPoint {
    pub x: f64,
    pub y: f64,
    pub start: bool,
}

pub fn drawing_width(points: &[InkPoint]) -> usize {
    let right = points
        .iter()
        .take(MAX_POINTS)
        .filter(|p| p.x.is_finite() && p.y.is_finite())
        .map(|p| p.x + BRUSH_WIDTH / 2.)
        .fold(0., f64::max);
    (right + 3.)
        .ceil()
        .clamp(INK_COLUMNS as f64, MAX_INK_COLUMNS as f64) as usize
}

pub fn canvas_width(points: &[InkPoint]) -> usize {
    let used = drawing_width(points);
    ((used + INK_COLUMNS * 2).div_ceil(INK_COLUMNS) * INK_COLUMNS)
        .clamp(INK_COLUMNS * 4, MAX_INK_COLUMNS)
}

/// Sample a round, three-cell-wide brush at LED centers, without partial coverage.
pub fn rasterize(points: &[InkPoint]) -> Vec<u32> {
    let width = drawing_width(points);
    let mut columns = vec![0; width];
    let mut previous: Option<&InkPoint> = None;
    let top = (ROWS - INK_ROWS) / 2;
    for p in points.iter().take(MAX_POINTS) {
        if !p.x.is_finite() || !p.y.is_finite() {
            previous = None;
            continue;
        }
        let from = previous.filter(|_| !p.start).unwrap_or(p);
        let radius = BRUSH_WIDTH / 2.;
        let left = ((from.x.min(p.x) - radius).floor().max(0.) as usize).min(width);
        let right = ((from.x.max(p.x) + radius).ceil().max(0.) as usize).min(width);
        let upper = ((from.y.min(p.y) - radius).floor().max(0.) as usize).min(INK_ROWS);
        let lower = ((from.y.max(p.y) + radius).ceil().max(0.) as usize).min(INK_ROWS);
        let dx = p.x - from.x;
        let dy = p.y - from.y;
        let length_squared = dx * dx + dy * dy;
        for (x, column) in columns.iter_mut().enumerate().take(right).skip(left) {
            for y in upper..lower {
                let cx = x as f64 + 0.5;
                let cy = y as f64 + 0.5;
                let t = if length_squared > 0. {
                    (((cx - from.x) * dx + (cy - from.y) * dy) / length_squared).clamp(0., 1.)
                } else {
                    0.
                };
                let ex = cx - (from.x + t * dx);
                let ey = cy - (from.y + t * dy);
                if ex * ex + ey * ey <= radius * radius {
                    *column |= 1 << (top + y);
                }
            }
        }
        previous = Some(p);
    }
    columns
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::GLYPH_SCALE;

    fn p(x: f64, y: f64, start: bool) -> InkPoint {
        InkPoint { x, y, start }
    }

    #[test]
    fn sparse_samples_form_a_continuous_three_cell_stroke() {
        let columns = rasterize(&[p(2.5, 10.5, true), p(53.5, 10.5, false)]);
        let expected = 0b111 << 13;
        assert!(columns[2..54].iter().all(|c| *c == expected));
        assert_eq!(BRUSH_WIDTH as usize, GLYPH_SCALE);
    }

    #[test]
    fn pen_lifts_do_not_connect_and_dots_are_visible() {
        let columns = rasterize(&[p(3.5, 10.5, true), p(50.5, 10.5, true)]);
        assert_ne!(columns[3], 0);
        assert_ne!(columns[50], 0);
        assert!(columns[6..48].iter().all(|c| *c == 0));
    }

    #[test]
    fn off_pad_segments_clip_without_lighting_the_margins() {
        let columns = rasterize(&[p(-50., 10.5, true), p(100., 10.5, false)]);
        assert!(columns[..100].iter().all(|c| *c == 0b111 << 13));
        let rows_mask = ((1u32 << INK_ROWS) - 1) << ((ROWS - INK_ROWS) / 2);
        let columns = rasterize(&[p(4., -50., true), p(4., 100., false)]);
        assert!(columns.iter().all(|c| c & !rows_mask == 0));
        assert_eq!(rasterize(&[]), vec![0; INK_COLUMNS]);
    }

    #[test]
    fn long_drawings_keep_their_scale_and_omit_unused_canvas() {
        let points = [p(3.5, 10.5, true), p(220.5, 10.5, false)];
        let columns = rasterize(&points);
        assert_eq!(columns.len(), 225);
        assert_eq!(columns[200], 0b111 << 13);
        assert!(canvas_width(&points) >= columns.len() + INK_COLUMNS);
        assert!(columns.len() < canvas_width(&points));
        assert_eq!(canvas_width(&[p(1e9, 0., true)]), MAX_INK_COLUMNS);
    }
}
