use crate::network::Network;

#[derive(Clone, Debug)]
pub struct Service {
    pub line: usize,
    pub visits: Vec<usize>,
    pub stops: Vec<bool>,
    pub label: &'static str,
    pub circular: bool,
}

#[derive(Clone, Debug)]
pub struct Observation {
    pub id: usize,
    pub line: usize,
    pub edge: usize,
    pub progress: f64,
    pub reverse: bool,
    pub current: usize,
    pub next: usize,
    pub next_stop: usize,
    pub destination: usize,
    pub dwelling: bool,
    pub label: &'static str,
}

// Providers return topology-relative observations. Neither pixels nor Pax values
// cross this boundary. A future live adapter must identify its own provenance.
pub trait TrainProvider {
    fn snapshot(&self, network: &Network, seconds: f64) -> Vec<Observation>;
}

pub struct Simulation {
    pub services: Vec<Service>,
    plans: Vec<[Vec<Leg>; 2]>,
}

struct Leg {
    current: usize,
    next: usize,
    next_stop: usize,
    destination: usize,
    edge: usize,
    reverse: bool,
    dwell: f64,
    travel: f64,
    end: f64,
}

impl Simulation {
    pub fn new(network: &Network) -> Self {
        let mut services = Vec::new();
        for (i, line) in network.lines.iter().enumerate() {
            let visits: Vec<usize> = match line.code {
                "M" => (0..25).collect(),
                // One complete trip: Hikarigaoka → around the loop → Tochomae.
                "E" => (0..38).rev().chain(std::iter::once(27)).collect(),
                "JY" => (0..30).chain(std::iter::once(0)).collect(),
                _ => (0..line.stops.len()).collect(),
            };
            services.push(Service {
                line: i,
                stops: vec![true; visits.len()],
                visits,
                label: "Local",
                circular: line.code == "JY",
            });
            if line.code == "M" {
                let visits: Vec<usize> = [25, 26, 27].into_iter().chain(5..25).collect();
                services.push(Service {
                    line: i,
                    stops: vec![true; visits.len()],
                    visits,
                    label: "Honancho branch",
                    circular: false,
                });
            }
            if line.code == "T" || line.code == "F" {
                let visits: Vec<usize> = (0..line.stops.len()).collect();
                let stops = visits
                    .iter()
                    .map(|&j| {
                        if line.code == "T" {
                            j <= 13 || j == 17 || j == 22
                        } else {
                            [0, 5, 8, 12, 14, 15].contains(&j)
                        }
                    })
                    .collect();
                services.push(Service {
                    line: i,
                    visits,
                    stops,
                    label: if line.code == "T" { "Rapid" } else { "Express" },
                    circular: false,
                });
            }
        }
        let plans = services
            .iter()
            .map(|service| {
                std::array::from_fn(|direction| {
                    let mut visits = service.visits.clone();
                    let mut stops = service.stops.clone();
                    if direction == 1 {
                        visits.reverse();
                        stops.reverse();
                    }
                    let terminal = visits.len() - 1;
                    let origin = visits[0];
                    let destination = visits[terminal];
                    if !service.circular {
                        visits.extend(visits.clone().into_iter().rev().skip(1));
                        stops.extend(stops.clone().into_iter().rev().skip(1));
                    }
                    let mut elapsed = 0.;
                    visits
                        .windows(2)
                        .enumerate()
                        .map(|(i, pair)| {
                            let (edge, reverse) =
                                network.lines[service.line].edge_between(pair[0], pair[1]);
                            let dwell = if !stops[i] {
                                0.
                            } else if !service.circular && (i == 0 || i == terminal) {
                                38.
                            } else {
                                18.
                            };
                            let travel = Self::travel(service.line, edge);
                            elapsed += dwell + travel;
                            let next_stop = (i + 1..visits.len())
                                .find(|&j| stops[j])
                                .map(|j| visits[j])
                                .unwrap_or(destination);
                            Leg {
                                current: pair[0],
                                next: pair[1],
                                next_stop,
                                destination: if i < terminal { destination } else { origin },
                                edge,
                                reverse,
                                dwell,
                                travel,
                                end: elapsed,
                            }
                        })
                        .collect()
                })
            })
            .collect();
        Self { services, plans }
    }

    // Stable arithmetic seed and an injected clock make replay identical on web
    // and native. These durations are invented; they are not timetable data.
    fn travel(line: usize, edge: usize) -> f64 {
        58. + ((line * 17 + edge * 13) % 31) as f64
    }

    pub fn observe(
        &self,
        network: &Network,
        service: usize,
        train: usize,
        seconds: f64,
    ) -> Observation {
        let _ = network;
        let id = service * 6 + train;
        let legs = &self.plans[service][train % 2];
        let service = &self.services[service];
        let duration = legs.last().unwrap().end;
        let seconds = if seconds.is_finite() {
            seconds.max(0.)
        } else {
            0.
        };
        let time = (seconds + train as f64 * duration / 6.).rem_euclid(duration);
        let leg = &legs[legs.partition_point(|l| l.end <= time).min(legs.len() - 1)];
        let local = time - (leg.end - leg.dwell - leg.travel);
        let progress = ((local - leg.dwell) / leg.travel).clamp(0., 1.);
        Observation {
            id,
            line: service.line,
            edge: leg.edge,
            progress: if leg.reverse { 1. - progress } else { progress },
            reverse: leg.reverse,
            current: leg.current,
            next: leg.next,
            next_stop: leg.next_stop,
            destination: leg.destination,
            dwelling: local < leg.dwell,
            label: service.label,
        }
    }
}

impl TrainProvider for Simulation {
    fn snapshot(&self, network: &Network, seconds: f64) -> Vec<Observation> {
        (0..self.services.len())
            .flat_map(|s| (0..6).map(move |t| self.observe(network, s, t, seconds)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_service_uses_real_edges_and_oedo_terminates() {
        let n = Network::tokyo();
        let sim = Simulation::new(&n);
        for s in &sim.services {
            for p in s.visits.windows(2) {
                n.lines[s.line].edge_between(p[0], p[1]);
            }
        }
        let oedo = sim.services.iter().find(|s| s.line == 12).unwrap();
        assert_eq!((oedo.visits[0], *oedo.visits.last().unwrap()), (37, 27));
        assert_eq!(oedo.visits.iter().filter(|&&s| s == 27).count(), 2);
        assert!(!oedo.circular);
        let y = sim.services.iter().find(|s| s.line == 13).unwrap();
        assert!(y.circular);
        assert_eq!(y.visits.first(), y.visits.last());
    }
    #[test]
    fn deterministic_bounded_dwell_and_express_traversal() {
        let n = Network::tokyo();
        let sim = Simulation::new(&n);
        let s = sim
            .services
            .iter()
            .position(|s| s.line == 3 && s.label == "Rapid")
            .unwrap();
        assert!(!sim.services[s].stops[14]);
        for leg in &sim.plans[s][0] {
            if leg.current == 14 {
                assert_eq!(leg.dwell, 0.);
                assert_ne!(leg.next_stop, 15);
            }
        }
        assert_eq!(sim.services[s].visits.len(), 23);
        assert!(sim.observe(&n, s, 0, 0.).dwelling);
        assert!(!sim.observe(&n, s, 0, 40.).dwelling);
        for time in [0., 1., 123., 99999.] {
            let a = sim.snapshot(&n, time);
            let b = sim.snapshot(&n, time);
            for (a, b) in a.iter().zip(b) {
                assert_eq!(a.progress, b.progress);
                assert!((0.0..=1.0).contains(&a.progress));
            }
        }
    }
    #[test]
    fn terminal_turnarounds_and_cycle_boundaries_never_teleport() {
        let n = Network::tokyo();
        let sim = Simulation::new(&n);
        for plans in &sim.plans {
            for plan in plans {
                for pair in plan.windows(2) {
                    assert_eq!(pair[0].next, pair[1].current);
                }
                assert_eq!(plan.last().unwrap().next, plan[0].current);
            }
        }
        let service = sim.services.iter().position(|s| s.line == 12).unwrap();
        let forward = sim.services[service].visits.len() - 1;
        let leg = &sim.plans[service][0][forward];
        assert_eq!(leg.current, 27);
        assert_eq!(leg.destination, 37);
        assert_eq!(leg.dwell, 38.);
    }
}
