use crate::SiteTheme;
use pax_kit::math::Point2;
use pax_kit::*;

/// Curated Path draw-range example with a real native scrubber.
#[pax]
#[file("path_study.pax")]
pub struct PathStudy {
    pub progress: Property<f64>,
    pub playing: Property<bool>,
    pub _last_ms: Property<f64>,
}

impl PathStudy {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.progress.set(0.72);
        self.playing.set(true);
        self._last_ms.set(ctx.elapsed_time_millis() as f64);
    }
    pub fn tick(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        let dt = ((now - self._last_ms.get()) / 1000.0).clamp(0.0, 0.05);
        self._last_ms.set(now);
        let origin = ctx.local_point(Point2::new(0.0, 0.0));
        if self.playing.get()
            && -origin.y * ctx.bounds_self.get().1 < ctx.viewport.get().height
            && (1.0 - origin.y) * ctx.bounds_self.get().1 > 0.0
        {
            self.progress
                .set((self.progress.get() + dt * 0.16).rem_euclid(1.0));
        }
    }
    pub fn scrub(&mut self, _ctx: &NodeContext, event: Event<SliderChange>) {
        self.playing.set(false);
        self.progress.set(event.value);
    }
    pub fn toggle(&mut self, _ctx: &NodeContext, _event: Event<Click>) {
        self.playing.set(!self.playing.get());
    }
}

/// Real vector materials under scoped lighting, with an editable light position.
#[pax]
#[file("material_study.pax")]
pub struct MaterialStudy {
    pub light_position: Property<f64>,
    pub diameter: Property<f64>,
}
impl MaterialStudy {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.light_position.set(0.35);
        self.measure(ctx);
    }
    pub fn measure(&mut self, ctx: &NodeContext) {
        let (width, height) = ctx.bounds_self.get();
        self.diameter
            .set_if_neq(((width - 32.0) * 0.34).min((height - 138.0) * 0.8).max(1.0));
    }
}

/// Native scrolling and clipped vector content, inside the tablet silhouette.
#[pax]
#[file("native_scroll_study.pax")]
pub struct NativeScrollStudy {}
