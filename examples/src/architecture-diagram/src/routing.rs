//! Channel routing over the schematic's authored component placement.
use crate::layout::{self, Port};
use std::collections::BTreeMap;

pub type Point = (f64, f64);

pub fn port(id: &str, side: Port) -> Point {
    let [x, y, w, h] = placement(id);
    match side {
        Port::Left => (x, y + h / 2.),
        Port::Right => (x + w, y + h / 2.),
        Port::Top => (x + w / 2., y),
        Port::Bottom => (x + w / 2., y + h),
    }
}

fn placement(id: &str) -> [f64; 4] {
    layout::PLACEMENTS
        .iter()
        .find(|p| p.id == id)
        .expect("known endpoint")
        .rect
}

fn horizontal(side: Port) -> bool {
    matches!(side, Port::Left | Port::Right)
}
fn tangent(point: Point, side: Port) -> f64 {
    if horizontal(side) {
        point.1
    } else {
        point.0
    }
}
fn align(neighbor: &mut Point, endpoint: Point, side: Port) {
    if horizontal(side) {
        neighbor.1 = endpoint.1;
    } else {
        neighbor.0 = endpoint.0;
    }
}

fn simplify(points: &mut Vec<Point>) {
    points.dedup();
    let mut i = 1;
    while i + 1 < points.len() {
        let (a, b, c) = (points[i - 1], points[i], points[i + 1]);
        if (a.0 == b.0 && b.0 == c.0 && (b.1 - a.1) * (c.1 - b.1) >= 0.)
            || (a.1 == b.1 && b.1 == c.1 && (b.0 - a.0) * (c.0 - b.0) >= 0.)
        {
            points.remove(i);
        } else {
            i += 1;
        }
    }
}

pub fn routes() -> Vec<Vec<Point>> {
    let edges = layout::CONNECTIONS;
    let mut endpoints: Vec<_> = edges
        .iter()
        .map(|e| [port(e.source, e.from), port(e.target, e.to)])
        .collect();
    let mut groups = BTreeMap::<(&str, Port), Vec<(usize, usize, f64)>>::new();
    for (index, edge) in edges.iter().enumerate() {
        for (end, id, side) in [(0, edge.source, edge.from), (1, edge.target, edge.to)] {
            let center = endpoints[index][end];
            // The adjacent waypoint is aligned to the port below, so its
            // authored tangent cannot tell us which way the trace approaches.
            // Use the next bend to order the fan-out without local crossings.
            let mut route = edge.via.to_vec();
            if end == 1 {
                route.reverse();
            }
            if !route.is_empty() {
                route.remove(0);
            }
            route.push(endpoints[index][1 - end]);
            let toward = route
                .iter()
                .find(|p| tangent(**p, side) != tangent(center, side))
                .copied()
                .unwrap_or(center);
            groups
                .entry((id, side))
                .or_default()
                .push((index, end, tangent(toward, side)));
        }
    }
    for ((id, side), mut group) in groups {
        group.sort_by(|a, b| a.2.total_cmp(&b.2).then(a.0.cmp(&b.0)));
        let count = group.len();
        let rect = placement(id);
        let available = if horizontal(side) { rect[3] } else { rect[2] } - 32.;
        let spacing = if count > 1 {
            28_f64.min(available / (count - 1) as f64)
        } else {
            0.
        };
        for (rank, (index, end, _)) in group.into_iter().enumerate() {
            let offset = (rank as f64 - (count - 1) as f64 / 2.) * spacing;
            if horizontal(side) {
                endpoints[index][end].1 += offset;
            } else {
                endpoints[index][end].0 += offset;
            }
        }
    }
    let mut routes: Vec<_> = edges
        .iter()
        .enumerate()
        .map(|(index, edge)| {
            let [source, target] = endpoints[index];
            let mut points = vec![port(edge.source, edge.from)];
            points.extend_from_slice(edge.via);
            points.push(port(edge.target, edge.to));
            simplify(&mut points);
            if points.len() == 2 {
                points = if horizontal(edge.from) {
                    let x = (source.0 + target.0) / 2.;
                    vec![source, (x, source.1), (x, target.1), target]
                } else {
                    let y = (source.1 + target.1) / 2.;
                    vec![source, (source.0, y), (target.0, y), target]
                };
            } else {
                let last = points.len() - 1;
                points[0] = source;
                points[last] = target;
                align(&mut points[1], source, edge.from);
                align(&mut points[last - 1], target, edge.to);
            }
            simplify(&mut points);
            points
        })
        .collect();

    // Channel choices remain authored; track assignment is computed at mount.
    // Disjoint runs reuse a track. Concurrent runs travel as a compact bus with
    // a fixed gutter, rather than fanning across all available space.
    for (left, right) in [
        (388., 528.),
        (828., 1000.),
        (1640., 1840.),
        (2500., 2700.),
        (3000., 3120.),
    ] {
        pack_bus(&mut routes, false, left + 18., right - 18., 75., 2160.);
    }
    // The light/adapter bus stays above the renderer's title strip at y=868.
    for (low, high, left, right) in [
        (114., 162., 828., 2700.),
        (368., 464., 1640., 2700.),
        (588., 684., 1640., 2700.),
        (800., 848., 1640., 2700.),
        (1028., 1124., 1760., 2700.),
        (1248., 1344., 1760., 2700.),
        (1468., 1564., 1640., 2700.),
        (1688., 1784., 1640., 2700.),
    ] {
        pack_bus(&mut routes, true, low, high, left, right);
    }
    for route in &mut routes {
        simplify(route);
    }
    routes
}

/// Pack parallel runs within a free channel. Geometry is independent of Pax
/// properties and drawing primitives so a future viewer can reuse this pass.
fn pack_bus(routes: &mut [Vec<Point>], horizontal: bool, low: f64, high: f64, from: f64, to: f64) {
    let mut segments = Vec::new();
    for (edge, points) in routes.iter().enumerate() {
        for i in 1..points.len().saturating_sub(2) {
            let (a, b) = (points[i], points[i + 1]);
            let (ac, bc, along_a, along_b) = if horizontal {
                (a.1, b.1, a.0, b.0)
            } else {
                (a.0, b.0, a.1, b.1)
            };
            if ac == bc
                && ac >= low - 18.
                && ac <= high + 18.
                && along_a.min(along_b) >= from
                && along_a.max(along_b) <= to
            {
                segments.push((ac, edge, i, along_a.min(along_b), along_a.max(along_b)));
            }
        }
    }
    segments.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut tracks: Vec<Vec<(f64, usize, usize, f64, f64)>> = Vec::new();
    for segment in segments {
        if let Some(track) = tracks.iter_mut().find(|track| {
            track
                .iter()
                .all(|s| s.4 + 24. < segment.3 || segment.4 + 24. < s.3)
        }) {
            track.push(segment);
        } else {
            tracks.push(vec![segment]);
        }
    }
    let count = tracks.len();
    if count == 0 {
        return;
    }
    let gutter = if count > 1 {
        24_f64.min((high - low) / (count - 1) as f64)
    } else {
        0.
    };
    let start = (low + high - gutter * (count - 1) as f64) / 2.;
    for (rank, track) in tracks.into_iter().enumerate() {
        let coordinate = start + gutter * rank as f64;
        for (_, edge, segment, _, _) in track {
            if horizontal {
                routes[edge][segment].1 = coordinate;
                routes[edge][segment + 1].1 = coordinate;
            } else {
                routes[edge][segment].0 = coordinate;
                routes[edge][segment + 1].0 = coordinate;
            }
        }
    }
}

pub fn inset_target(points: &mut [Point], scale: f64) {
    let last = points.len() - 1;
    let (tip, previous) = (points[last], points[last - 1]);
    let length = (tip.0 - previous.0).hypot(tip.1 - previous.1);
    // Two screen pixels of air, including the round stroke cap, at every zoom.
    let gap = (2. / scale.max(0.1) + 1.).min((length - 0.5).max(0.));
    points[last] = (
        tip.0 - (tip.0 - previous.0) / length * gap,
        tip.1 - (tip.1 - previous.1) / length * gap,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bus_keeps_concurrent_traces_apart_and_reuses_disjoint_tracks() {
        let mut routes = vec![
            vec![(0., 0.), (100., 0.), (100., 100.), (200., 100.)],
            vec![(0., 20.), (110., 20.), (110., 80.), (200., 80.)],
            vec![(0., 140.), (120., 140.), (120., 200.), (200., 200.)],
        ];
        let endpoints: Vec<_> = routes.iter().map(|r| (r[0], *r.last().unwrap())).collect();
        pack_bus(&mut routes, false, 80., 160., 0., 240.);
        assert_eq!(routes[0][1].0, routes[2][1].0);
        assert_eq!((routes[0][1].0 - routes[1][1].0).abs(), 24.);
        for (route, endpoints) in routes.iter().zip(endpoints) {
            assert_eq!((route[0], *route.last().unwrap()), endpoints);
        }
        // Transposing the geometry uses the same allocation for horizontal buses.
        for route in &mut routes {
            for point in route {
                *point = (point.1, point.0);
            }
        }
        pack_bus(&mut routes, true, 80., 160., 0., 240.);
        assert_eq!(routes[0][1].1, routes[2][1].1);
        assert_eq!((routes[0][1].1 - routes[1][1].1).abs(), 24.);
    }

    #[test]
    fn schematic_has_separate_traces_and_short_feedback_routes() {
        let routes = routes();
        let segments: Vec<_> = routes
            .iter()
            .enumerate()
            .flat_map(|(edge, points)| {
                points.windows(2).map(move |p| {
                    let vertical = p[0].0 == p[1].0;
                    let (coordinate, a, b) = if vertical {
                        (p[0].0, p[0].1, p[1].1)
                    } else {
                        (p[0].1, p[0].0, p[1].0)
                    };
                    (edge, vertical, coordinate, a.min(b), a.max(b))
                })
            })
            .collect();
        for (i, a) in segments.iter().enumerate() {
            for assembly in layout::ASSEMBLIES {
                let [x, y, _, _] = assembly.rect;
                let label_right = x + 18. + assembly.title.len() as f64 * 11. + 8.;
                // Menlo at 17px fits in this title strip with a little air.
                let blocked = if a.1 {
                    a.2 > x + 18. && a.2 < label_right && a.3 < y + 41. && a.4 > y + 16.
                } else {
                    a.2 > y + 16. && a.2 < y + 41. && a.3 < label_right && a.4 > x + 18.
                };
                assert!(
                    !blocked,
                    "{} crosses the {} label",
                    layout::CONNECTIONS[a.0].id,
                    assembly.title
                );
            }
            for b in &segments[i + 1..] {
                if a.0 != b.0 && a.1 == b.1 && a.4.min(b.4) > a.3.max(b.3) {
                    assert!(
                        (a.2 - b.2).abs() >= 12.,
                        "traces {} and {} are too close",
                        layout::CONNECTIONS[a.0].id,
                        layout::CONNECTIONS[b.0].id
                    );
                }
            }
        }
        for id in ["input", "measurement"] {
            let index = layout::CONNECTIONS.iter().position(|e| e.id == id).unwrap();
            let length: f64 = routes[index]
                .windows(2)
                .map(|p| (p[0].0 - p[1].0).abs() + (p[0].1 - p[1].1).abs())
                .sum();
            assert!(length < 2500., "{id} takes a sheet-wide detour");
        }
    }
}
