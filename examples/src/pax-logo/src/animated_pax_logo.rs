#![allow(unused_imports)]

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
    pub fill: Property<Fill>,
    /// Lettering/backing fill in Light mode, signboard/post in Dark mode.
    pub letter_fill: Property<Fill>,
    /// Normalized animation position from the initial pose (0.0) to the
    /// finished logo (1.0). Consumers own playback by binding this property.
    pub progress: Property<f64>,
}

impl Default for AnimatedPaxLogo {
    fn default() -> Self {
        Self {
            background_mode: Property::new(LogoBackgroundMode::Light),
            fill: Property::new(Fill::Solid(Color::BLACK)),
            letter_fill: Property::new(Fill::Solid(Color::WHITE)),
            progress: Property::new(0.0),
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
        assert_eq!(logo.fill.get(), Fill::Solid(Color::BLACK));
        assert_eq!(logo.letter_fill.get(), Fill::Solid(Color::WHITE));
    }

    #[test]
    fn background_modes_roundtrip_as_pax_property_values() {
        for mode in [LogoBackgroundMode::Light, LogoBackgroundMode::Dark] {
            let restored = LogoBackgroundMode::from_pax_any(mode.clone().to_pax_any()).unwrap();
            assert_eq!(restored, mode);
        }
    }
}
