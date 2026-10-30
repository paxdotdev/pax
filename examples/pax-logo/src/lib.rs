#![allow(unused_imports)]

use pax_kit::pax_engine::api::cursor::CursorStyle;
use pax_kit::*;

pax_kit::runnable! { root: crate::Example }

const UNFURL_MS: f64 = 1440.;
const IDLE_START_MS: f64 = 2100.;
const IDLE_PERIOD_MS: f64 = 12000.;

#[pax]
#[custom(Default)]
#[file("lib.pax")]
pub struct Example {
    pub logo_progress: Property<f64>,
    pub background_mode: Property<LogoBackgroundMode>,
    /// Default-on treatment for this scene; false renders the plain logo.
    pub radiance_enabled: Property<bool>,
    pub playhead_ms: Property<f64>,
    pub playing: Property<bool>,
    pub visible: Property<bool>,
    pub last_time_ms: Property<f64>,
    pub intro_light_x: Property<f64>,
    pub intro_light_y: Property<f64>,
    pub light_brightness: Property<f64>,
    pub idle_light_x: Property<f64>,
    pub idle_light_y: Property<f64>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            logo_progress: Property::new(0.),
            background_mode: Property::new(LogoBackgroundMode::Dark),
            radiance_enabled: Property::new(true),
            playhead_ms: Property::new(0.),
            playing: Property::new(true),
            visible: Property::new(true),
            last_time_ms: Property::new(0.),
            intro_light_x: Property::new(70.),
            intro_light_y: Property::new(23.34),
            light_brightness: Property::new(5.),
            idle_light_x: Property::new(28.),
            idle_light_y: Property::new(233.),
        }
    }
}

// Extend the study's 0–100% path (160,225)–(820,185) to −20–120%.
// Start at the slide's settled endpoint with zero velocity and a 12-second period.
fn idle_position(playhead_ms: f64) -> (f64, f64) {
    let phase = ((playhead_ms - IDLE_START_MS).max(0.) / IDLE_PERIOD_MS).rem_euclid(1.);
    let cosine = (phase * std::f64::consts::TAU).cos();
    (490. - 462. * cosine, 205. + 28. * cosine)
}

impl Example {
    fn set_playhead(&self, millis: f64) {
        self.playhead_ms.local().set(millis);
        let (x, y) = idle_position(millis);
        self.idle_light_x.local().set(x);
        self.idle_light_y.local().set(y);
    }

    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.last_time_ms.set(ctx.elapsed_time_millis() as f64);
        self.set_playhead(0.);
    }

    pub fn advance(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        // Use elapsed time so the twelve-second path does not slow down when
        // an expensive frame misses its deadline. Visibility resets the clock.
        let dt = (now - self.last_time_ms.get()).max(0.);
        self.last_time_ms.set(now);
        if self.playing.get() && self.visible.get() {
            self.set_playhead(self.playhead_ms.get() + dt);
        }
    }

    pub fn visibility(&mut self, ctx: &NodeContext, event: Event<ViewportProximityChange>) {
        self.visible.set(event.current.is_in_viewport());
        self.last_time_ms.set(ctx.elapsed_time_millis() as f64);
    }

    pub fn toggle_background(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.background_mode.set(match self.background_mode.get() {
            LogoBackgroundMode::Light => LogoBackgroundMode::Dark,
            LogoBackgroundMode::Dark => LogoBackgroundMode::Light,
        });
    }

    pub fn toggle_radiance(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        self.radiance_enabled.set(!self.radiance_enabled.get());
        self.last_time_ms.set(ctx.elapsed_time_millis() as f64);
    }

    pub fn replay(&mut self, ctx: &NodeContext, _: Event<Click>) {
        self.set_playhead(0.);
        self.playing.set(true);
        self.last_time_ms.set(ctx.elapsed_time_millis() as f64);
    }

    pub fn scrub_logo(&mut self, _: &NodeContext, event: Event<SliderChange>) {
        self.playing.set(false);
        self.set_playhead(event.value.clamp(0., 1.) * UNFURL_MS);
    }

    pub fn hover(&mut self, ctx: &NodeContext, _: Event<MouseOver>) {
        ctx.set_cursor(CursorStyle::Pointer);
    }

    pub fn leave(&mut self, ctx: &NodeContext, _: Event<MouseOut>) {
        ctx.set_cursor(CursorStyle::Auto);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_handoff_is_stationary_at_the_left_endpoint() {
        assert_eq!(idle_position(IDLE_START_MS), (28., 233.));
        let next = idle_position(IDLE_START_MS + 1.);
        assert!((next.0 - 28.).abs() < 0.0001);
        assert!((next.1 - 233.).abs() < 0.0001);
    }

    #[test]
    fn idle_reaches_the_right_after_six_seconds_and_repeats_after_twelve() {
        assert_eq!(idle_position(IDLE_START_MS + 6000.), (952., 177.));
        for offset in [0., 173., 3000., 8450., 11999.] {
            let a = idle_position(IDLE_START_MS + offset);
            let b = idle_position(IDLE_START_MS + offset + 12000.);
            assert!((a.0 - b.0).abs() < 1e-9);
            assert!((a.1 - b.1).abs() < 1e-9);
        }
    }
}
