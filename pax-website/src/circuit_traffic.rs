use super::layout::{Point, Wiring};

const PACKAGE_HOP_MS: u64 = 2_167;
const EMISSION_MS: u64 = 840;
const INGRESS_PX_PER_SECOND: f64 = 153.3;
pub const PACKET_SLOTS: usize = 10;

#[derive(Clone, Debug)]
pub struct Traffic {
    pub packets: Vec<Packet>,
    next_id: usize,
    next_emission: u64,
    now: u64,
}

#[derive(Clone, Debug)]
pub struct Packet {
    pub id: usize,
    pub route: usize,
    hop_start: u64,
    hop_ms: u64,
    deadline: u64,
}

impl Default for Traffic {
    fn default() -> Self {
        Self {
            packets: Vec::new(),
            next_id: 0,
            next_emission: 0,
            // Start with traffic in flight and the next cyan emission at the jack.
            now: 12 * EMISSION_MS,
        }
    }
}

impl Traffic {
    pub fn advance(
        &mut self,
        ms: u64,
        wiring: &Wiring,
        tracks: &[Track],
        mut choose: impl FnMut(usize) -> usize,
    ) {
        self.now += ms;
        while self.next_emission <= self.now {
            let id = self.next_id;
            let route = wiring.ethernet[id % 3];
            self.packets.push(Packet {
                id,
                route,
                hop_start: self.next_emission,
                hop_ms: tracks[route].ingress_ms(),
                deadline: self.next_emission + PACKET_SLOTS as u64 * EMISSION_MS,
            });
            self.next_id += 1;
            self.next_emission += EMISSION_MS;
        }
        self.packets.retain_mut(|packet| {
            while self.now - packet.hop_start >= packet.hop_ms {
                let outputs = &wiring.outputs[wiring.destination[packet.route]];
                if outputs.is_empty() {
                    return false;
                }
                packet.route = outputs[choose(outputs.len())];
                packet.hop_start += packet.hop_ms;
                // The longer branch finishes its final wire before this emission slot
                // is reused. Every other packet can then light the copper within five lights.
                packet.hop_ms = PACKAGE_HOP_MS.min(packet.deadline - packet.hop_start);
                debug_assert!(packet.hop_ms > 0);
            }
            true
        });
    }

    pub fn progress(&self, packet: &Packet) -> f64 {
        (self.now - packet.hop_start) as f64 / packet.hop_ms as f64
    }
}

/// Cumulative distances keep the velocity constant through the octilinear bends.
#[derive(Clone, Debug, Default)]
pub struct Track {
    vertices: Vec<(f64, Point)>,
}

impl Track {
    pub fn new(points: Vec<Point>) -> Self {
        let mut vertices = Vec::with_capacity(points.len());
        let mut distance = 0.0;
        let mut previous = points[0];
        for p in points {
            distance += (p.0 - previous.0).hypot(p.1 - previous.1);
            vertices.push((distance, p));
            previous = p;
        }
        Self { vertices }
    }

    fn ingress_ms(&self) -> u64 {
        let length = self.vertices.last().unwrap().0;
        (1_000.0 * length / INGRESS_PX_PER_SECOND).round().max(1.0) as u64
    }

    pub fn point(&self, progress: f64) -> Point {
        let distance = self.vertices.last().unwrap().0 * progress;
        let end = self.vertices.partition_point(|(s, _)| *s <= distance);
        if end == self.vertices.len() {
            return self.vertices[end - 1].1;
        }
        let (a, (ax, ay)) = self.vertices[end - 1];
        let (b, (bx, by)) = self.vertices[end];
        let t = (distance - a) / (b - a);
        (ax + (bx - ax) * t, ay + (by - ay) * t)
    }
}

#[cfg(test)]
mod tests {
    use super::super::layout;
    use super::*;

    #[test]
    fn staggered_sources_forward_the_same_packets_to_wired_leaves() {
        for compact in [false, true] {
            let (chips, routes, wiring) = layout::build(compact);
            for (i, route) in wiring.ethernet.into_iter().enumerate() {
                assert_eq!(routes[route][0], (44.0, 77.0 + 23.0 * i as f64));
                assert_eq!(chips[wiring.destination[route]].label, "CircuitBoard");
            }
            let tracks: Vec<_> = routes.into_iter().map(Track::new).collect();
            for choose_last in [false, true] {
                let mut traffic = Traffic {
                    now: 0,
                    ..Traffic::default()
                };
                let choose = |n| if choose_last { n - 1 } else { 0 };
                traffic.advance(0, &wiring, &tracks, choose);
                assert_eq!(traffic.packets.len(), 1);
                let source = &traffic.packets[0];
                assert_eq!(source.route, wiring.ethernet[0]);
                let ingress = source.hop_ms;
                assert_eq!(ingress, if compact { 313 } else { 444 });
                traffic.advance(ingress - 1, &wiring, &tracks, choose);
                assert_eq!(traffic.packets[0].route, wiring.ethernet[0]);
                traffic.advance(1, &wiring, &tracks, choose);
                let mut destinations = vec!["CircuitBoard"];
                while let Some(p) = traffic.packets.iter().find(|p| p.id == 0) {
                    destinations.push(chips[wiring.destination[p.route]].label);
                    traffic.advance(p.hop_ms, &wiring, &tracks, choose);
                }
                if choose_last {
                    assert_eq!(
                        destinations,
                        ["CircuitBoard", "For", "CircuitPackage", "For", "CircuitPin"]
                    );
                } else {
                    assert_eq!(
                        destinations,
                        ["CircuitBoard", "For", "CircuitPackage", "Text"]
                    );
                }
            }
        }
    }

    #[test]
    fn irregular_ticks_preserve_handoffs_and_bound_direct_lights() {
        let (_, routes, wiring) = layout::build(false);
        let tracks: Vec<_> = routes.into_iter().map(Track::new).collect();
        for branch in [0, 1] {
            let mut traffic = Traffic::default();
            let choose = |n| if branch == 0 { 0 } else { n - 1 };
            let mut elapsed = 0;
            for dt in [0, 991, 9, 17, 83].into_iter().cycle().take(190) {
                traffic.advance(dt, &wiring, &tracks, choose);
                elapsed += dt;
                for p in &traffic.packets {
                    if p.hop_start == p.id as u64 * EMISSION_MS {
                        assert_eq!(p.route, wiring.ethernet[p.id % 3]);
                    }
                }
                let mut occupied = [false; PACKET_SLOTS];
                for p in &traffic.packets {
                    let slot = p.id % PACKET_SLOTS;
                    assert!(!occupied[slot], "two visible pulses share slot {slot}");
                    occupied[slot] = true;
                }
                assert!((8..=10).contains(&traffic.packets.len()));
                assert!(traffic.packets.iter().filter(|p| p.id % 2 == 0).count() <= 5);
            }
            let mut delayed = Traffic::default();
            delayed.advance(elapsed, &wiring, &tracks, choose);
            let snapshot = |t: &Traffic| {
                t.packets
                    .iter()
                    .map(|p| (p.id, p.route, p.hop_start))
                    .collect::<Vec<_>>()
            };
            assert_eq!(snapshot(&traffic), snapshot(&delayed));
        }
    }
}
