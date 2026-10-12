#![allow(unused_imports)]

use crate::*;
use pax_engine::api::cursor::CursorStyle;
use pax_engine::api::*;
use pax_engine::*;

/// The surface behind the logo, not the signboard's own color.
#[derive(PartialEq, Eq)]
#[pax]
#[engine_import_path("pax_engine")]
pub enum LogoBackgroundMode {
    /// Dark signboard with light letters, for a light surrounding surface.
    #[default]
    Light,
    /// Light signboard with dark letters, for a dark surrounding surface.
    Dark,
}

/// The animated Pax logo, with a normalized playhead and optional click replay.
///
/// Bind `progress` to coordinate a slider or a parent-owned timeline. The
/// component does not start playback on mount or paint its surrounding surface.
/// Scale an enclosing Group from the original 920.72 × 463.95 coordinate system.
#[pax]
#[engine_import_path("pax_engine")]
#[custom(Default)]
#[file("drawing/pax_logo/animated_pax_logo.pax")]
pub struct AnimatedPaxLogo {
    /// Selects the contrasting signboard palette. Defaults to Light to preserve
    /// existing consumers; Dark exchanges `fill` and `letter_fill` throughout.
    pub background_mode: Property<LogoBackgroundMode>,
    /// Signboard/post fill in Light mode, lettering/counters in Dark mode.
    pub fill: Property<Paint>,
    /// Lettering/backing fill in Light mode, signboard/post in Dark mode.
    pub letter_fill: Property<Paint>,
    /// Normalized animation position from the initial pose (0.0) to the
    /// finished logo (1.0). Bind this property for coordinated playback. Clicking
    /// restarts it unless `click_to_replay` is false.
    pub progress: Property<f64>,
    /// Restart the 1440ms animation when clicked or tapped. Defaults to true.
    /// Disable for externally driven playback or a noninteractive still logo.
    pub click_to_replay: Property<bool>,
}

impl Default for AnimatedPaxLogo {
    fn default() -> Self {
        Self {
            background_mode: Property::new(LogoBackgroundMode::Light),
            fill: Property::new(Paint::Solid(Color::BLACK)),
            letter_fill: Property::new(Paint::Solid(Color::WHITE)),
            progress: Property::new(0.0),
            click_to_replay: Property::new(true),
        }
    }
}

impl AnimatedPaxLogo {
    fn replay_if_enabled(&self) {
        if self.click_to_replay.get() {
            // Reset the same owner-thread view that starts the transition. A
            // shared publication is not imported until the next UI settlement.
            let progress = self.progress.local();
            progress.cancel_transitions();
            progress.set(0.0);
            progress.ease_to(
                1.0,
                Duration::Milliseconds(1440.into()),
                EasingCurve::Linear,
            );
        }
    }

    pub fn replay(&mut self, _: &NodeContext, _: Event<Click>) {
        self.replay_if_enabled();
    }

    pub fn hover(&mut self, ctx: &NodeContext, _: Event<MouseOver>) {
        if self.click_to_replay.get() {
            ctx.set_cursor(CursorStyle::Pointer);
        }
    }

    pub fn leave(&mut self, ctx: &NodeContext, _: Event<MouseOut>) {
        if self.click_to_replay.get() {
            ctx.set_cursor(CursorStyle::Auto);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pax_engine::api::pax_value::ToFromPaxAny;

    #[test]
    fn light_mode_preserves_the_original_palette() {
        let logo = AnimatedPaxLogo::default();
        assert_eq!(logo.background_mode.get(), LogoBackgroundMode::Light);
        assert_eq!(logo.fill.get(), Paint::Solid(Color::BLACK));
        assert_eq!(logo.letter_fill.get(), Paint::Solid(Color::WHITE));
    }

    #[test]
    fn background_modes_roundtrip_as_pax_property_values() {
        for mode in [LogoBackgroundMode::Light, LogoBackgroundMode::Dark] {
            let restored = LogoBackgroundMode::from_pax_any(mode.clone().to_pax_any()).unwrap();
            assert_eq!(restored, mode);
        }
    }

    #[test]
    fn replay_restarts_the_shared_progress_even_mid_animation() {
        use pax_engine::api::properties::{drain_effects, register_millis, PropertyGraph};
        let graph = PropertyGraph::new(|| {});
        let _entered = graph.enter();
        let millis = LocalProperty::new(0_u64);
        register_millis(&millis);
        let logo = AnimatedPaxLogo::default();
        let bound_progress = logo.progress.local();
        bound_progress.set(1.0);
        assert!(logo.click_to_replay.get());
        logo.replay_if_enabled();
        assert_eq!(bound_progress.get(), 0.0);
        millis.set(720);
        drain_effects(64);
        assert!((bound_progress.get() - 0.5).abs() < 1e-9);
        logo.replay_if_enabled();
        assert_eq!(bound_progress.get(), 0.0);
        millis.set(2160);
        drain_effects(64);
        assert_eq!(bound_progress.get(), 1.0);
    }

    #[test]
    fn disabled_replay_preserves_external_playback() {
        use pax_engine::api::properties::{drain_effects, register_millis, PropertyGraph};
        let graph = PropertyGraph::new(|| {});
        let _entered = graph.enter();
        let millis = LocalProperty::new(0_u64);
        register_millis(&millis);
        let logo = AnimatedPaxLogo::default();
        logo.click_to_replay.set(false);
        logo.progress.local().ease_to(
            1.0,
            Duration::Milliseconds(1000.into()),
            EasingCurve::Linear,
        );
        millis.set(250);
        drain_effects(64);
        logo.replay_if_enabled();
        assert!((logo.progress.local().get() - 0.25).abs() < 1e-9);
        millis.set(750);
        drain_effects(64);
        assert!((logo.progress.local().get() - 0.75).abs() < 1e-9);
    }
}
