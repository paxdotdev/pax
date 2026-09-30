use pax_kit::*;
// These component imports are resolved by the Pax template analyzer.
#[allow(unused_imports)]
use pax_logo::{AnimatedPaxLogo, AnimatedPaxLogoBanner, AnimatedPaxLogoPost, LogoBackgroundMode};

mod connection_link;
mod content;
mod edge_layer;
mod inspector;
#[allow(unused_imports)]
use edge_layer::EdgeLayer;
#[allow(unused_imports)]
use inspector::Inspector;
mod doc_link;
mod docs;
mod routing;
#[allow(unused_imports)]
use doc_link::DocLink;
mod layout;
mod part;
#[allow(unused_imports)]
use part::DiagramPart;

#[pax]
pub struct PartData {
    pub id: String,
    pub number: String,
    pub title: String,
    pub interface: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub font_size: f64,
    pub outline: Vec<PathElement>,
}

#[pax]
pub struct EdgeData {
    pub id: String,
    pub points: Vec<RoutePoint>,
    pub elements: Vec<PathElement>,
    pub arrow: Vec<PathElement>,
}

#[pax]
pub struct RoutePoint {
    pub x: f64,
    pub y: f64,
}

#[pax]
pub struct RelationData {
    pub target: String,
    pub title: String,
    pub description: String,
}

#[pax]
pub struct InspectionData {
    pub serial: u64,
    pub visible: bool,
    pub exit_at: u64,
    pub title: String,
    pub interface: String,
    pub body: String,
    pub source: String,
    pub source_y: f64,
    pub docs: Vec<DocData>,
    pub connections: Vec<RelationData>,
}

#[pax]
pub struct DocData {
    pub title: String,
    pub url: String,
}

#[pax]
pub struct AssemblyData {
    pub title: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[pax]
#[main]
#[file("lib.pax")]
pub struct ArchitectureDiagram {
    pub parts: Property<Vec<PartData>>,
    pub edges: Property<Vec<EdgeData>>,
    pub assemblies: Property<Vec<AssemblyData>>,
    pub selected: Property<String>,
    pub inspected: Property<String>,
    pub panels: Property<Vec<InspectionData>>,
    pub retired_panels: Property<Vec<u64>>,
    pub next_panel: Property<u64>,
    pub hovered_edge: Property<String>,
    pub pinned_edge: Property<String>,
    pub zoom: Property<f64>,
    pub sheet_scale: Property<f64>,
    pub scroll_x: Property<f64>,
    pub scroll_y: Property<f64>,
    pub export_mode: Property<bool>,
    pub header_height: Property<f64>,
}

fn point(x: f64, y: f64) -> PathElement {
    PathElement::Point(Size::Pixels(x.into()), Size::Pixels(y.into()))
}

fn polyline(points: &[(f64, f64)]) -> Vec<PathElement> {
    let mut elements = Vec::new();
    for (index, &(x, y)) in points.iter().enumerate() {
        if index > 0 {
            elements.push(PathElement::Line);
        }
        elements.push(point(x, y));
    }
    elements
}

fn chamfer(w: f64, h: f64, chip: f64) -> Vec<PathElement> {
    let mut outline = polyline(&[
        (0., 0.),
        (w - chip, 0.),
        (w, chip),
        (w, h),
        (chip, h),
        (0., h - chip),
    ]);
    outline.push(PathElement::Close);
    outline
}

fn drawing_edges(routes: &[Vec<routing::Point>], scale: f64) -> Vec<EdgeData> {
    routes
        .iter()
        .enumerate()
        .map(|(index, route)| {
            let mut points = route.clone();
            routing::inset_target(&mut points, scale);
            let tip = points[points.len() - 1];
            let previous = points[points.len() - 2];
            let length = (tip.0 - previous.0).hypot(tip.1 - previous.1);
            let (dx, dy) = ((tip.0 - previous.0) / length, (tip.1 - previous.1) / length);
            EdgeData {
                id: layout::CONNECTIONS[index].id.into(),
                points: points
                    .iter()
                    .map(|p| RoutePoint { x: p.0, y: p.1 })
                    .collect(),
                elements: polyline(&points),
                arrow: polyline(&[
                    (tip.0 - dx * 11. - dy * 5., tip.1 - dy * 11. + dx * 5.),
                    tip,
                    (tip.0 - dx * 11. + dy * 5., tip.1 - dy * 11. - dx * 5.),
                ]),
            }
        })
        .collect()
}

impl ArchitectureDiagram {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.parts.set(
            layout::PLACEMENTS
                .iter()
                .enumerate()
                .map(|(index, p)| {
                    let spec = content::PARTS
                        .iter()
                        .find(|s| s.id == p.id)
                        .expect("known part");
                    let [x, y, w, h] = p.rect;
                    let outline = chamfer(w, h, 10.);
                    PartData {
                        id: p.id.into(),
                        number: format!("{:02}", index + 1),
                        title: spec.title.into(),
                        interface: spec.interface.into(),
                        x,
                        y,
                        w,
                        h,
                        font_size: if w < 230. { 21. } else { 25. },
                        outline,
                    }
                })
                .collect(),
        );
        self.assemblies.set(
            layout::ASSEMBLIES
                .iter()
                .map(|a| AssemblyData {
                    title: a.title.into(),
                    x: a.rect[0],
                    y: a.rect[1],
                    w: a.rect[2],
                    h: a.rect[3],
                })
                .collect(),
        );

        let bounds = ctx.bounds_parent.clone();
        self.header_height.replace_with(Property::computed(
            move || {
                let (w, _) = bounds.get();
                if w >= 660. {
                    64.
                } else {
                    120.
                }
            },
            &[ctx.bounds_parent.untyped()],
        ));
        let bounds = ctx.bounds_parent.clone();
        let zoom = self.zoom.clone();
        let export = self.export_mode.clone();
        let header = self.header_height.clone();
        self.sheet_scale.replace_with(Property::computed(
            move || {
                let (w, h) = bounds.get();
                let fitted = ((w - 24.).max(1.) / layout::WIDTH)
                    .min((h - header.get() - 12.).max(1.) / layout::HEIGHT);
                if export.get() {
                    (w / layout::WIDTH).min((h - header.get()).max(1.) / layout::HEIGHT)
                } else if zoom.get() == 0.0 {
                    fitted.max(0.1)
                } else {
                    zoom.get().clamp(0.15, 2.0)
                }
            },
            &[
                ctx.bounds_parent.untyped(),
                self.zoom.untyped(),
                self.export_mode.untyped(),
                self.header_height.untyped(),
            ],
        ));
        let routes = routing::routes();
        let scale = self.sheet_scale.clone();
        self.edges.replace_with(Property::computed(
            move || drawing_edges(&routes, scale.get()),
            &[self.sheet_scale.untyped()],
        ));
    }

    pub fn update_details(&mut self, ctx: &NodeContext) {
        let id = self.selected.get();
        let now = ctx.elapsed_millis.get();
        // Keep each layer in the outer repeat until its inner @out has finished.
        // Directly removing that repeat item would promote its retained exit
        // ahead of active siblings, reversing the new-card-on-top stack.
        if self.retired_panels.read(|retired| !retired.is_empty())
            || self.panels.read(|panels| {
                panels
                    .iter()
                    .any(|p| p.visible && p.exit_at != 0 && now >= p.exit_at)
            })
        {
            let retired = self.retired_panels.get();
            let mut panels = self.panels.get();
            for panel in &mut panels {
                if panel.exit_at != 0 && now >= panel.exit_at {
                    panel.visible = false;
                }
            }
            panels.retain(|p| !retired.contains(&p.serial));
            self.panels.set(panels);
            if !retired.is_empty() {
                self.retired_panels.set(Vec::new());
            }
        }
        if self.inspected.get() == id {
            return;
        }
        self.inspected.set(id.clone());
        let mut panels = self.panels.get();
        for panel in &mut panels {
            if panel.exit_at == 0 {
                // A short head start lets the incoming card cover the old one
                // while both slides finish together. Close exits immediately.
                panel.exit_at = now + if id.is_empty() { 1 } else { 80 };
            }
        }
        if let Some(spec) = content::PARTS.iter().find(|p| p.id == id) {
            let connections: Vec<_> = layout::CONNECTIONS
                .iter()
                .filter(|e| e.source == id || e.target == id)
                .map(|edge| {
                    let outgoing = edge.source == id;
                    let peer = if outgoing { edge.target } else { edge.source };
                    let title = content::PARTS.iter().find(|p| p.id == peer).unwrap().title;
                    RelationData {
                        target: peer.into(),
                        title: format!("{} {}", if outgoing { "→" } else { "←" }, title),
                        description: edge.label.into(),
                    }
                })
                .collect();
            let serial = self.next_panel.get() + 1;
            self.next_panel.set(serial);
            let panel = InspectionData {
                serial,
                visible: true,
                exit_at: 0,
                title: spec.title.into(),
                interface: spec.interface.into(),
                body: spec.explanation.into(),
                source: spec.source.into(),
                source_y: 602. + connections.len() as f64 * 66.,
                connections,
                docs: docs::for_part(&id)
                    .iter()
                    .map(|doc| DocData {
                        title: doc.title.into(),
                        url: format!("https://docs.pax.dev/{}.html", doc.page),
                    })
                    .collect(),
            };
            // The keyed wrappers preserve front-to-back ordering during exits.
            panels.insert(0, panel);
        }
        self.panels.set(panels);
    }

    fn change_zoom(&mut self, factor: f64) {
        let before = self.sheet_scale.get();
        let after = (before * factor).clamp(0.15, 2.0);
        self.zoom.set(after);
        self.scroll_x.set(self.scroll_x.get() * after / before);
        self.scroll_y.set(self.scroll_y.get() * after / before);
    }
    pub fn zoom_in(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.change_zoom(1.35);
    }
    pub fn zoom_out(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.change_zoom(1.0 / 1.35);
    }
    fn reset(&mut self) {
        self.zoom.set(0.0);
        self.scroll_x.set(0.0);
        self.scroll_y.set(0.0);
        self.selected.set(String::new());
        self.hovered_edge.set(String::new());
        self.pinned_edge.set(String::new());
    }
    pub fn fit(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.reset();
    }
    pub fn close(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.selected.set(String::new());
    }
    pub fn exit_sheet(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.export_mode.set(false);
    }
    pub fn sheet(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.reset();
        self.export_mode.set(true);
    }
    fn step(&mut self, offset: isize) {
        let parts = self.parts.get();
        let current = parts.iter().position(|p| p.id == self.selected.get());
        let index = match current {
            Some(i) => (i as isize + offset).rem_euclid(parts.len() as isize) as usize,
            None => {
                if offset > 0 {
                    0
                } else {
                    parts.len() - 1
                }
            }
        };
        self.selected.set(parts[index].id.clone());
        if self.zoom.get() != 0.0 {
            self.scroll_x
                .set((parts[index].x * self.sheet_scale.get() - 80.).max(0.));
            self.scroll_y
                .set((parts[index].y * self.sheet_scale.get() - 80.).max(0.));
        }
    }
    pub fn keyboard(&mut self, _: &NodeContext, event: Event<KeyDown>) {
        match event.keyboard.key.as_str() {
            "+" | "=" => self.change_zoom(1.35),
            "-" => self.change_zoom(1.0 / 1.35),
            "0" => self.reset(),
            "Escape" => {
                self.pinned_edge.set(String::new());
                self.hovered_edge.set(String::new());
                self.selected.set(String::new());
                self.export_mode.set(false);
            }
            "n" | "N" => self.step(1),
            "p" | "P" => self.step(-1),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn edge_picker_tracks_bends_and_separates_neighboring_lanes() {
        let make = |id: &str, points: &[(f64, f64)]| EdgeData {
            id: id.into(),
            points: points
                .iter()
                .map(|p| RoutePoint { x: p.0, y: p.1 })
                .collect(),
            elements: Vec::new(),
            arrow: Vec::new(),
        };
        let edges = vec![
            make("a", &[(100., 0.), (100., 100.), (50., 100.)]),
            make("b", &[(110., 0.), (110., 100.)]),
        ];
        assert_eq!(edge_layer::nearest(&edges, (103., 60.), 10.), "a");
        assert_eq!(edge_layer::nearest(&edges, (107., 60.), 10.), "b");
        assert_eq!(edge_layer::nearest(&edges, (75., 103.), 10.), "a");
        assert_eq!(edge_layer::nearest(&edges, (100., 115.), 10.), "");
    }

    #[test]
    fn graph_has_unique_parts_connected_endpoints_and_valid_routes() {
        let ids: HashSet<_> = content::PARTS.iter().map(|p| p.id).collect();
        assert_eq!(ids.len(), content::PARTS.len());
        let placed: HashSet<_> = layout::PLACEMENTS.iter().map(|p| p.id).collect();
        assert_eq!(ids, placed);
        let mut edges = HashSet::new();
        let mut crossings = Vec::new();
        let routes = routing::routes();
        let mut endpoints = HashSet::new();
        for (edge, points) in layout::CONNECTIONS.iter().zip(&routes) {
            assert!(edges.insert(edge.id), "duplicate edge {}", edge.id);
            assert!(ids.contains(edge.source) && ids.contains(edge.target));
            assert!(!edge.topology.is_empty());
            for (id, side, point) in [
                (edge.source, edge.from, points[0]),
                (edge.target, edge.to, *points.last().unwrap()),
            ] {
                assert!(
                    endpoints.insert((id, side, point.0.to_bits(), point.1.to_bits())),
                    "shared port on {id}"
                );
            }
            assert!(
                points.iter().all(|p| p.1 > layout::HEADING_BOTTOM),
                "route above headings {}",
                edge.id
            );
            for pair in points.windows(2) {
                assert_ne!(pair[0], pair[1], "empty segment {}", edge.id);
                for p in layout::PLACEMENTS {
                    let [x, y, w, h] = p.rect;
                    let [(ax, ay), (bx, by)] = [pair[0], pair[1]];
                    let crosses = if ax == bx {
                        ax > x && ax < x + w && ay.min(by) < y + h && ay.max(by) > y
                    } else {
                        ay > y && ay < y + h && ax.min(bx) < x + w && ax.max(bx) > x
                    };
                    if crosses {
                        crossings.push(format!("{} crosses {} at {:?}", edge.id, p.id, pair));
                    }
                }
                assert!(
                    pair[0].0 == pair[1].0 || pair[0].1 == pair[1].1,
                    "non-orthogonal {}",
                    edge.id
                );
            }
        }
        assert!(crossings.is_empty(), "{}", crossings.join("\n"));
        for p in layout::PLACEMENTS {
            let links = docs::for_part(p.id);
            assert!((2..=4).contains(&links.len()), "doc count for {}", p.id);
            for link in links {
                let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../pax-docs/book/src")
                    .join(format!("{}.md", link.page));
                assert!(source.is_file(), "missing docs {}", link.page);
            }
            let [x, y, w, h] = p.rect;
            assert_eq!(h, layout::CARD_HEIGHT);
            assert!([200., 300., 640.].contains(&w));
            assert!(
                x >= 0.
                    && y >= 0.
                    && w > 0.
                    && h > 0.
                    && x + w <= layout::WIDTH
                    && y + h <= layout::HEIGHT
            );
            assert!(
                layout::CONNECTIONS
                    .iter()
                    .any(|e| e.source == p.id || e.target == p.id),
                "disconnected {}",
                p.id
            );
        }
    }
}
