use crate::{EdgeData, RoutePoint};
use pax_kit::pax_engine::api::{cursor::CursorStyle, math::Point2};
use pax_kit::*;

// The sheet catches input below the cards. Distance to the centerline resolves
// neighboring lanes without overlapping hit rectangles choosing the wrong edge.
#[pax]
#[inlined(
    <Rectangle anchor=0% width=100% height=100% fill=TRANSPARENT/>
    @settings {
        @mouse_move: trace
        @mouse_out: leave
        @click: toggle
    }
)]
pub struct EdgeLayer {
    pub edges: Property<Vec<EdgeData>>,
    pub sheet_scale: Property<f64>,
    pub hovered: Property<String>,
    pub pinned: Property<String>,
}

pub fn distance(point: (f64, f64), a: &RoutePoint, b: &RoutePoint) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length = dx * dx + dy * dy;
    let t = if length > 0. {
        ((point.0 - a.x) * dx + (point.1 - a.y) * dy) / length
    } else {
        0.
    }
    .clamp(0., 1.);
    (point.0 - a.x - t * dx).hypot(point.1 - a.y - t * dy)
}

pub fn nearest(edges: &[EdgeData], point: (f64, f64), tolerance: f64) -> String {
    edges
        .iter()
        .filter_map(|edge| {
            let d = edge
                .points
                .windows(2)
                .map(|p| distance(point, &p[0], &p[1]))
                .fold(f64::INFINITY, f64::min);
            (d <= tolerance).then_some((d, edge.id.as_str()))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, id)| id.to_owned())
        .unwrap_or_default()
}

impl EdgeLayer {
    fn at(&self, ctx: &NodeContext, x: f64, y: f64, tolerance: f64) -> String {
        let p = ctx.local_point(Point2::new(x, y));
        // local_point returns fractions of the node's bounds, while routes use
        // sheet pixels. Keep the picking radius constant in viewport pixels.
        let (width, height) = ctx.bounds_self.get();
        self.edges.read(|edges| {
            nearest(
                edges,
                (p.x * width, p.y * height),
                tolerance / self.sheet_scale.get().max(0.1),
            )
        })
    }
    pub fn trace(&mut self, ctx: &NodeContext, event: Event<MouseMove>) {
        let id = self.at(ctx, event.mouse.x, event.mouse.y, 6.);
        if self.hovered.get() != id {
            ctx.set_cursor(if id.is_empty() {
                CursorStyle::Auto
            } else {
                CursorStyle::Pointer
            });
            self.hovered.set(id);
        }
    }
    pub fn leave(&mut self, ctx: &NodeContext, _: Event<MouseOut>) {
        self.hovered.set(String::new());
        ctx.set_cursor(CursorStyle::Auto);
    }
    pub fn toggle(&mut self, ctx: &NodeContext, event: Event<Click>) {
        let id = self.at(ctx, event.mouse.x, event.mouse.y, 10.);
        self.pinned.set(if self.pinned.get() == id {
            String::new()
        } else {
            id
        });
    }
}
