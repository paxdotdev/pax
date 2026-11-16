use crate::network::Network;
use std::collections::BTreeMap;

pub const MAP_SIZE: (f64, f64) = (3000., 1680.);

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn mix(self, b: Self, t: f64) -> Self {
        Self::new(self.x + (b.x - self.x) * t, self.y + (b.y - self.y) * t)
    }
    pub fn distance(self, b: Self) -> f64 {
        (self.x - b.x).hypot(self.y - b.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cubic {
    pub p: [Point; 4],
}
impl Cubic {
    pub fn straight(a: Point, b: Point) -> Self {
        Self {
            p: [a, a.mix(b, 1. / 3.), a.mix(b, 2. / 3.), b],
        }
    }
    pub fn at(&self, t: f64) -> Point {
        let a = self.p[0].mix(self.p[1], t);
        let b = self.p[1].mix(self.p[2], t);
        let c = self.p[2].mix(self.p[3], t);
        a.mix(b, t).mix(b.mix(c, t), t)
    }
    pub fn mix(&self, b: &Self, t: f64) -> Self {
        Self {
            p: std::array::from_fn(|i| self.p[i].mix(b.p[i], t)),
        }
    }
    pub fn translated(&self, dx: f64, dy: f64) -> Self {
        Self {
            p: self.p.map(|p| Point::new(p.x + dx, p.y + dy)),
        }
    }
}

// Four compatible cubic pieces per edge allow crisp schematic elbows to morph
// into smooth loops without changing topology or the renderer's path structure.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Curve {
    pub segments: [Cubic; 4],
}
impl Cubic {
    fn split(self) -> (Self, Self) {
        let a = self.p[0].mix(self.p[1], 0.5);
        let b = self.p[1].mix(self.p[2], 0.5);
        let c = self.p[2].mix(self.p[3], 0.5);
        let d = a.mix(b, 0.5);
        let e = b.mix(c, 0.5);
        let f = d.mix(e, 0.5);
        (
            Self {
                p: [self.p[0], a, d, f],
            },
            Self {
                p: [f, e, c, self.p[3]],
            },
        )
    }
}
impl Curve {
    pub fn from_cubic(c: Cubic) -> Self {
        let (a, b) = c.split();
        let (a, b) = (a.split(), b.split());
        Self {
            segments: [a.0, a.1, b.0, b.1],
        }
    }
    pub fn straight(a: Point, b: Point) -> Self {
        Self::from_cubic(Cubic::straight(a, b))
    }
    pub fn start(&self) -> Point {
        self.segments[0].p[0]
    }
    pub fn end(&self) -> Point {
        self.segments[3].p[3]
    }
    pub fn at(&self, t: f64) -> Point {
        let t = t.clamp(0., 1.) * 4.;
        let i = (t as usize).min(3);
        self.segments[i].at(t - i as f64)
    }
    pub fn mix(&self, b: &Self, t: f64) -> Self {
        Self {
            segments: std::array::from_fn(|i| self.segments[i].mix(&b.segments[i], t)),
        }
    }
    pub fn translated(&self, x: f64, y: f64) -> Self {
        Self {
            segments: self.segments.map(|c| c.translated(x, y)),
        }
    }
    fn elbow(a: Point, b: Point, diagonal_first: bool) -> Self {
        let points = octilinear(a, b, diagonal_first);
        if points.len() == 2 {
            return Self::straight(a, b);
        }
        let corner = points[1];
        let radius = 12_f64
            .min(a.distance(corner) * 0.25)
            .min(b.distance(corner) * 0.25);
        let enter = corner.mix(a, radius / corner.distance(a));
        let leave = corner.mix(b, radius / corner.distance(b));
        let middle = Cubic {
            p: [
                enter,
                enter.mix(corner, 2. / 3.),
                leave.mix(corner, 2. / 3.),
                leave,
            ],
        };
        let mut pieces = vec![Cubic::straight(a, enter), middle, Cubic::straight(leave, b)];
        let i = if a.distance(enter) > leave.distance(b) {
            0
        } else {
            2
        };
        let (x, y) = pieces[i].split();
        pieces.splice(i..=i, [x, y]);
        Self {
            segments: pieces.try_into().unwrap(),
        }
    }
}
fn octilinear(a: Point, b: Point, diagonal_first: bool) -> Vec<Point> {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let d = dx.abs().min(dy.abs());
    if d < 0.001 || (dx.abs() - dy.abs()).abs() < 0.001 {
        return vec![a, b];
    }
    let diagonal = Point::new(dx.signum() * d, dy.signum() * d);
    let elbow = if diagonal_first {
        Point::new(a.x + diagonal.x, a.y + diagonal.y)
    } else {
        Point::new(b.x - diagonal.x, b.y - diagonal.y)
    };
    vec![a, elbow, b]
}
fn along(points: &[Point], t: f64) -> Point {
    let lengths: Vec<_> = points.windows(2).map(|w| w[0].distance(w[1])).collect();
    let mut d = lengths.iter().sum::<f64>() * t;
    for (i, &length) in lengths.iter().enumerate() {
        if d <= length {
            return points[i].mix(points[i + 1], d / length.max(0.001));
        }
        d -= length;
    }
    *points.last().unwrap()
}

#[derive(Clone, Debug)]
pub struct SampledCurve {
    pub curve: Curve,
    pub lengths: [f64; 65],
}
impl SampledCurve {
    pub fn new(curve: Curve) -> Self {
        let mut lengths = [0.; 65];
        let mut last = curve.start();
        for (i, length) in lengths.iter_mut().enumerate().skip(1) {
            let p = curve.at(i as f64 / 64.);
            *length = last.distance(p);
            last = p;
        }
        for i in 1..65 {
            lengths[i] += lengths[i - 1];
        }
        Self { curve, lengths }
    }
    pub fn sample(&self, progress: f64) -> (Point, f64) {
        let target = progress.clamp(0., 1.) * self.lengths[64];
        let i = self.lengths.partition_point(|&l| l < target).clamp(1, 64);
        let t = ((i - 1) as f64
            + (target - self.lengths[i - 1])
                / (self.lengths[i] - self.lengths[i - 1]).max(0.00001))
            / 64.;
        let p = self.curve.at(t);
        let a = self.curve.at((t - 0.001).max(0.));
        let b = self.curve.at((t + 0.001).min(1.));
        (p, (b.y - a.y).atan2(b.x - a.x).to_degrees())
    }
    pub fn distance(&self, p: Point) -> f64 {
        (0..64)
            .map(|i| {
                let a = self.curve.at(i as f64 / 64.);
                let b = self.curve.at((i + 1) as f64 / 64.);
                let dx = b.x - a.x;
                let dy = b.y - a.y;
                let t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy).max(0.0001);
                p.distance(a.mix(b, t.clamp(0., 1.)))
            })
            .fold(f64::INFINITY, f64::min)
    }
}

pub fn map(network: &Network) -> Vec<Vec<Curve>> {
    let mut anchors: BTreeMap<&str, Point> = BTreeMap::new();
    for line in crate::map_data::ANCHORS
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let p: Vec<_> = line.split('|').collect();
        anchors.insert(
            p[0],
            Point::new(
                {
                    // Give the dense central interchanges extra horizontal room.
                    let x: f64 = p[1].parse().unwrap();
                    x + (x - 600.).clamp(0., 1200.) * 0.3
                },
                p[2].parse().unwrap(),
            ),
        );
    }
    let mut positions: Vec<Option<Point>> = network
        .stations
        .iter()
        .map(|s| anchors.get(s.name.as_str()).copied())
        .collect();
    // Authored interchange/end anchors establish the schematic. Intermediate
    // stations are distributed evenly, never inferred from train timing.
    for line in &network.lines {
        for chain in &line.chains {
            for window in chain.windows(chain.len()) {
                let known: Vec<_> = window
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &s)| positions[line.stops[s].station].map(|p| (i, p)))
                    .collect();
                for pair in known.windows(2) {
                    let (a, p) = pair[0];
                    let (b, q) = pair[1];
                    for i in a + 1..b {
                        let id = line.stops[window[i]].station;
                        if positions[id].is_none() {
                            positions[id] = Some(along(
                                &octilinear(p, q, true),
                                (i - a) as f64 / (b - a) as f64,
                            ));
                        }
                    }
                }
            }
        }
    }
    network
        .lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let points: Vec<_> = line
                .stops
                .iter()
                .enumerate()
                .map(|(j, stop)| {
                    let station = &network.stations[stop.station];
                    let p = positions[stop.station]
                        .unwrap_or_else(|| panic!("missing map anchor: {}", station.name));
                    // Badges form a compact row at interchanges. On shared horizontal
                    // corridors, separate the tracks perpendicular to travel instead.
                    let rank = station.lines.iter().position(|&l| l == index).unwrap() as f64;
                    let dx = (rank - (station.lines.len() - 1) as f64 / 2.) * 30.;
                    let (dx, dy) = if (index == 5 || index == 8) && j < 8 {
                        (0., if index == 5 { -20. } else { 20. })
                    } else if (index == 7 || index == 10) && j < 3 {
                        (0., if index == 7 { -20. } else { 20. })
                    } else {
                        (dx, 0.)
                    };
                    Point::new(p.x + dx, p.y + dy)
                })
                .collect();
            line.edges
                .iter()
                .map(|&(a, b)| Curve::elbow(points[a], points[b], true))
                .collect()
        })
        .collect()
}

fn capsule(t: f64, length: f64, radius: f64) -> (Point, Point) {
    let straight = length - 2. * radius;
    let perimeter = 2. * straight + 2. * std::f64::consts::PI * radius;
    let d = t.rem_euclid(1.) * perimeter;
    if d < straight {
        (Point::new(radius + d, 0.), Point::new(1., 0.))
    } else if d < straight + std::f64::consts::PI * radius {
        let a = (d - straight) / radius - std::f64::consts::FRAC_PI_2;
        (
            Point::new(
                length - radius + radius * a.cos(),
                radius + radius * a.sin(),
            ),
            Point::new(-a.sin(), a.cos()),
        )
    } else if d < 2. * straight + std::f64::consts::PI * radius {
        (
            Point::new(
                length - radius - (d - straight - std::f64::consts::PI * radius),
                2. * radius,
            ),
            Point::new(-1., 0.),
        )
    } else {
        let a = (d - 2. * straight - std::f64::consts::PI * radius) / radius
            + std::f64::consts::FRAC_PI_2;
        (
            Point::new(radius + radius * a.cos(), radius + radius * a.sin()),
            Point::new(-a.sin(), a.cos()),
        )
    }
}

pub fn isolated(network: &Network, line_id: usize, portrait: bool) -> (Vec<Curve>, (f64, f64)) {
    let line = &network.lines[line_id];
    let loop_count = match line.code {
        "JY" => 30,
        "E" => 28,
        _ => 0,
    };
    let length = if loop_count > 0 {
        loop_count as f64 * 70.
    } else {
        line.chains[0].len() as f64 * 105.
    };
    let mut points = vec![Point::default(); line.stops.len()];
    let mut tangents = points.clone();
    if loop_count > 0 {
        for i in 0..loop_count {
            let (p, t) = capsule(i as f64 / loop_count as f64, length, 160.);
            points[i] = Point::new(p.x + 140., p.y + 180.);
            tangents[i] = t;
        }
        if line.code == "E" {
            for i in 28..38 {
                points[i] = Point::new(points[27].x - 100. * (i - 27) as f64, 700.);
            }
        }
    } else {
        for (i, &s) in line.chains[0].iter().enumerate() {
            points[s] = Point::new(140. + i as f64 * 105., 300.);
        }
        if line.code == "M" {
            for (i, &s) in [27, 26, 25].iter().enumerate() {
                points[s] = Point::new(points[5].x - (i + 1) as f64 * 105., 580.);
            }
        }
    }
    // Keep a label gutter when the Oedo tail extends left of the loop.
    let min_x = points.iter().map(|p| p.x).fold(140., f64::min) - 140.;
    let transform = |p: Point| {
        let p = Point::new(p.x - min_x, p.y);
        if portrait {
            Point::new(p.y, p.x)
        } else {
            p
        }
    };
    let curves = line
        .edges
        .iter()
        .map(|&(a, b)| {
            let p = points[a];
            let q = points[b];
            let mut c = Cubic::straight(p, q);
            if loop_count > 0 && a < loop_count && b < loop_count {
                let d = p.distance(q) / 3.;
                c.p[1] = Point::new(p.x + tangents[a].x * d, p.y + tangents[a].y * d);
                c.p[2] = Point::new(q.x - tangents[b].x * d, q.y - tangents[b].y * d);
            }
            Curve::from_cubic(Cubic {
                p: c.p.map(transform),
            })
        })
        .collect();
    let size = (
        length + 280. - min_x,
        if line.code == "E" {
            860.
        } else if line.code == "M" {
            740.
        } else {
            680.
        },
    );
    (curves, if portrait { (size.1, size.0) } else { size })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overview_uses_octilinear_straights_and_nonzero_edges() {
        let n = Network::tokyo();
        for (line, edges) in map(&n).iter().enumerate() {
            for (edge, curve) in edges.iter().enumerate() {
                assert!(
                    SampledCurve::new(*curve).lengths[64] > 1.,
                    "{} edge {}",
                    n.lines[line].code,
                    edge
                );
                for c in &curve.segments {
                    let dx = c.p[3].x - c.p[0].x;
                    let dy = c.p[3].y - c.p[0].y;
                    let straight = c.p[1..3]
                        .iter()
                        .all(|p| ((p.x - c.p[0].x) * dy - (p.y - c.p[0].y) * dx).abs() < 0.001);
                    if straight {
                        assert!(
                            dx.abs() < 0.001
                                || dy.abs() < 0.001
                                || (dx.abs() - dy.abs()).abs() < 0.001
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn retargeting_keeps_current_geometry_and_arc_sampling_is_consistent() {
        let n = Network::tokyo();
        let full = map(&n);
        let (wide, _) = isolated(&n, 13, false);
        let (tall, _) = isolated(&n, 13, true);
        for i in 0..wide.len() {
            let current = full[13][i].mix(&wide[i], 0.41);
            let retarget = current.mix(&tall[i], 0.);
            for t in [0., 0.2, 0.5, 0.9, 1.] {
                assert!(current.at(t).distance(retarget.at(t)) < 0.00001);
            }
            let sampler = SampledCurve::new(current);
            for t in [0., 0.1, 0.5, 0.8, 1.] {
                let (p, angle) = sampler.sample(t);
                assert!(angle.is_finite());
                assert!(sampler.distance(p) < 0.5);
            }
        }
    }
    #[test]
    fn all_layouts_preserve_edges_station_junctions_and_loop_closure() {
        let n = Network::tokyo();
        let full = map(&n);
        for (i, line) in n.lines.iter().enumerate() {
            for portrait in [false, true] {
                let (isolated, size) = isolated(&n, i, portrait);
                assert_eq!(isolated.len(), full[i].len());
                for edge in &isolated {
                    for p in [edge.start(), edge.end()] {
                        assert!(p.x >= 100. && p.y >= 100.);
                        assert!(p.x <= size.0 - 100. && p.y <= size.1 - 100.);
                    }
                }
                for t in [0., 0.3, 0.7, 1.] {
                    let mut ends = BTreeMap::new();
                    for (e, &(a, b)) in line.edges.iter().enumerate() {
                        let curve = full[i][e].mix(&isolated[e], t);
                        for (id, p) in [(a, curve.start()), (b, curve.end())] {
                            if let Some(previous) = ends.insert(id, p) {
                                assert!(p.distance(previous) < 0.00001);
                            }
                        }
                        let sample = SampledCurve::new(curve);
                        assert!(sample.sample(0.).0.distance(curve.start()) < 0.001);
                        assert!(sample.sample(1.).0.distance(curve.end()) < 0.001);
                    }
                }
            }
        }
    }
}
