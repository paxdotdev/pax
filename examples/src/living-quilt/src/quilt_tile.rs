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
    pub progress: f64,
    pub x: f64,
    pub y: f64,
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
        let motif = seed as usize % 2;
        let orientation = (seed >> 5) as usize % 4;
        let direction = (seed >> 11) as usize % 4;
        let palette = (seed >> 18) as usize % COLORS.len();
        let inverted = (seed >> 25) & 1 == 1;
        let polygons = if motif == 0 {
            vec![
                vec![(0., 0.), (100., 0.), (100., 100.)],
                vec![(0., 0.), (100., 100.), (0., 100.)],
            ]
        } else {
            vec![
                vec![(0., 0.), (100., 0.), (50., 50.)],
                vec![(100., 0.), (100., 100.), (50., 50.)],
                vec![(100., 100.), (0., 100.), (50., 50.)],
                vec![(0., 100.), (0., 0.), (50., 50.)],
            ]
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
        let mut panel = Self {
            id,
            pieces,
            direction,
            started_at_ms: now,
            progress: if settled { 1.0 } else { 0.0 },
            x: 0.0,
            y: 0.0,
        };
        panel.position();
        panel
    }

    pub fn advance(&mut self, now: u64) {
        if self.progress >= 1.0 {
            return;
        }
        let t = (now.saturating_sub(self.started_at_ms) as f64 / PANEL_MS).clamp(0.0, 1.0);
        self.progress = 1.0 - (1.0 - t).powi(3);
        self.position();
    }

    fn position(&mut self) {
        let remaining = 100.0 * (1.0 - self.progress);
        (self.x, self.y) = match self.direction {
            0 => (0.0, -remaining),
            1 => (remaining, 0.0),
            2 => (0.0, remaining),
            _ => (-remaining, 0.0),
        };
    }
}

impl TileState {
    pub fn advance(&mut self, now: u64) {
        for panel in &mut self.panels {
            panel.advance(now);
        }
        // Newest is on top in Pax. Once one covers the tile, older panels are
        // invisible and can be retired without snapping an unfinished slide.
        if let Some(covered) = self.panels.iter().position(|p| p.progress >= 1.0) {
            self.panels.truncate(covered + 1);
        }
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
        let mut directions = std::collections::HashSet::new();
        for seed in 1..100 {
            let mut p = Panel::new(1, seed, 0, false);
            directions.insert(p.direction);
            assert_eq!(p.x.abs() + p.y.abs(), 100.0);
            p.advance(100);
            assert!(p.x.abs() + p.y.abs() > 0.0);
            p.advance(580);
            assert_eq!((p.x, p.y), (0.0, 0.0));
        }
        assert_eq!(directions.len(), 4);
    }
    #[test]
    fn new_arrival_does_not_restart_an_ongoing_slide() {
        let mut tile = TileState {
            id: 0,
            panels: vec![Panel::new(1, 1, 0, false), Panel::new(0, 2, 0, true)],
        };
        tile.advance(200);
        let progress = tile.panels[0].progress;
        tile.panels.insert(0, Panel::new(2, 3, 200, false));
        tile.advance(300);
        assert!(tile.panels[1].progress > progress);
        assert!(tile.panels[0].progress > 0.0);
        tile.advance(800);
        assert_eq!(tile.panels.len(), 1);
        assert_eq!(tile.panels[0].id, 2);
    }
}
