use crate::{
    geometry::{Curve, Point, MAP_SIZE},
    network::Network,
};

#[derive(Clone, Default)]
pub struct Label {
    pub offset: Point,
    pub width: f64,
    pub height: f64,
    pub text: String,
    pub major: bool,
}

pub fn is_major(name: &str) -> bool {
    matches!(
        name,
        "Tokyo"
            | "Shinjuku"
            | "Shibuya"
            | "Ikebukuro"
            | "Ueno"
            | "Ginza"
            | "Asakusa"
            | "Roppongi"
            | "Otemachi"
            | "Shimbashi"
            | "Shinagawa"
            | "Meguro"
            | "Kita-senju"
            | "Iidabashi"
            | "Oshiage"
    )
}

pub fn stop_positions(network: &Network, curves: &[Vec<Curve>]) -> Vec<Vec<Point>> {
    network
        .lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let mut positions = vec![Point::default(); line.stops.len()];
            for (edge, &(a, b)) in line.edges.iter().enumerate() {
                positions[a] = curves[i][edge].start();
                positions[b] = curves[i][edge].end();
            }
            positions
        })
        .collect()
}

#[derive(Clone, Copy)]
struct Rect(f64, f64, f64, f64);
impl Rect {
    fn overlap(self, b: Self) -> f64 {
        (self.2.min(b.2) - self.0.max(b.0)).max(0.) * (self.3.min(b.3) - self.1.max(b.1)).max(0.)
    }
}

// Lay out names once in schematic coordinates. Zoom scales this typography
// with the map instead of reflowing hundreds of native Text nodes per frame.
pub fn labels(network: &Network, curves: &[Vec<Curve>]) -> Vec<Label> {
    let positions = stop_positions(network, curves);
    let mut centers = vec![Point::default(); network.stations.len()];
    let mut owners = centers.clone();
    let mut badges = Vec::new();
    for (i, line) in network.lines.iter().enumerate() {
        for (j, stop) in line.stops.iter().enumerate() {
            let p = positions[i][j];
            centers[stop.station].x += p.x / network.stations[stop.station].lines.len() as f64;
            centers[stop.station].y += p.y / network.stations[stop.station].lines.len() as f64;
            if network.stations[stop.station].lines[0] == i {
                owners[stop.station] = p;
            }
            badges.push(Rect(p.x - 15., p.y - 20., p.x + 15., p.y + 20.));
        }
    }
    let mut order: Vec<_> = (0..network.stations.len()).collect();
    order.sort_by_key(|&id| {
        (
            !is_major(&network.stations[id].name),
            std::cmp::Reverse(network.stations[id].lines.len()),
            id,
        )
    });
    let mut result = vec![Label::default(); network.stations.len()];
    let mut placed = Vec::<(Rect, f64)>::new();
    let track_points: Vec<_> = curves
        .iter()
        .flatten()
        .flat_map(|c| (0..25).map(move |i| c.at(i as f64 / 24.)))
        .collect();
    for id in order {
        let station = &network.stations[id];
        let major = is_major(&station.name);
        let text = if major {
            station.name.clone()
        } else {
            if let Some((a, b)) = station.name.rsplit_once('-') {
                format!("{a}-\n{b}")
            } else {
                station.name.clone()
            }
        };
        let font = if major { 27. } else { 18. };
        let width = text.lines().map(|s| s.len()).max().unwrap_or(1) as f64 * font * 0.57 + 8.;
        let height = text.lines().count() as f64 * font * 1.25 + 4.;
        let p = centers[id];
        let span = station.lines.len() as f64 * 15. + 8.;
        // Above/below follows the printed-map convention. Alternate the first
        // side on dense horizontal runs; remaining candidates clear junctions.
        let above = Point::new(-width / 2., -27. - height);
        let below = Point::new(-width / 2., 27.);
        let candidates = [
            if id % 2 == 0 { above } else { below },
            if id % 2 == 0 { below } else { above },
            Point::new(span, -height / 2.),
            Point::new(-span - width, -height / 2.),
            Point::new(22., -27. - height),
            Point::new(-22. - width, -27. - height),
            Point::new(22., 27.),
            Point::new(-22. - width, 27.),
            Point::new(-width / 2., -58. - height),
            Point::new(-width / 2., 58.),
        ];
        let (offset, rect) = candidates
            .into_iter()
            .enumerate()
            .map(|(rank, d)| {
                let rect = Rect(p.x + d.x, p.y + d.y, p.x + d.x + width, p.y + d.y + height);
                let outside = rect.0 < 8.
                    || rect.1 < 8.
                    || rect.2 > MAP_SIZE.0 - 8.
                    || rect.3 > MAP_SIZE.1 - 8.;
                let score = placed
                    .iter()
                    .map(|(r, weight)| rect.overlap(*r) * weight)
                    .sum::<f64>()
                    + badges.iter().map(|r| rect.overlap(*r) * 8.).sum::<f64>()
                    + track_points
                        .iter()
                        .filter(|p| {
                            p.x > rect.0 - 4.
                                && p.x < rect.2 + 4.
                                && p.y > rect.1 - 4.
                                && p.y < rect.3 + 4.
                        })
                        .count() as f64
                        * 75.
                    + if outside { 1_000_000. } else { 0. }
                    + rank as f64 * 3.;
                (score, d, rect)
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, d, r)| (d, r))
            .unwrap();
        placed.push((rect, if major { 60. } else { 8. }));
        result[id] = Label {
            offset: Point::new(p.x + offset.x - owners[id].x, p.y + offset.y - owners[id].y),
            width,
            height,
            text,
            major,
        };
    }
    result
}
