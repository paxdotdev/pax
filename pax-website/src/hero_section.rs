use crate::{RotatingHeadline, SiteTheme};
use pax_kit::*;

#[pax]
#[file("hero_section.pax")]
pub struct HeroSection {
    pub content_height: Property<f64>,
    pub compact: Property<bool>,
    pub font_px: Property<f64>,
    pub logo_scale: Property<f64>,
    pub logo_progress: Property<f64>,
    pub _last_ms: Property<f64>,
    pub _startup_quiet_ms: Property<f64>,
    pub _startup_deadline_ms: Property<f64>,
    pub _logo_started: Property<bool>,
}

impl HeroSection {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        self._last_ms.set(now);
        self._startup_deadline_ms.set(now + 5000.0);
        self.sync_layout(ctx);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        let previous_height = self.content_height.get();
        self.sync_layout(ctx);
        let now = ctx.elapsed_time_millis() as f64;
        let elapsed = (now - self._last_ms.get()).max(0.0);
        self._last_ms.set(now);
        if !self._logo_started.get() {
            // Start on a settled run of frames, after expensive tree/layout work,
            // instead of advancing the reveal in steps during blocked frames.
            let quiet =
                if elapsed <= 40.0 && (previous_height - self.content_height.get()).abs() < 0.5 {
                    self._startup_quiet_ms.get() + elapsed
                } else {
                    0.0
                };
            self._startup_quiet_ms.set(quiet);
            // A persistently low frame rate must not suppress the logo forever.
            if quiet < 100.0 && now < self._startup_deadline_ms.get() {
                return;
            }
            self._logo_started.set(true);
            // Yield startup work before starting the shared, replayable playhead.
            self.logo_progress.local().ease_to(
                1.0,
                Duration::Milliseconds(1440.into()),
                EasingCurve::Linear,
            );
        }
    }

    fn sync_layout(&mut self, ctx: &NodeContext) {
        crate::section_layout::publish_height(ctx, "hero_content", &self.content_height);
        let width = ctx.bounds_self.get().0;
        let compact = width < 900.0;
        self.compact.set_if_neq(compact);
        let text_width = if compact {
            width - 48.0
        } else {
            width * 0.88 * 0.66
        };
        self.font_px
            .set_if_neq((text_width / 11.4).clamp(23.0, 90.0));
        let logo_width = if compact {
            (width - 48.0).min(300.0) * 0.8
        } else {
            width * 0.88 * 0.28
        };
        self.logo_scale.set_if_neq(logo_width / 920.72 * 100.0);
    }
}
