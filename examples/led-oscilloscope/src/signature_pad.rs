use crate::geometry::{disc, point};
use crate::ink::{canvas_width, InkPoint, BRUSH_WIDTH, INK_COLUMNS, INK_ROWS, MAX_POINTS};
use crate::message_editor::EditorInk;
use pax_kit::math::Point2;
use pax_kit::*;

#[pax]
#[file("signature_pad.pax")]
pub struct SignaturePad {
    pub ink_path: Property<Vec<PathElement>>,
    pub dots: Property<Vec<PathElement>>,
    pub brush_width: Property<f64>,
    pub content_width: Property<f64>,
    pub scroll_x: Property<f64>,
    pub scroll_max: Property<f64>,
}

#[derive(Default)]
struct PadState {
    active: Option<Option<i64>>,
    revision: u64,
    bounds: (f64, f64),
}
impl Store for PadState {}

impl SignaturePad {
    pub fn mount(&mut self, ctx: &NodeContext) {
        ctx.provide_store(PadState::default())
            .expect("signature pad mounts once");
    }

    pub fn pre_render(&mut self, ctx: &NodeContext) {
        let bounds = ctx.bounds_self.get();
        let (revision, view_column) = ctx
            .with_store(|ink: &mut EditorInk| (ink.revision, ink.view_column))
            .unwrap_or_default();
        let (dirty, resized) = ctx
            .with_store(|s: &mut PadState| {
                let resized = s.bounds != bounds;
                let dirty = s.revision != revision || resized;
                s.revision = revision;
                s.bounds = bounds;
                (dirty, resized)
            })
            .unwrap_or_default();
        let scale = bounds.0.max(1.) / INK_COLUMNS as f64;
        if resized {
            self.scroll_x.set_if_neq(view_column * scale);
        } else {
            let _ = ctx.with_store(|ink: &mut EditorInk| {
                ink.view_column = self.scroll_x.get().max(0.) / scale;
            });
        }
        if !dirty {
            return;
        }
        let mut path = Vec::new();
        let mut dots = Vec::new();
        let _ = ctx.with_store(|ink: &mut EditorInk| {
            let width = canvas_width(&ink.points) as f64 * scale;
            self.content_width.set_if_neq(width);
            self.scroll_max.set_if_neq((width - bounds.0).max(0.));
            for p in &ink.points {
                let (x, y) = (p.x * scale, p.y * scale);
                if p.start || path.is_empty() {
                    path.push(point(x, y));
                    disc(&mut dots, x, y, BRUSH_WIDTH * scale / 2.);
                } else {
                    path.extend([PathElement::Line, point(x, y)]);
                }
            }
        });
        self.ink_path.set(path);
        self.dots.set(dots);
        self.brush_width.set(BRUSH_WIDTH * scale);
    }

    fn sample(&mut self, ctx: &NodeContext, x: f64, y: f64, start: bool) {
        let local = ctx.local_point(Point2::new(x, y));
        // The handler is on the viewport component, above its internal Scroller.
        let scale = ctx.bounds_self.get().0.max(1.) / INK_COLUMNS as f64;
        let (x, y) = (
            local.x * INK_COLUMNS as f64 + self.scroll_x.get() / scale,
            local.y * INK_ROWS as f64,
        );
        if !x.is_finite() || !y.is_finite() {
            return;
        }
        let _ = ctx.with_store(|ink: &mut EditorInk| {
            if ink.points.len() >= MAX_POINTS {
                return;
            }
            if !start
                && ink
                    .points
                    .last()
                    .is_some_and(|p| (p.x - x).powi(2) + (p.y - y).powi(2) < 0.04)
            {
                return;
            }
            ink.points.push(InkPoint { x, y, start });
            ink.revision += 1;
        });
    }

    fn begin(&mut self, ctx: &NodeContext, id: Option<i64>, x: f64, y: f64) {
        let started = ctx
            .with_store(|s: &mut PadState| {
                if s.active.is_some() {
                    return false;
                }
                s.active = Some(id);
                true
            })
            .unwrap_or(false);
        if started {
            self.sample(ctx, x, y, true);
        }
    }

    fn move_pen(&mut self, ctx: &NodeContext, id: Option<i64>, x: f64, y: f64) {
        let active = ctx
            .with_store(|s: &mut PadState| s.active == Some(id))
            .unwrap_or(false);
        if active {
            let local = ctx.local_point(Point2::new(x, y));
            if !(0.0..=1.0).contains(&local.x) || !(0.0..=1.0).contains(&local.y) {
                self.end(ctx, id);
                return;
            }
            self.sample(ctx, x, y, false);
        }
    }

    fn end(&mut self, ctx: &NodeContext, id: Option<i64>) {
        let _ = ctx.with_store(|s: &mut PadState| {
            if s.active == Some(id) {
                s.active = None;
            }
        });
    }

    pub fn down(&mut self, ctx: &NodeContext, e: Event<MouseDown>) {
        if matches!(e.mouse.button, MouseButton::Left) {
            e.prevent_default();
            self.begin(ctx, None, e.mouse.x, e.mouse.y);
        }
    }
    pub fn move_mouse(&mut self, ctx: &NodeContext, e: Event<MouseMove>) {
        self.move_pen(ctx, None, e.mouse.x, e.mouse.y);
    }
    pub fn up(&mut self, ctx: &NodeContext, e: Event<MouseUp>) {
        if matches!(e.mouse.button, MouseButton::Left) {
            self.move_pen(ctx, None, e.mouse.x, e.mouse.y);
            self.end(ctx, None);
        }
    }
    pub fn out(&mut self, ctx: &NodeContext, _: Event<MouseOut>) {
        // The pad deliberately lifts the pen at its edge; entering again needs a new press.
        self.end(ctx, None);
    }
    pub fn cancel_mouse(&mut self, ctx: &NodeContext, _: Event<MouseCancel>) {
        self.end(ctx, None);
    }
    pub fn scroll(&mut self, ctx: &NodeContext, _: Event<Scroll>) {
        // Moving the viewport lifts the pen instead of joining distant canvas positions.
        let _ = ctx.with_store(|s: &mut PadState| s.active = None);
    }
    pub fn touch(&mut self, ctx: &NodeContext, e: Event<TouchStart>) {
        if let Some(t) = e.touches.first() {
            self.begin(ctx, Some(t.identifier), t.x, t.y);
        }
    }
    pub fn move_touch(&mut self, ctx: &NodeContext, e: Event<TouchMove>) {
        e.prevent_default();
        for t in &e.touches {
            self.move_pen(ctx, Some(t.identifier), t.x, t.y);
        }
    }
    pub fn release_touch(&mut self, ctx: &NodeContext, e: Event<TouchEnd>) {
        for t in &e.touches {
            self.move_pen(ctx, Some(t.identifier), t.x, t.y);
            self.end(ctx, Some(t.identifier));
        }
    }
    pub fn cancel_touch(&mut self, ctx: &NodeContext, e: Event<TouchCancel>) {
        for t in &e.touches {
            self.end(ctx, Some(t.identifier));
        }
    }
}
