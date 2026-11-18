// Demo-local geometry, rebuilt only when the board changes layout breakpoint.
pub type Point = (f64, f64);

pub struct Chip {
    pub center: Point,
    pub size: Point,
    pub label: &'static str,
    pub reference: &'static str,
    pub pins_per_side: usize,
}

impl Chip {
    fn pins(&self, side: f64, indices: &[usize]) -> Port {
        let pins = indices
            .iter()
            .map(|&i| {
                (
                    self.center.0 + side * (self.size.0 / 2.0 + 18.0),
                    self.center.1 - self.size.1 / 2.0
                        + 18.0
                        + i as f64 * (self.size.1 - 36.0) / (self.pins_per_side - 1) as f64,
                )
            })
            .collect();
        Port {
            pins,
            outward: (side, 0.0),
        }
    }
}

struct Port {
    pins: Vec<Point>,
    outward: Point,
}

impl Port {
    fn entry(&self) -> Point {
        let center = mul(
            self.pins.iter().copied().fold((0.0, 0.0), add),
            1.0 / self.pins.len() as f64,
        );
        let spread = self
            .pins
            .iter()
            .map(|&p| dot(sub(p, center), normal(self.outward)).abs())
            .fold(0.0, f64::max);
        add(center, mul(self.outward, spread + 22.0))
    }
}

pub fn dimensions(compact: bool) -> Point {
    if compact {
        (660.0, 580.0)
    } else {
        (700.0, 580.0)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Wiring {
    pub ethernet: [usize; 3],
    pub destination: Vec<usize>,
    pub outputs: Vec<Vec<usize>>,
}

pub fn build(compact: bool) -> (Vec<Chip>, Vec<Vec<Point>>, Wiring) {
    let (left, right) = if compact {
        (200.0, 470.0)
    } else {
        (220.0, 510.0)
    };
    // CircuitPin folds above its For to reclaim the last row while keeping
    // the two columns and the other parent-child relationships easy to follow.
    // Bottom-left pad lies on the normal through the adjacent copper bend.
    let pin_y = if compact { 347.75 } else { 337.75 };
    let positions = [
        (left, 100.0, 180.0, 82.0),
        (left, 215.0, 96.0, 82.0),
        (left, 350.0, 200.0, 128.0),
        (left, 495.0, 110.0, 82.0),
        (right, 495.0, 96.0, 82.0),
        (right, pin_y, 166.0, 82.0),
    ];
    let chips: Vec<_> = positions
        .into_iter()
        .zip([
            ("CircuitBoard", "U1 · ROOT", 3),
            ("For", "U2 · CHIPS", 3),
            ("CircuitPackage", "U3 · INSTANCE", 5),
            ("Text", "U4 · LABEL", 3),
            ("For", "U5 · PINS", 3),
            ("CircuitPin", "U6 · INSTANCE", 3),
        ])
        .map(|((x, y, w, h), (label, reference, pins_per_side))| Chip {
            center: (x, y),
            size: (w, h),
            label,
            reference,
            pins_per_side,
        })
        .collect();

    // Incoming and outgoing pin banks are disjoint, including the package's
    // left-hand input and left child. Unary bundles run beside their column,
    // leaving only the branch to the pin subtree to cross between columns.
    let edges = [
        (0, 1.0, vec![0, 1, 2], 1, 1.0, vec![0, 1, 2]),
        (1, -1.0, vec![0, 1, 2], 2, -1.0, vec![0, 1, 2]),
        (2, -1.0, vec![3, 4], 3, -1.0, vec![0, 2]),
        (2, 1.0, vec![3, 4], 4, -1.0, vec![0, 2]),
        (4, 1.0, vec![0, 1, 2], 5, 1.0, vec![0, 1, 2]),
    ];
    let mut routes = Vec::new();
    let mut wiring = Wiring {
        outputs: vec![Vec::new(); chips.len()],
        ..Default::default()
    };
    for (edge, (parent, side_a, indices_a, child, side_b, indices_b)) in
        edges.into_iter().enumerate()
    {
        let a = chips[parent].pins(side_a, &indices_a);
        let b = chips[child].pins(side_b, &indices_b);
        let start = a.entry();
        let end = b.entry();
        let middle = match edge {
            0 | 1 | 2 | 4 => {
                let lane = if side_a < 0.0 {
                    start.0.min(end.0) - 16.0
                } else {
                    start.0.max(end.0) + 16.0
                };
                vec![(lane, start.1), (lane, end.1)]
            }
            3 => {
                let lane = (start.0 + end.0) / 2.0;
                vec![(lane, start.1), (lane, end.1)]
            }
            _ => unreachable!(),
        };
        let mut centerline = vec![start];
        centerline.extend(middle);
        centerline.push(end);
        for copper in bundle(a, b, &chamfer(centerline)) {
            wiring.outputs[parent].push(routes.len());
            wiring.destination.push(child);
            routes.push(copper);
        }
    }
    // The connector's outer pin bank faces the root's inputs directly. Matching
    // pad pitch leaves three short parallel runs without unnecessary fan-outs.
    for (i, pad) in chips[0].pins(-1.0, &[0, 1, 2]).pins.into_iter().enumerate() {
        wiring.ethernet[i] = routes.len();
        wiring.destination.push(0);
        routes.push(vec![(44.0, 77.0 + i as f64 * 23.0), pad]);
    }
    (chips, routes, wiring)
}

fn add(a: Point, b: Point) -> Point {
    (a.0 + b.0, a.1 + b.1)
}
fn sub(a: Point, b: Point) -> Point {
    (a.0 - b.0, a.1 - b.1)
}
fn mul(a: Point, s: f64) -> Point {
    (a.0 * s, a.1 * s)
}
fn dot(a: Point, b: Point) -> f64 {
    a.0 * b.0 + a.1 * b.1
}
fn normal(a: Point) -> Point {
    (-a.1, a.0)
}
fn unit(a: Point) -> Point {
    mul(a, 1.0 / a.0.hypot(a.1))
}

fn chamfer(mut points: Vec<Point>) -> Vec<Point> {
    points.dedup();
    let mut result = vec![points[0]];
    for p in points.windows(3) {
        let a = sub(p[0], p[1]);
        let b = sub(p[2], p[1]);
        let cut = 24.0_f64
            .min(a.0.hypot(a.1) * 0.45)
            .min(b.0.hypot(b.1) * 0.45);
        result.push(add(p[1], mul(unit(a), cut)));
        result.push(add(p[1], mul(unit(b), cut)));
    }
    result.push(*points.last().unwrap());
    result
}

fn offset(points: &[Point], distance: f64) -> Vec<Point> {
    let normals: Vec<_> = points
        .windows(2)
        .map(|p| normal(unit(sub(p[1], p[0]))))
        .collect();
    points
        .iter()
        .enumerate()
        .map(|(i, &p)| {
            let shift = if i == 0 {
                normals[0]
            } else if i == points.len() - 1 {
                normals[i - 1]
            } else {
                mul(
                    add(normals[i - 1], normals[i]),
                    1.0 / (1.0 + dot(normals[i - 1], normals[i])),
                )
            };
            add(p, mul(shift, distance))
        })
        .collect()
}

fn fan(pin: Point, join: Point, outward: Point) -> Vec<Point> {
    let delta = sub(join, pin);
    let lateral = dot(delta, normal(outward)).abs();
    let lead = dot(delta, outward) - lateral - 8.0;
    assert!(lead >= 0.0, "trace fan must fit its pad clearance");
    vec![
        pin,
        add(pin, mul(outward, lead)),
        sub(join, mul(outward, 8.0)),
        join,
    ]
}

fn bundle(mut a: Port, mut b: Port, centerline: &[Point]) -> Vec<Vec<Point>> {
    let start_normal = normal(a.outward);
    let end_normal = normal(mul(b.outward, -1.0));
    a.pins
        .sort_by(|&x, &y| dot(x, start_normal).total_cmp(&dot(y, start_normal)));
    b.pins
        .sort_by(|&x, &y| dot(x, end_normal).total_cmp(&dot(y, end_normal)));
    let count = a.pins.len();
    a.pins
        .iter()
        .zip(&b.pins)
        .enumerate()
        .map(|(i, (&from, &to))| {
            // Perpendicular spacing stays constant through diagonal runs and bends.
            let lane = offset(centerline, (i as f64 - (count - 1) as f64 / 2.0) * 4.0);
            let mut points = fan(from, lane[0], a.outward);
            points.extend(lane.iter().skip(1));
            points.extend(
                fan(to, *lane.last().unwrap(), b.outward)
                    .into_iter()
                    .rev()
                    .skip(1),
            );
            points.dedup();
            points
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layouts_keep_traces_octilinear_and_on_board() {
        for compact in [false, true] {
            let (_, routes, _) = build(compact);
            let (w, h) = dimensions(compact);
            assert_eq!(routes.len(), 16);
            for route in routes {
                for &(x, y) in &route {
                    assert!(x > 8.0 && x < w - 8.0 && y > 8.0 && y < h - 8.0, "{x},{y}");
                }
                for p in route.windows(2) {
                    let (x, y) = sub(p[1], p[0]);
                    assert!(
                        x.abs() < 1e-7 || y.abs() < 1e-7 || (x.abs() - y.abs()).abs() < 1e-7,
                        "non-octilinear segment {p:?}"
                    );
                }
            }
        }
    }
    #[test]
    fn nested_bends_preserve_perpendicular_lane_pitch() {
        let center = chamfer(vec![
            (0.0, 0.0),
            (100.0, 0.0),
            (100.0, 100.0),
            (200.0, 100.0),
        ]);
        let lane = offset(&center, 4.0);
        for (a, b) in center.windows(2).zip(lane.windows(2)) {
            let n = normal(unit(sub(a[1], a[0])));
            for i in 0..2 {
                assert!((dot(sub(b[i], a[i]), n) - 4.0).abs() < 1e-8);
            }
        }
    }

    #[test]
    fn two_columns_fold_the_pin_child_above_its_parent() {
        for compact in [false, true] {
            let (chips, routes, _) = build(compact);
            for (parent, child) in [(0, 1), (1, 2), (2, 3), (2, 4)] {
                assert!(chips[child].center.1 > chips[parent].center.1);
            }
            assert_eq!(chips[3].center.1, chips[4].center.1);
            let bend = routes[9].windows(2).find(|p| {
                let d = sub(p[1], p[0]);
                d.0 > 0.0 && d.1 > 0.0
            }).unwrap();
            let midpoint = mul(add(bend[0], bend[1]), 0.5);
            let pin = chips[5].pins(-1.0, &[2]).pins[0];
            assert!(dot(sub(pin, midpoint), unit(sub(bend[1], bend[0]))).abs() < 1e-8);
            assert!(chips[5].center.1 < chips[4].center.1);
            for (parent, child) in [(0, 1), (1, 2), (4, 5)] {
                assert_eq!(chips[parent].center.0, chips[child].center.0);
            }
            assert_eq!(chips[2].center.0, chips[3].center.0);
            assert!(chips[4].center.0 > chips[3].center.0);
        }
    }

    #[test]
    fn copper_clears_package_bodies() {
        for compact in [false, true] {
            let (chips, routes, _) = build(compact);
            for route in routes {
                for segment in route.windows(2) {
                    // Clip the segment against each expanded package rectangle.
                    for chip in &chips {
                        let min = sub(chip.center, add(mul(chip.size, 0.5), (3.0, 3.0)));
                        let max = add(chip.center, add(mul(chip.size, 0.5), (3.0, 3.0)));
                        let delta = sub(segment[1], segment[0]);
                        let mut enter = 0.0_f64;
                        let mut exit = 1.0_f64;
                        for (origin, direction, low, high) in [
                            (segment[0].0, delta.0, min.0, max.0),
                            (segment[0].1, delta.1, min.1, max.1),
                        ] {
                            if direction.abs() < 1e-8 {
                                if origin < low || origin > high {
                                    exit = -1.0;
                                }
                            } else {
                                let a = (low - origin) / direction;
                                let b = (high - origin) / direction;
                                enter = enter.max(a.min(b));
                                exit = exit.min(a.max(b));
                            }
                        }
                        assert!(
                            enter > exit,
                            "compact={compact}: {segment:?} crosses {}",
                            chip.label
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn copper_does_not_touch_unrelated_pin_pads() {
        for compact in [false, true] {
            let (chips, routes, _) = build(compact);
            for chip in &chips {
                for side in [-1.0, 1.0] {
                    let indices: Vec<_> = (0..chip.pins_per_side).collect();
                    for pad in chip.pins(side, &indices).pins {
                        for route in &routes {
                            if route.first() == Some(&pad) || route.last() == Some(&pad) {
                                continue;
                            }
                            for segment in route.windows(2) {
                                let delta = sub(segment[1], segment[0]);
                                let t = (dot(sub(pad, segment[0]), delta) / dot(delta, delta))
                                    .clamp(0.0, 1.0);
                                let distance = sub(pad, add(segment[0], mul(delta, t)));
                                assert!(
                                    distance.0.hypot(distance.1) > 9.5,
                                    "compact={compact}: {segment:?} touches {} pad {pad:?}",
                                    chip.label
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
