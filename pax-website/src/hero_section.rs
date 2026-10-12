use crate::{RotatingHeadline, SiteTheme};
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
}

impl HeroSection {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        // Share the playhead with AnimatedPaxLogo so its default click-to-replay
        // transition owns playback after this initial reveal.
        self.logo_progress.local().ease_to(
            1.0,
            Duration::Milliseconds(1440.into()),
            EasingCurve::Linear,
        );
        self.sync_layout(ctx);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
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
