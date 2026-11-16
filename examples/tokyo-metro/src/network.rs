use std::collections::BTreeMap;

use crate::data;

#[derive(Clone, Debug)]
pub struct Station {
    pub id: String,
    pub name: String,
    pub lines: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct Stop {
    pub station: usize,
    pub code: String,
}

#[derive(Clone, Debug)]
pub struct Line {
    pub code: &'static str,
    pub name: &'static str,
    pub japanese: &'static str,
    pub color: &'static str,
    pub operator: &'static str,
    pub stops: Vec<Stop>,
    // Edges index line stops. A route occurrence may visit a stop more than once.
    pub edges: Vec<(usize, usize)>,
    pub chains: Vec<Vec<usize>>,
}

#[derive(Clone, Debug)]
pub struct Network {
    pub stations: Vec<Station>,
    pub lines: Vec<Line>,
    pub transfers: Vec<(usize, usize)>,
    pub shared_track: Vec<((usize, usize), (usize, usize))>,
    pub continuations: Vec<(usize, usize, &'static str)>,
}

impl Network {
    pub fn tokyo() -> Self {
        let mut result = Self {
            stations: vec![],
            lines: vec![],
            transfers: vec![],
            shared_track: vec![],
            continuations: vec![],
        };
        let mut by_name = BTreeMap::new();
        for (index, &(code, name, japanese, color, raw)) in data::METRO
            .iter()
            .chain(data::TOEI)
            .chain(std::iter::once(&(
                "JY",
                "Yamanote",
                "山手線",
                "80A943",
                data::YAMANOTE,
            )))
            .enumerate()
        {
            let mut entries: Vec<(String, String)> = raw
                .split('|')
                .enumerate()
                .map(|(i, entry)| {
                    if index < 9 {
                        let (number, name) = entry.split_once(' ').unwrap();
                        (number.into(), name.into())
                    } else {
                        (format!("{code}{:02}", i + 1), entry.into())
                    }
                })
                .collect();
            // Keep branch stops after the main line; numbering remains operator-authored.
            if code == "M" {
                entries.sort_by_key(|e| (e.0.starts_with('m'), e.0.clone()));
            }
            let stops: Vec<Stop> = entries
                .into_iter()
                .map(|(code, name)| {
                    let id = *by_name.entry(name.clone()).or_insert_with(|| {
                        let id = result.stations.len();
                        result.stations.push(Station {
                            id: name.to_lowercase().replace([' ', '-'], "_"),
                            name,
                            lines: vec![],
                        });
                        id
                    });
                    result.stations[id].lines.push(index);
                    Stop { station: id, code }
                })
                .collect();
            let n = stops.len();
            let chains = match code {
                "M" => vec![(0..25).collect(), vec![5, 27, 26, 25]],
                "E" => vec![
                    (0..28).chain(std::iter::once(0)).collect(),
                    (27..38).collect(),
                ],
                "JY" => vec![(0..n).chain(std::iter::once(0)).collect()],
                _ => vec![(0..n).collect()],
            };
            let edges = chains
                .iter()
                .flat_map(|c| c.windows(2).map(|w| (w[0], w[1])))
                .collect();
            result.lines.push(Line {
                code,
                name,
                japanese,
                color,
                operator: if index < 9 {
                    "Tokyo Metro"
                } else if index < 13 {
                    "Toei Subway"
                } else {
                    "JR East"
                },
                stops,
                edges,
                chains,
            });
        }
        // Pedestrian interchanges do not merge station identities or create track edges.
        for group in [
            &["Akasaka-mitsuke", "Nagatacho"][..],
            &["Kokkai-gijidomae", "Tameike-sanno"],
            &["Hibiya", "Yurakucho"],
            &["Ginza", "Ginza-itchome"],
            &["Tsukiji", "Shintomicho"],
            &["Awajicho", "Shin-ochanomizu", "Ogawamachi"],
            &["Korakuen", "Kasuga"],
            &[
                "Ueno-hirokoji",
                "Naka-okachimachi",
                "Ueno-okachimachi",
                "Okachimachi",
            ],
            &["Shinjuku", "Shinjuku-nishiguchi"],
            &["Akihabara", "Iwamotocho"],
            &["Higashi-nihombashi", "Bakuro-yokoyama"],
            &["Harajuku", "Meiji-jingumae"],
            &["Hamamatsucho", "Daimon"],
            &["Tamachi", "Mita"],
            &["Toranomon", "Toranomon Hills"],
        ] {
            for name in &group[1..] {
                result.transfers.push((by_name[group[0]], by_name[*name]));
            }
        }
        // Only verified shared tracks, not merely adjacent tunnels between the same stations.
        for (a, b, count) in [(7, 10, 2), (5, 8, 5)] {
            for edge in 0..count {
                result.shared_track.push(((a, edge), (b, edge)));
            }
        }
        for (line, stop, label) in [
            (2, 21, "Tobu Skytree Line"),
            (3, 0, "JR Chuo–Sobu Line"),
            (3, 22, "JR / Toyo Rapid"),
            (4, 0, "Odakyu Line"),
            (4, 18, "JR Joban Line"),
            (5, 0, "Tobu Tojo Line"),
            (5, 5, "Seibu Yurakucho Line"),
            (6, 0, "Tokyu Den-en-toshi Line"),
            (6, 13, "Tobu Skytree Line"),
            (7, 0, "Tokyu Meguro Line"),
            (7, 18, "Saitama Railway"),
            (8, 0, "Tobu Tojo Line"),
            (8, 5, "Seibu Yurakucho Line"),
            (8, 15, "Tokyu Toyoko Line"),
            (9, 6, "Keikyu Line"),
            (9, 19, "Keisei Oshiage Line"),
            (10, 0, "Tokyu Meguro Line"),
            (11, 0, "Keio New Line"),
        ] {
            result.continuations.push((line, stop, label));
        }
        result
    }

    pub fn station(&self, name: &str) -> usize {
        self.stations
            .iter()
            .position(|s| s.name == name)
            .expect(name)
    }
}

impl Line {
    pub fn edge_between(&self, a: usize, b: usize) -> (usize, bool) {
        self.edges
            .iter()
            .enumerate()
            .find_map(|(i, &(x, y))| {
                if (a, b) == (x, y) {
                    Some((i, false))
                } else if (a, b) == (y, x) {
                    Some((i, true))
                } else {
                    None
                }
            })
            .expect("service must traverse a physical line edge")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_operator_lists_and_valid_edges() {
        let n = Network::tokyo();
        assert_eq!(
            n.lines.iter().map(|l| l.stops.len()).collect::<Vec<_>>(),
            [19, 28, 22, 23, 20, 24, 14, 19, 16, 20, 27, 21, 38, 30]
        );
        for l in &n.lines {
            let mut codes = std::collections::BTreeSet::new();
            for s in &l.stops {
                assert!(codes.insert(&s.code));
                assert!(s.station < n.stations.len());
            }
            for &(a, b) in &l.edges {
                assert!(a != b && a < l.stops.len() && b < l.stops.len());
            }
        }
        assert_eq!(n.lines[1].edges.len(), 27);
        assert!(n.lines[1].edges.contains(&(5, 27)));
        assert!(n.lines[12].edges.contains(&(27, 0)));
        assert!(n.lines[13].edges.contains(&(29, 0)));
    }
    #[test]
    fn shared_tracks_and_transfers_preserve_identities() {
        let n = Network::tokyo();
        for &((a, e), (b, f)) in &n.shared_track {
            let (x, y) = n.lines[a].edges[e];
            let (u, v) = n.lines[b].edges[f];
            assert_eq!(
                (n.lines[a].stops[x].station, n.lines[a].stops[y].station),
                (n.lines[b].stops[u].station, n.lines[b].stops[v].station)
            );
        }
        assert_ne!(n.station("Kasuga"), n.station("Korakuen"));
        assert!(n
            .transfers
            .contains(&(n.station("Korakuen"), n.station("Kasuga"))));
    }
}
