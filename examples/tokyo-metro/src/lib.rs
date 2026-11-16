use geometry::{Curve, Point};
use motion::{CameraMotion, LayoutMotion};
use network::Network;
use pax_kit::*;
use simulation::{Simulation, TrainProvider};
mod cartography;
mod data;
pub mod geometry;
mod layers;
mod map_data;
mod motion;
mod navigation;
#[allow(unused_imports)]
use layers::{RoutePaths, RouteStations, RouteTrains};
#[allow(unused_imports)]
use navigation::AtlasRoute;
pub mod network;
pub mod simulation;
mod surface;
#[allow(unused_imports)]
use surface::{AtlasSurface, LineChoice, StationChoice};

pax_kit::runnable! { root: crate::TokyoAtlas }

#[pax]
pub struct LineView {
    pub id: usize,
    pub code: String,
    pub title: String,
    pub color: Color,
}
#[pax]
pub struct StationView {
    pub id: usize,
    pub x: f64,
    pub y: f64,
    pub badge_scale: f64,
    pub badge: String,
    pub color: Color,
    pub show_code: bool,
    pub font: Font,
    pub font_size: f64,
    pub label_height: f64,
    pub label_x: f64,
    pub label_y: f64,
    pub label_width: f64,
    pub note: String,
    pub note_x: f64,
    pub note_y: f64,
    pub note_width: f64,
    pub note_height: f64,
    pub note_font_size: f64,
    pub label: String,
    pub show_label: bool,
}
#[pax]
#[derive(PartialEq)]
pub struct TrainView {
    pub id: usize,
    pub x: f64,
    pub y: f64,
    pub angle: f64,
    pub color: Color,
}

#[pax]
pub struct SearchHit {
    pub id: usize,
    pub label: String,
}

pub struct AtlasStore {
    network: Network,
    simulation: Simulation,
    layout: LayoutMotion,
    camera: CameraMotion,
    map_labels: Vec<cartography::Label>,
    layers: Vec<LayerState>,
    observations: Vec<simulation::Observation>,
    viewport: (f64, f64),
    selected: u32,
    navigation: navigation::Navigation,
    last_time: u64,
    simulation_time: f64,
    map_camera: (f64, f64, f64),
    last_zoom: f64,
    last_search: String,
    focus_station: Option<usize>,
    restore_camera: bool,
}
impl Store for AtlasStore {}

#[derive(Clone, Copy, PartialEq)]
struct StationKey {
    isolated: bool,
    portrait: bool,
    fine: bool,
    scale: u64,
    width: u64,
}

struct LayerState {
    paths: Vec<PathElement>,
    stations: Vec<StationView>,
    trains: Vec<TrainView>,
    path_revision: u64,
    station_revision: u64,
    train_revision: u64,
    station_key: Option<StationKey>,
    train_time: f64,
    path_view: Option<RoutePaths>,
    station_view: Option<RouteStations>,
    train_view: Option<RouteTrains>,
}

fn low_detail_name(name: &str) -> bool {
    matches!(
        name,
        "Tokyo" | "Shinjuku" | "Shibuya" | "Ikebukuro" | "Ueno" | "Asakusa" | "Shinagawa"
    )
}

#[pax]
#[file("lib.pax")]
pub struct TokyoAtlas {
    pub lines: Property<Vec<LineView>>,
    pub choices: Property<Vec<LineView>>,
    pub choice_count: Property<usize>,
    pub search_open: Property<bool>,
    pub search_query: Property<String>,
    pub search_results: Property<Vec<SearchHit>>,
    pub station_choice: Property<usize>,
    pub transfer_elements: Property<Vec<PathElement>>,
    pub interchange_elements: Property<Vec<PathElement>>,
    pub options: Property<Vec<String>>,
    pub selected: Property<u32>,
    pub selected_train: Property<i64>,
    pub train_options: Property<Vec<String>>,
    pub train_choice: Property<u32>,
    pub train_ids: Property<Vec<usize>>,
    pub title: Property<String>,
    pub subtitle: Property<String>,
    pub detail_title: Property<String>,
    pub detail_body: Property<String>,
    pub detail_color: Property<Color>,
    pub status: Property<String>,
    pub paused: Property<bool>,
    pub reduced_motion: Property<bool>,
    pub speed: Property<f64>,
    pub zoom: Property<f64>,
    pub view_scale: Property<f64>,
    pub sheet_width: Property<f64>,
    pub sheet_height: Property<f64>,
    pub canvas_width: Property<f64>,
    pub canvas_height: Property<f64>,
    pub sidebar: Property<f64>,
    pub header: Property<f64>,
    pub body_width: Property<f64>,
    pub compact_height: Property<bool>,
    pub scroll_x: Property<f64>,
    pub scroll_y: Property<f64>,
    pub safe_top: Property<f64>,
    pub safe_bottom: Property<f64>,
    pub safe_left: Property<f64>,
    pub safe_right: Property<f64>,
}

fn color(hex: &str) -> Color {
    let n = i32::from_str_radix(hex, 16).unwrap();
    Color::rgb(
        ((n >> 16) & 255).into(),
        ((n >> 8) & 255).into(),
        (n & 255).into(),
    )
}
fn point(p: Point) -> PathElement {
    PathElement::Point(Size::Pixels(p.x.into()), Size::Pixels(p.y.into()))
}
fn path(curves: &[Curve]) -> Vec<PathElement> {
    curves
        .iter()
        .flat_map(|edge| edge.segments.iter())
        .flat_map(|c| {
            [
                point(c.p[0]),
                PathElement::Cubic(
                    Size::Pixels(c.p[1].x.into()),
                    Size::Pixels(c.p[1].y.into()),
                    Size::Pixels(c.p[2].x.into()),
                    Size::Pixels(c.p[2].y.into()),
                ),
                point(c.p[3]),
            ]
        })
        .collect()
}

impl TokyoAtlas {
    pub fn mount(&mut self, ctx: &NodeContext) {
        let network = Network::tokyo();
        let simulation = Simulation::new(&network);
        let full = geometry::map(&network);
        let map_labels = cartography::labels(&network, &full);
        self.lines.set(
            network
                .lines
                .iter()
                .enumerate()
                .map(|(id, line)| LineView {
                    id,
                    code: line.code.into(),
                    title: line.name.into(),
                    color: color(line.color),
                })
                .collect(),
        );
        let layers = full
            .iter()
            .map(|curves| LayerState {
                paths: path(curves),
                stations: vec![],
                trains: vec![],
                path_revision: 1,
                station_revision: 0,
                train_revision: 0,
                station_key: None,
                train_time: f64::NAN,
                path_view: None,
                station_view: None,
                train_view: None,
            })
            .collect();
        let observations = simulation.snapshot(&network, 610.);
        self.options.set(
            std::iter::once("All lines · network".into())
                .chain(
                    network
                        .lines
                        .iter()
                        .map(|l| format!("{}  {}", l.code, l.name)),
                )
                .collect(),
        );
        self.selected_train.set(-1);
        self.zoom.set(1.);
        self.view_scale.set(0.4);
        self.speed.set(6.);
        ctx.provide_store(AtlasStore {
            network,
            map_labels,
            simulation,
            layout: LayoutMotion::new(full),
            camera: CameraMotion::new(0.4),
            layers,
            observations,
            viewport: (0., 0.),
            selected: 0,
            navigation: navigation::Navigation::default(),
            last_time: 0,
            simulation_time: 610.,
            map_camera: (1., 0., 0.),
            last_zoom: 1.,
            last_search: "\0".into(),
            focus_station: None,
            restore_camera: false,
        })
        .expect("atlas mounted");
        self.frame(ctx);
    }
    pub fn frame(&mut self, ctx: &NodeContext) {
        let _ = ctx.with_store(|s: &mut AtlasStore| {
            self.update(s, ctx.bounds_self.get(), ctx.elapsed_millis.get(), ctx)
        });
    }
    fn update(
        &mut self,
        s: &mut AtlasStore,
        (width, height): (f64, f64),
        now: u64,
        ctx: &NodeContext,
    ) {
        let width = (width - self.safe_left.get() - self.safe_right.get()).max(1.);
        let height = (height - self.safe_top.get() - self.safe_bottom.get()).max(1.);
        let sidebar = if width >= 1100. { 238. } else { 0. };
        let compact_height = height < 500. && width >= 600.;
        let header = if compact_height {
            68.
        } else if width < 600. {
            204.
        } else {
            134.
        };
        self.body_width.set_if_neq(width);
        self.compact_height.set_if_neq(compact_height);
        let cw = (width - sidebar).max(1.);
        let ch = (height - header - 118.).max(1.);
        self.sidebar.set_if_neq(sidebar);
        self.header.set_if_neq(header);
        self.canvas_width.set_if_neq(cw);
        self.canvas_height.set_if_neq(ch);
        self.choice_count.set_if_neq(self.choices.get().len());
        let query = self.search_query.get();
        if s.last_search != query {
            let needle = query.to_lowercase().replace([' ', '-'], "");
            self.search_results.set(
                s.network
                    .stations
                    .iter()
                    .enumerate()
                    .filter(|(id, station)| {
                        station.id.replace('_', "").contains(&needle)
                            || s.network.lines.iter().any(|l| {
                                l.stops.iter().any(|st| {
                                    st.station == *id && st.code.to_lowercase().contains(&needle)
                                })
                            })
                    })
                    .take(8)
                    .map(|(id, station)| SearchHit {
                        id,
                        label: format!(
                            "{}  ·  {}",
                            station.name,
                            station
                                .lines
                                .iter()
                                .map(|&l| s.network.lines[l].code)
                                .collect::<Vec<_>>()
                                .join(" / ")
                        ),
                    })
                    .collect(),
            );
            s.last_search = query;
        }
        if self.station_choice.get() > 0 {
            let id = self.station_choice.get() - 1;
            if let Some(station) = s.network.stations.get(id) {
                let current = self.selected.get().checked_sub(1).map(|i| i as usize);
                let line = current
                    .filter(|i| station.lines.contains(i))
                    .unwrap_or(station.lines[0]);
                self.selected.set(line as u32 + 1);
                s.focus_station = Some(id);
            }
            self.station_choice.set(0);
            self.search_open.set(false);
        }
        let (selected, destination) = s.navigation.resolve(self.selected.get(), s.selected);
        self.selected.set_if_neq(selected);
        if let Some(destination) = destination {
            ctx.navigate_to(&destination, NavigationTarget::Current);
        }
        let initial = s.viewport == (0., 0.);
        let resized = s.viewport != (cw, ch);
        let changed = selected != s.selected;
        let zoomed = (self.zoom.get() - s.last_zoom).abs() > 0.0001;
        let orientation_changed = (ch > cw) != (s.viewport.1 > s.viewport.0);
        if changed || resized || zoomed {
            if resized && !changed && selected > 0 && !initial {
                let old_scale = self.view_scale.get().max(0.01);
                let center = Point::new(
                    (self.scroll_x.get() + s.viewport.0 / 2.) / old_scale,
                    (self.scroll_y.get() + s.viewport.1 / 2.) / old_scale,
                );
                let line = (selected - 1) as usize;
                s.focus_station = s.network.lines[line]
                    .edges
                    .iter()
                    .enumerate()
                    .map(|(e, &(a, _))| {
                        (
                            s.layout.current[line][e].start().distance(center),
                            s.network.lines[line].stops[a].station,
                        )
                    })
                    .min_by(|a, b| a.0.total_cmp(&b.0))
                    .map(|p| p.1);
            }
            if changed {
                self.choices.set(vec![]);
                if s.selected == 0 {
                    s.map_camera = (self.zoom.get(), self.scroll_x.get(), self.scroll_y.get());
                }
                self.selected_train.set(-1);
                self.train_choice.set(0);
                self.zoom
                    .set(if selected == 0 { s.map_camera.0 } else { 1. });
                self.scroll_x
                    .set(if selected == 0 { s.map_camera.1 } else { 0. });
                self.scroll_y
                    .set(if selected == 0 { s.map_camera.2 } else { 0. });
                s.restore_camera = selected == 0;
            }
            if changed || (selected > 0 && orientation_changed) {
                s.layout.retarget(&s.network, selected, ch > cw, now);
            }
            let size = s.layout.target_size;
            let base = if selected == 0 {
                (cw / size.0).min(ch / size.1)
            } else if ch > cw {
                (cw / size.0).min(0.9)
            } else {
                (ch / size.1).min(0.9)
            };
            s.camera.retarget(base * self.zoom.get(), now);
            if changed || initial {
                if selected > 0 {
                    let line = &s.network.lines[selected as usize - 1];
                    self.title
                        .set_if_neq(format!("{}  {}", line.code, line.name));
                    self.subtitle.set_if_neq(format!(
                        "{}  /  {}  /  {} stations",
                        line.japanese,
                        line.operator,
                        line.stops.len()
                    ));
                } else {
                    self.title.set_if_neq("Tokyo".into());
                    self.subtitle
                        .set_if_neq("13 subway lines + Yamanote".into());
                }
            }
            s.selected = selected;
            s.viewport = (cw, ch);
            s.last_zoom = self.zoom.get();
        }
        let immediate = initial || self.reduced_motion.get();
        let geometry_changed = s.layout.advance(now, immediate);
        s.camera.advance(now, immediate);
        self.view_scale.set_if_neq(s.camera.value);
        self.sheet_width.set_if_neq(s.layout.size.0);
        self.sheet_height.set_if_neq(s.layout.size.1);
        let scale = s.camera.value.max(0.01);
        for &i in &geometry_changed {
            s.layers[i].paths = path(&s.layout.current[i]);
            s.layers[i].path_revision += 1;
            // Hidden layers must refresh when they reappear, even with a paused clock.
            s.layers[i].station_key = None;
            s.layers[i].train_time = f64::NAN;
        }
        for i in 0..s.network.lines.len() {
            if s.layout.alpha[i] <= 0.01 && selected as usize != i + 1 {
                continue;
            }
            let isolated = selected as usize == i + 1;
            let small_names = scale < 0.22
                && s.network.lines[i].stops.iter().any(|stop| {
                    let st = &s.network.stations[stop.station];
                    st.lines[0] == i && low_detail_name(&st.name)
                });
            let key = StationKey {
                isolated,
                portrait: isolated && ch > cw,
                fine: scale >= 0.22,
                scale: if isolated || small_names {
                    scale.to_bits()
                } else {
                    0
                },
                width: if isolated { cw.to_bits() } else { 0 },
            };
            if s.layers[i].station_key != Some(key) || geometry_changed.contains(&i) {
                s.layers[i].stations = station_views(s, i, scale, cw, ch);
                s.layers[i].station_revision += 1;
                s.layers[i].station_key = Some(key);
            }
        }
        if selected == 0 && (initial || changed || !geometry_changed.is_empty()) {
            self.refresh_connectors(s);
        }
        if !s.layout.active && !s.camera.active {
            if s.restore_camera {
                self.scroll_x.set(s.map_camera.1);
                self.scroll_y.set(s.map_camera.2);
                s.restore_camera = false;
            }
            if let Some(id) = s.focus_station.take() {
                if let Some(line) = selected.checked_sub(1).map(|i| i as usize) {
                    let l = &s.network.lines[line];
                    if let Some(stop) = l.stops.iter().position(|st| st.station == id) {
                        if let Some((edge, &(a, _))) = l
                            .edges
                            .iter()
                            .enumerate()
                            .find(|(_, e)| e.0 == stop || e.1 == stop)
                        {
                            let p = if a == stop {
                                s.layout.current[line][edge].start()
                            } else {
                                s.layout.current[line][edge].end()
                            };
                            self.scroll_x.set((p.x * scale - cw / 2.).max(0.));
                            self.scroll_y.set((p.y * scale - ch / 2.).max(0.));
                        }
                    }
                }
            }
        }
        let delta = if s.last_time == 0 || now.saturating_sub(s.last_time) > 1500 {
            0.
        } else {
            now.saturating_sub(s.last_time) as f64 / 1000.
        };
        s.last_time = now;
        if !self.paused.get() && !self.reduced_motion.get() && delta > 0. {
            s.simulation_time += delta * self.speed.get();
            s.observations = s.simulation.snapshot(&s.network, s.simulation_time);
        }
        for i in 0..s.network.lines.len() {
            if s.layout.alpha[i] <= 0.01 {
                continue;
            }
            if s.layers[i].train_time != s.simulation_time || geometry_changed.contains(&i) {
                let trains: Vec<_> = s
                    .observations
                    .iter()
                    .filter(|o| o.line == i)
                    .map(|o| {
                        let (p, angle) = s.layout.samples[i][o.edge].sample(o.progress);
                        TrainView {
                            id: o.id,
                            x: p.x,
                            y: p.y,
                            angle: angle + if o.reverse { 180. } else { 0. },
                            color: color(s.network.lines[i].color),
                        }
                    })
                    .collect();
                let layer = &mut s.layers[i];
                if trains != layer.trains {
                    layer.trains = trains;
                    layer.train_revision += 1;
                }
                layer.train_time = s.simulation_time;
            }
        }
        for i in 0..s.layers.len() {
            s.publish_layer(i);
        }
        let observations = &s.observations;
        if changed || self.train_options.get().is_empty() {
            let visible: Vec<_> = observations
                .iter()
                .filter(|o| selected == 0 || o.line + 1 == selected as usize)
                .collect();
            self.train_options.set(
                std::iter::once("Inspect a train…".into())
                    .chain(visible.iter().map(|o| {
                        format!(
                            "{} · Demo {:03} · {}",
                            s.network.lines[o.line].code,
                            o.id + 1,
                            o.label
                        )
                    }))
                    .collect(),
            );
            self.train_ids.set(visible.iter().map(|o| o.id).collect());
        }
        if self.train_choice.get() > 0 {
            let ids = self.train_ids.get();
            if let Some(&id) = ids.get(self.train_choice.get() as usize - 1) {
                self.selected_train.set(id as i64);
            }
            self.train_choice.set(0);
        }
        if let Some(o) = observations
            .iter()
            .find(|o| o.id as i64 == self.selected_train.get())
        {
            let line = &s.network.lines[o.line];
            let station = |i: usize| s.network.stations[line.stops[i].station].name.as_str();
            self.detail_title
                .set_if_neq(format!("{} · Demo {:03}", line.name, o.id + 1));
            self.detail_color.set_if_neq(color(line.color));
            let direction = if line.code == "JY" {
                if o.reverse {
                    "Outer loop · clockwise".into()
                } else {
                    "Inner loop · counterclockwise".into()
                }
            } else {
                format!("Toward {}", station(o.destination))
            };
            self.detail_body.set_if_neq(format!(
                "{}  ·  {}\n{}\n{} {}\nNext stop: {}\n\nSimulated position · synthetic timing",
                line.operator,
                o.label,
                direction,
                if o.dwelling { "At" } else { "From" },
                station(o.current),
                station(o.next_stop)
            ));
        }
        self.status.set_if_neq(format!(
            "{} trains  ·  {}× simulation  ·  {}",
            observations
                .iter()
                .filter(|o| selected == 0 || o.line + 1 == selected as usize)
                .count(),
            self.speed.get() as u32,
            if self.paused.get() || self.reduced_motion.get() {
                "paused"
            } else {
                "running"
            }
        ));
    }
    fn refresh_connectors(&mut self, s: &AtlasStore) {
        let mut junctions = vec![Vec::new(); s.network.stations.len()];
        let positions = cartography::stop_positions(&s.network, &s.layout.current);
        for (i, line) in s.network.lines.iter().enumerate() {
            for (j, stop) in line.stops.iter().enumerate() {
                junctions[stop.station].push(positions[i][j]);
            }
        }
        self.interchange_elements.set(path(
            &junctions
                .iter()
                .flat_map(|points| points.windows(2).map(|p| Curve::straight(p[0], p[1])))
                .collect::<Vec<_>>(),
        ));
        let mut by_station = std::collections::BTreeMap::new();
        for (i, line) in s.network.lines.iter().enumerate() {
            for (e, &(a, b)) in line.edges.iter().enumerate() {
                by_station
                    .entry(line.stops[a].station)
                    .or_insert(s.layout.current[i][e].start());
                by_station
                    .entry(line.stops[b].station)
                    .or_insert(s.layout.current[i][e].end());
            }
        }
        self.transfer_elements.set(path(
            &s.network
                .transfers
                .iter()
                .map(|(a, b)| Curve::straight(by_station[a], by_station[b]))
                .collect::<Vec<_>>(),
        ));
    }
    pub fn back(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.selected.set(0);
    }
    pub fn zoom_in(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.zoom.set((self.zoom.get() * 1.3).min(8.));
    }
    pub fn zoom_out(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.zoom.set((self.zoom.get() / 1.3).max(0.1));
    }
    pub fn fit(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        let _ = ctx.with_store(|s: &mut AtlasStore| {
            let (cw, ch) = s.viewport;
            let size = s.layout.target_size;
            let base = if s.selected == 0 {
                (cw / size.0).min(ch / size.1)
            } else if ch > cw {
                (cw / size.0).min(0.9)
            } else {
                (ch / size.1).min(0.9)
            };
            self.zoom
                .set(((cw / size.0).min(ch / size.1) / base).max(0.1));
        });
        self.scroll_x.set(0.);
        self.scroll_y.set(0.);
    }
    pub fn pause(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.paused.set(!self.paused.get());
    }
    pub fn motion(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.reduced_motion.set(!self.reduced_motion.get());
    }
    pub fn speed_up(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.speed.set(if self.speed.get() == 6. { 1. } else { 6. });
    }
    pub fn dismiss(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.selected_train.set(-1);
        self.choices.set(vec![]);
        self.search_open.set(false);
    }
    pub fn open_search(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.search_open.set(true);
    }
    pub fn keyboard(&mut self, _: &NodeContext, e: Event<KeyDown>) {
        if e.keyboard.key == "Escape" {
            if self.search_open.get() {
                self.search_open.set(false);
            } else if !self.choices.get().is_empty() {
                self.choices.set(vec![]);
            } else if self.selected_train.get() >= 0 {
                self.selected_train.set(-1);
            } else {
                self.selected.set(0);
            }
        }
    }
}

fn station_views(s: &AtlasStore, i: usize, scale: f64, cw: f64, ch: f64) -> Vec<StationView> {
    let l = &s.network.lines[i];
    let selected = s.selected;
    let mut stations = Vec::with_capacity(l.stops.len());
    let mut positions = vec![Point::default(); l.stops.len()];
    for (e, &(a, b)) in l.edges.iter().enumerate() {
        positions[a] = s.layout.current[i][e].start();
        positions[b] = s.layout.current[i][e].end();
    }
    for (j, stop) in l.stops.iter().enumerate() {
        let st = &s.network.stations[stop.station];
        let first_name = st.lines[0] == i;
        let p = positions[j];
        let isolated = selected as usize == i + 1;
        let map_label = &s.map_labels[stop.station];
        let badge_scale = if isolated { 0.72 / scale } else { 1. };
        let portrait = ch > cw;
        let pill = l.code == "JY" || l.code == "E";
        let label_width = if !isolated {
            map_label.width
        } else if portrait && pill {
            if p.x < 340. {
                (p.x * scale - 24.).clamp(36., 126.)
            } else {
                (cw - p.x * scale - 24.).clamp(36., 126.)
            }
        } else if !portrait {
            85.
        } else {
            136.
        };
        let (dx, dy) = if !isolated {
            (map_label.offset.x, map_label.offset.y)
        } else if portrait {
            if pill && p.x < 340. {
                (-(label_width + 16.) / scale, -12. / scale)
            } else {
                (16. / scale, -12. / scale)
            }
        } else {
            (
                -12. / scale,
                if pill && p.y > 340. {
                    22. / scale
                } else {
                    -52. / scale
                },
            )
        };
        let text = if isolated {
            st.name.replace('-', "-\n")
        } else {
            map_label.text.clone()
        };
        let note = if isolated {
            s.network
                .continuations
                .iter()
                .filter(|&&(l, stop, _)| l == i && stop == j)
                .map(|(_, _, label)| format!("↗ {label}"))
                .collect::<Vec<_>>()
                .join(" / ")
        } else {
            String::new()
        };
        let small_map = !isolated && scale < 0.22;
        let label_unit = if small_map { 0.36 / scale } else { 1. };
        let label_x = p.x + dx * label_unit;
        let label_y = p.y + dy * label_unit;
        let show_label = isolated || (first_name && (scale >= 0.22 || low_detail_name(&st.name)));
        stations.push(StationView {
            id: i * 100 + j,
            x: p.x,
            y: p.y,
            badge_scale,
            badge: format!(
                "{}\n{}",
                stop.code.trim_end_matches(|c: char| c.is_ascii_digit()),
                &stop.code[stop.code.len() - 2..]
            ),
            color: color(l.color),
            show_code: isolated || scale >= 0.22,
            font: Font::Web(
                "Arial".into(),
                String::new(),
                FontStyle::Normal,
                if map_label.major {
                    FontWeight::Bold
                } else {
                    FontWeight::Normal
                },
            ),
            font_size: if isolated {
                (if cw < 600. { 10.5 } else { 11.5 }) / scale
            } else if map_label.major {
                27. * label_unit
            } else {
                18. * label_unit
            },
            label_height: if isolated {
                48. / scale
            } else {
                map_label.height * label_unit
            },
            label_x,
            label_y,
            label_width: if isolated {
                label_width / scale
            } else {
                label_width * label_unit
            },
            note,
            note_x: if portrait {
                p.x - 142. / scale
            } else {
                label_x
            },
            note_y: p.y + if portrait { 18. / scale } else { 22. / scale },
            note_width: 130. / scale,
            note_height: 36. / scale,
            note_font_size: 9. / scale,
            label: text,
            show_label,
        });
    }
    stations
}

impl AtlasStore {
    fn publish_layer(&self, id: usize) {
        let layer = &self.layers[id];
        let alpha = self.layout.alpha[id];
        let scale = self.camera.value.max(0.01);
        let isolated = self.selected as usize == id + 1;
        if let Some(view) = &layer.path_view {
            view.publish(layer, alpha, scale, isolated);
        }
        if let Some(view) = &layer.station_view {
            view.publish(layer, alpha, isolated);
        }
        if let Some(view) = &layer.train_view {
            view.publish(layer, alpha, scale, isolated);
        }
    }
}
