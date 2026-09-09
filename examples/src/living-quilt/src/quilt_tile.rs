#[allow(unused_imports)] // Referenced by quilt_tile.pax.
use crate::quilt_panel::QuiltPanel;
use pax_kit::*;

pub const PANEL_MS: f64 = 580.0;

#[pax]
#[file("quilt_tile.pax")]
pub struct QuiltTile {
    pub panels: Property<Vec<Panel>>,
    pub colorized: Property<bool>,
}

#[pax]
#[custom(Defaults)]
pub struct TileState {
    pub id: usize,
    pub panels: Vec<Panel>,
}

#[pax]
#[custom(Defaults)]
pub struct Panel {
    pub id: usize,
    pub pieces: Vec<PanelPiece>,
    pub direction: usize,
    pub started_at_ms: u64,
    pub initially_settled: bool,
    pub pushes: Vec<PanelPush>,
}

#[pax]
#[custom(Defaults)]
pub struct PanelPush {
    pub direction: usize,
    pub started_at_ms: u64,
}

#[pax]
#[custom(Defaults)]
pub struct PanelPiece {
    pub id: usize,
    pub elements: Vec<PathElement>,
    pub gray: Color,
    pub color: Color,
}

impl Panel {
    pub fn new(id: usize, seed: u64, now: u64, settled: bool) -> Self {
        let seed = scramble(seed);
        let motif = seed as usize % 3;
        let orientation = (seed >> 5) as usize % 4;
        let palette = (seed >> 18) as usize % COLORS.len();
        let inverted = (seed >> 25) & 1 == 1;
        let polygons = match motif {
            0 => vec![
                vec![(0., 0.), (100., 0.), (100., 100.)],
                vec![(0., 0.), (100., 100.), (0., 100.)],
            ],
            1 => vec![
                vec![(0., 0.), (100., 0.), (50., 50.)],
                vec![(100., 0.), (100., 100.), (50., 50.)],
                vec![(100., 100.), (0., 100.), (50., 50.)],
                vec![(0., 100.), (0., 0.), (50., 50.)],
            ],
            // The other three quarters are one concave face, without internal seams.
            _ => vec![
                vec![(0., 0.), (100., 0.), (50., 50.)],
                vec![(0., 0.), (50., 50.), (100., 0.), (100., 100.), (0., 100.)],
            ],
        };
        let pieces = polygons
            .iter()
            .enumerate()
            .map(|(i, polygon)| {
                let role = if inverted { 3 - i } else { i };
                PanelPiece {
                    id: i,
                    elements: polygon_path(polygon, orientation),
                    gray: Color::from_hex(GRAYS[role]),
                    color: Color::from_hex(COLORS[palette][role]),
                }
            })
            .collect();
        Self {
            id,
            pieces,
            direction: 0,
            started_at_ms: now,
            initially_settled: settled,
            pushes: Vec::new(),
        }
    }

    pub fn progress(&self, now: u64) -> f64 {
        if self.initially_settled {
            1.0
        } else {
            slide_progress(self.started_at_ms, now)
        }
    }

    pub fn position(&self, now: u64) -> (f64, f64) {
        let remaining = 100.0 * (1.0 - self.progress(now));
        let (dx, dy) = direction_vector(self.direction);
        // The entering panel starts opposite the outward travel direction.
        let mut position = (-dx * remaining, -dy * remaining);
        for push in &self.pushes {
            let (dx, dy) = direction_vector(push.direction);
            let distance = slide_progress(push.started_at_ms, now) * 100.0;
            position.0 += dx * distance;
            position.1 += dy * distance;
        }
        position
    }
}

impl TileState {
    pub fn push_panel(&mut self, id: usize, seed: u64, direction: usize, now: u64) {
        let mut panel = Panel::new(id, seed, now, false);
        panel.direction = direction;
        // Record each new push once. Per-frame positions are owned by the
        // mounted QuiltPanel, so unchanged paths never join the motion signal.
        for older in &mut self.panels {
            older.pushes.push(PanelPush {
                direction,
                started_at_ms: now,
            });
        }
        self.panels.insert(0, panel);
        self.advance(now);
    }

    pub fn needs_retirement(&self, now: u64) -> bool {
        self.panels
            .iter()
            .take(self.panels.len().saturating_sub(1))
            .any(|p| p.progress(now) >= 1.0)
    }

    pub fn advance(&mut self, now: u64) {
        // Newest is on top in Pax. Once one covers the tile, older panels are
        // invisible and can be retired without snapping an unfinished slide.
        if let Some(covered) = self.panels.iter().position(|p| p.progress(now) >= 1.0) {
            self.panels.truncate(covered + 1);
        }
    }
}

fn slide_progress(started_at_ms: u64, now: u64) -> f64 {
    let t = (now.saturating_sub(started_at_ms) as f64 / PANEL_MS).clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Clockwise compass order, with positive y pointing down the screen.
pub fn direction_vector(direction: usize) -> (f64, f64) {
    match direction {
        0 => (0.0, -1.0),
        1 => (1.0, 0.0),
        2 => (0.0, 1.0),
        _ => (-1.0, 0.0),
    }
}

const GRAYS: [&str; 4] = ["EBEBEB", "191919", "797979", "B3B3B3"];
const COLORS: [[&str; 4]; 5] = [
    ["FE32BF", "1BEAF1", "FFB824", "6329FF"],
    ["F2F32B", "8145FF", "FF4C70", "1FF7CC"],
    ["40F3FF", "FF722E", "D532FF", "FDFFC9"],
    ["FF42DB", "95FF36", "532BFF", "FFD83D"],
    ["FFF028", "25C9FF", "FF3D68", "B742FF"],
];

fn scramble(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn polygon_path(points: &[(f64, f64)], orientation: usize) -> Vec<PathElement> {
    let mut elements = Vec::new();
    for (i, &(mut x, mut y)) in points.iter().enumerate() {
        for _ in 0..orientation {
            (x, y) = (100.0 - y, x);
        }
        if i > 0 {
            elements.push(PathElement::Line);
        }
        elements.push(PathElement::Point(
            Size::Percent(x.into()),
            Size::Percent(y.into()),
        ));
    }
    elements.push(PathElement::Close);
    elements
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_direction_moves_on_one_axis_and_settles_exactly() {
        for direction in 0..4 {
            let mut p = Panel::new(1, 1, 0, false);
            p.direction = direction;
            let (x, y) = p.position(0);
            assert_eq!(x.abs() + y.abs(), 100.0);
            let (x, y) = p.position(100);
            assert!(x.abs() + y.abs() > 0.0);
            assert_eq!(p.position(580), (0.0, 0.0));
        }
    }
    #[test]
    fn new_arrival_does_not_restart_an_ongoing_slide() {
        let mut tile = TileState {
            id: 0,
            panels: vec![Panel::new(0, 2, 0, true)],
        };
        tile.push_panel(1, 1, 1, 0);
        tile.advance(200);
        let progress = tile.panels[0].progress(200);
        let positions: Vec<_> = tile.panels.iter().map(|p| p.position(200)).collect();
        tile.push_panel(2, 3, 2, 200);
        assert_eq!(
            positions,
            tile.panels[1..]
                .iter()
                .map(|p| p.position(200))
                .collect::<Vec<_>>()
        );
        tile.advance(300);
        assert!(tile.panels[1].progress(300) > progress);
        assert!(tile.panels[0].progress(300) > 0.0);
        tile.advance(800);
        assert_eq!(tile.panels.len(), 1);
        assert_eq!(tile.panels[0].id, 2);
    }

    #[test]
    fn incoming_and_outgoing_panels_share_one_edge_and_velocity() {
        for direction in 0..4 {
            let mut tile = TileState {
                id: 0,
                panels: vec![Panel::new(0, 1, 0, true)],
            };
            tile.push_panel(1, 2, direction, 0);
            let (dx, dy) = direction_vector(direction);
            for now in [0, 80, 200, 400, 579] {
                tile.advance(now);
                let incoming = &tile.panels[0];
                let (ix, iy) = incoming.position(now);
                let (ox, oy) = tile.panels[1].position(now);
                assert!((ox - ix - dx * 100.0).abs() < 1e-8);
                assert!((oy - iy - dy * 100.0).abs() < 1e-8);
                assert!((ox - dx * incoming.progress(now) * 100.0).abs() < 1e-8);
                assert!((oy - dy * incoming.progress(now) * 100.0).abs() < 1e-8);
            }
        }
    }

    #[test]
    fn overlapping_pushes_from_every_direction_leave_no_holes() {
        let mut tile = TileState {
            id: 0,
            panels: vec![Panel::new(0, 1, 0, true)],
        };
        for (i, direction) in [1, 2, 3, 0, 2, 1].into_iter().enumerate() {
            tile.push_panel(i + 1, i as u64, direction, i as u64 * 90);
            for step in 0..9 {
                tile.advance(i as u64 * 90 + step * 10);
                for x in (0..100).step_by(5) {
                    for y in (0..100).step_by(5) {
                        assert!(tile.panels.iter().any(|p| {
                            let (px, py) = p.position(i as u64 * 90 + step * 10);
                            x as f64 + 0.5 >= px
                                && x as f64 + 0.5 <= px + 100.0
                                && y as f64 + 0.5 >= py
                                && y as f64 + 0.5 <= py + 100.0
                        }));
                    }
                }
            }
        }
        tile.advance(2000);
        assert_eq!(tile.panels.len(), 1);
        assert_eq!(tile.panels[0].position(2000), (0.0, 0.0));
    }

    #[test]
    fn panel_membership_changes_only_at_arrival_and_retirement() {
        let mut tile = TileState {
            id: 0,
            panels: vec![Panel::new(0, 1, 0, true)],
        };
        assert!(!tile.needs_retirement(100));
        tile.push_panel(1, 2, 1, 100);
        assert!(!tile.needs_retirement(679));
        assert!(tile.needs_retirement(680));
        tile.advance(680);
        assert_eq!(tile.panels.len(), 1);
        assert_eq!(tile.panels[0].id, 1);
        assert!(!tile.needs_retirement(1000));
    }

    #[test]
    fn alphabet_includes_a_single_quarter_against_a_unioned_three_quarters() {
        let mut found = false;
        for seed in 0..100 {
            let panel = Panel::new(0, seed, 0, true);
            if panel.pieces.len() == 2 && panel.pieces[1].elements.len() == 10 {
                let area = |piece: &PanelPiece| {
                    let points: Vec<_> = piece
                        .elements
                        .iter()
                        .filter_map(|e| match e {
                            PathElement::Point(x, y) => Some((
                                x.evaluate((100.0, 100.0), Axis::X),
                                y.evaluate((100.0, 100.0), Axis::Y),
                            )),
                            _ => None,
                        })
                        .collect();
                    points
                        .iter()
                        .zip(points.iter().cycle().skip(1))
                        .map(|(a, b)| a.0 * b.1 - a.1 * b.0)
                        .sum::<f64>()
                        .abs()
                        / 2.0
                };
                assert_eq!(area(&panel.pieces[0]), 2500.0);
                assert_eq!(area(&panel.pieces[1]), 7500.0);
                found = true;
            }
        }
        assert!(found);
    }
}
