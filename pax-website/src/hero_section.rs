use crate::{LogoAnchor, LogoHome, RotatingHeadline, SiteTheme};
use pax_kit::*;

#[pax]
#[file("hero_section.pax")]
pub struct HeroSection {
    pub content_height: Property<f64>,
    pub _height_bound: Property<bool>,
    pub compact: Property<bool>,
    pub font_px: Property<f64>,
    pub logo_width: Property<f64>,
    pub logo_progress: Property<f64>,
    pub _vertical_padding: Property<f64>,
    pub _last_ms: Property<f64>,
    pub _last_content_height: Property<f64>,
    pub _startup_quiet_ms: Property<f64>,
    pub _startup_deadline_ms: Property<f64>,
    pub _logo_started: Property<bool>,
    pub logo_anchor: Property<LogoAnchor>,
    pub scroll_y: Property<f64>,
}

impl HeroSection {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        self._last_ms.set(now);
        self._last_content_height.set(self.content_height.get());
        self._startup_deadline_ms.set(now + 5000.0);
        self._height_bound.set(false);
        self.bind_layout(ctx);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        let previous_height = self._last_content_height.get();
        if !self._height_bound.get() {
            self._height_bound
                .set(crate::section_layout::bind_height_with_padding(
                    ctx,
                    "hero_content",
                    &self.content_height,
                    &self._vertical_padding,
                ));
        }
        let height = self.content_height.get();
        self._last_content_height.set(height);
        let now = ctx.elapsed_time_millis() as f64;
        let elapsed = (now - self._last_ms.get()).max(0.0);
        self._last_ms.set(now);
        if !self._logo_started.get() {
            // Start on a settled run of frames, after expensive tree/layout work,
            // instead of advancing the reveal in steps during blocked frames.
            let quiet = if elapsed <= 40.0 && (previous_height - height).abs() < 0.5 {
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

    fn bind_layout(&self, ctx: &NodeContext) {
        crate::section_layout::bind_compact(ctx, &self.compact, 900.0);
        let width = ctx.bounds_self.clone();
        let deps = [width.untyped()];
        let font_px = self.font_px.local();
        let logo_width = self.logo_width.local();
        let vertical_padding = self._vertical_padding.local();
        let update = move || {
            let width = width.get().0;
            // Top inset plus bottom breathing room; desktop copy starts 56px lower.
            vertical_padding.set_if_neq(if width < 900.0 { 104.0 } else { 160.0 });
            let text_width = if width < 900.0 {
                width - 48.0
            } else {
                width * 0.88 * 0.66
            };
            font_px.set_if_neq((text_width / 11.4).clamp(23.0, 90.0));
            let target_logo_width = if width < 900.0 {
                // Reserve View Source's 112px width and a 24px gap on the shared top row.
                ((width - 48.0).min(300.0) * 0.8).min((width - 184.0).max(1.0))
            } else {
                width * 0.88 * 0.28
            };
            logo_width.set_if_neq(target_logo_width);
        };
        update();
        ctx.subscribe(&deps, update);
    }
}
