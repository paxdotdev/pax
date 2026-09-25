use crate::{RotatingHeadline, SiteTheme};
use pax_kit::math::Point2;
use pax_kit::*;
use pax_logo::{AnimatedPaxLogo, LogoBackgroundMode};

#[pax]
#[file("hero_section.pax")]
pub struct HeroSection {
    pub content_height: Property<f64>,
    pub compact: Property<bool>,
    pub font_px: Property<f64>,
    pub logo_scale: Property<f64>,
    pub logo_progress: Property<f64>,
    pub playing: Property<bool>,
    pub _last_ms: Property<f64>,
}

impl HeroSection {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.playing.set(true);
        self._last_ms.set(ctx.elapsed_time_millis() as f64);
        self.sync_layout(ctx);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
        let now = ctx.elapsed_time_millis() as f64;
        let dt = (now - self._last_ms.get()).clamp(0.0, 50.0);
        self._last_ms.set(now);
        let origin = ctx.local_point(Point2::new(0.0, 0.0));
        let height = ctx.bounds_self.get().1;
        let visible =
            -origin.y * height < ctx.viewport.get().height && (1.0 - origin.y) * height > 0.0;
        if self.playing.get() && visible && self.logo_progress.get() < 1.0 {
            self.logo_progress
                .set((self.logo_progress.get() + dt / 1800.0).min(1.0));
        }
    }

    pub fn toggle_motion(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.playing.set(!self.playing.get());
        if !self.playing.get() {
            self.logo_progress.set(1.0);
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
