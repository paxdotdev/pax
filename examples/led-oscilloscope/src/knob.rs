use crate::geometry::line;
use pax_kit::math::Point2;
use pax_kit::*;

/// Reusable normalized instrument control. Share `value` with `bind:`.
/// Drag up/right, or scroll down/right, to increase. Drags retain their input target.
#[pax]
#[file("knob.pax")]
pub struct InstrumentKnob {
    pub value: Property<f64>,
    pub label: Property<String>,
    pub value_label: Property<String>,
    /// Optional formatted readout; empty uses the normalized 0–100 value.
    pub readout: Property<String>,
    pub color: Property<Color>,
    pub diameter: Property<f64>,
    pub marks: Property<Vec<PathElement>>,
    pub ridges: Property<Vec<PathElement>>,
    pub dragging: Property<bool>,
}

#[derive(Default)]
struct KnobState {
    drag: Option<(Option<i64>, f64, f64, f64)>,
    size: f64,
}
impl Store for KnobState {}

pub fn scrub(value: f64, dx: f64, dy: f64) -> f64 {
    (value + (dx - dy) / 180.).clamp(0., 1.)
}

impl InstrumentKnob {
    pub fn mount(&mut self, ctx: &NodeContext) {
        ctx.provide_store(KnobState::default())
            .expect("knob mounts once");
    }
    pub fn pre_render(&mut self, ctx: &NodeContext) {
        let readout = self.readout.get();
        self.value_label.set_if_neq(if readout.is_empty() {
            format!("{:.0}", self.value.get() * 100.)
        } else {
            readout
        });
        let d = self.diameter.get();
        let _ = ctx.with_store(|state: &mut KnobState| {
            if state.size == d {
                return;
            }
            state.size = d;
            let mut marks = Vec::new();
            let mut ridges = Vec::new();
            for i in 0..=20 {
                let a = (-225. + i as f64 * 13.5).to_radians();
                let (s, c) = a.sin_cos();
                let r = d * 0.61;
                let inner = if i % 5 == 0 { r - 6. } else { r - 3. };
                line(
                    &mut marks,
                    d * 0.5 + c * inner,
                    d * 0.5 + s * inner,
                    d * 0.5 + c * r,
                    d * 0.5 + s * r,
                );
            }
            for i in 0..40 {
                let a = i as f64 * std::f64::consts::TAU / 40.;
                let (s, c) = a.sin_cos();
                line(
                    &mut ridges,
                    d * 0.5 + c * d * 0.42,
                    d * 0.5 + s * d * 0.42,
                    d * 0.5 + c * d * 0.48,
                    d * 0.5 + s * d * 0.48,
                );
            }
            self.marks.set(marks);
            self.ridges.set(ridges);
        });
    }
    fn begin(&mut self, ctx: &NodeContext, id: Option<i64>, x: f64, y: f64) {
        let p = ctx.local_point(Point2::new(x, y));
        let (w, h) = ctx.bounds_self.get();
        let _ = ctx.with_store(|s: &mut KnobState| {
            if s.drag.is_none() {
                s.drag = Some((id, p.x * w, p.y * h, self.value.get()));
                self.dragging.set(true);
            }
        });
    }
    fn drag(&mut self, ctx: &NodeContext, id: Option<i64>, x: f64, y: f64) {
        let p = ctx.local_point(Point2::new(x, y));
        let (w, h) = ctx.bounds_self.get();
        let _ = ctx.with_store(|s: &mut KnobState| {
            if let Some((active, x, y, value)) = s.drag {
                if active == id {
                    let next = scrub(value, p.x * w - x, p.y * h - y);
                    self.value.set(next);
                    // Incremental deltas let a reversal respond immediately at either limit.
                    s.drag = Some((active, p.x * w, p.y * h, next));
                }
            }
        });
    }
    fn end(&mut self, ctx: &NodeContext) {
        let _ = ctx.with_store(|s: &mut KnobState| s.drag = None);
        self.dragging.set(false);
    }
    pub fn down(&mut self, ctx: &NodeContext, e: Event<MouseDown>) {
        if e.mouse.button == MouseButton::Left {
            e.prevent_default();
            self.begin(ctx, None, e.mouse.x, e.mouse.y);
        }
    }
    pub fn move_mouse(&mut self, ctx: &NodeContext, e: Event<MouseMove>) {
        self.drag(ctx, None, e.mouse.x, e.mouse.y);
    }
    pub fn up(&mut self, ctx: &NodeContext, e: Event<MouseUp>) {
        if e.mouse.button == MouseButton::Left {
            self.drag(ctx, None, e.mouse.x, e.mouse.y);
            self.end(ctx);
        }
    }
    pub fn cancel_mouse(&mut self, ctx: &NodeContext, e: Event<MouseCancel>) {
        if e.mouse.button == MouseButton::Left {
            self.end(ctx);
        }
    }
    pub fn touch(&mut self, ctx: &NodeContext, e: Event<TouchStart>) {
        if let Some(t) = e.touches.first() {
            self.begin(ctx, Some(t.identifier), t.x, t.y);
        }
    }
    pub fn move_touch(&mut self, ctx: &NodeContext, e: Event<TouchMove>) {
        e.prevent_default();
        for t in &e.touches {
            self.drag(ctx, Some(t.identifier), t.x, t.y);
        }
    }
    pub fn release_touch(&mut self, ctx: &NodeContext, _: Event<TouchEnd>) {
        self.end(ctx);
    }
    pub fn cancel_touch(&mut self, ctx: &NodeContext, _: Event<TouchCancel>) {
        self.end(ctx);
    }
    pub fn wheel(&mut self, _: &NodeContext, e: Event<Wheel>) {
        e.prevent_default();
        self.value
            .set((self.value.get() + (e.delta_y + e.delta_x) * 0.002).clamp(0., 1.));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scrub_directions_and_limits() {
        assert!(scrub(0.5, 20., 0.) > 0.5);
        assert!(scrub(0.5, 0., -20.) > 0.5);
        assert!(scrub(0.5, -20., 0.) < 0.5);
        assert_eq!(scrub(0.5, 10000., 0.), 1.);
        assert_eq!(scrub(0.5, 0., 10000.), 0.);
    }
}
