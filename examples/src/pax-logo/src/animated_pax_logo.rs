#![allow(unused_imports)]

use pax_kit::pax_engine::api::cursor::CursorStyle;
use pax_kit::*;

/// The surface behind the logo, not the signboard's own color.
#[derive(PartialEq, Eq)]
#[pax]
pub enum LogoBackgroundMode {
    /// Dark signboard with light letters, for a light surrounding surface.
    #[default]
    Light,
    /// Light signboard with dark letters, for a dark surrounding surface.
    Dark,
}

#[pax]
#[custom(Default)]
#[file("animated_pax_logo.pax")]
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
            self.progress.cancel_transitions();
            self.progress.set(0.0);
            self.progress.ease_to(
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
    use pax_kit::pax_value::ToFromPaxAny;

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
        use pax_kit::pax_engine::api::properties::register_millis;
        let millis = Property::new(0_u64);
        register_millis(&millis);
        let logo = AnimatedPaxLogo::default();
        let bound_progress = logo.progress.clone();
        bound_progress.set(1.0);
        assert!(logo.click_to_replay.get());
        logo.replay_if_enabled();
        assert_eq!(bound_progress.get(), 0.0);
        millis.set(720);
        assert!((bound_progress.get() - 0.5).abs() < 1e-9);
        logo.replay_if_enabled();
        assert_eq!(bound_progress.get(), 0.0);
        millis.set(2160);
        assert_eq!(bound_progress.get(), 1.0);
    }

    #[test]
    fn disabled_replay_preserves_external_playback() {
        use pax_kit::pax_engine::api::properties::register_millis;
        let millis = Property::new(0_u64);
        register_millis(&millis);
        let logo = AnimatedPaxLogo::default();
        logo.click_to_replay.set(false);
        logo.progress.ease_to(
            1.0,
            Duration::Milliseconds(1000.into()),
            EasingCurve::Linear,
        );
        millis.set(250);
        logo.replay_if_enabled();
        assert!((logo.progress.get() - 0.25).abs() < 1e-9);
        millis.set(750);
        assert!((logo.progress.get() - 0.75).abs() < 1e-9);
    }
}
