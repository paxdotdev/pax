use std::fmt;
use std::str::FromStr;

/// Selects which debug-time source changes may update a running Pax app.
///
/// This policy uses `Logic` rather than a language name because compiled Rust
/// cartridges and future interpreted application-logic modules share the same
/// semantic reload lane. Release builds always behave as [`HotReloadMode::Off`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum HotReloadMode {
    /// Reload both `.pax` sources and application logic.
    #[default]
    All,
    /// Reload only `.pax` sources.
    Pax,
    /// Reload only application logic.
    Logic,
    /// Do not update the running app from source changes.
    Off,
}

impl HotReloadMode {
    /// Returns the stable CLI, environment, and Cargo-metadata spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Pax => "pax",
            Self::Logic => "logic",
            Self::Off => "off",
        }
    }

    /// Returns whether `.pax` changes may update the running app.
    pub const fn reloads_pax(self) -> bool {
        matches!(self, Self::All | Self::Pax)
    }

    /// Returns whether application-logic changes may update the running app.
    pub const fn reloads_logic(self) -> bool {
        matches!(self, Self::All | Self::Logic)
    }
}

impl fmt::Display for HotReloadMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for HotReloadMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "all" => Ok(Self::All),
            "pax" => Ok(Self::Pax),
            "logic" => Ok(Self::Logic),
            "off" => Ok(Self::Off),
            _ => Err(format!(
                "unsupported hot-reload mode `{value}`; expected one of: all, pax, logic, off"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HotReloadMode;

    #[test]
    fn modes_expose_independent_reload_lanes() {
        assert_eq!(
            (
                HotReloadMode::All.reloads_pax(),
                HotReloadMode::All.reloads_logic()
            ),
            (true, true)
        );
        assert_eq!(
            (
                HotReloadMode::Pax.reloads_pax(),
                HotReloadMode::Pax.reloads_logic()
            ),
            (true, false)
        );
        assert_eq!(
            (
                HotReloadMode::Logic.reloads_pax(),
                HotReloadMode::Logic.reloads_logic()
            ),
            (false, true)
        );
        assert_eq!(
            (
                HotReloadMode::Off.reloads_pax(),
                HotReloadMode::Off.reloads_logic()
            ),
            (false, false)
        );
    }

    #[test]
    fn stable_spellings_round_trip() {
        for mode in [
            HotReloadMode::All,
            HotReloadMode::Pax,
            HotReloadMode::Logic,
            HotReloadMode::Off,
        ] {
            assert_eq!(mode.as_str().parse(), Ok(mode));
            assert_eq!(mode.to_string(), mode.as_str());
        }
    }
}
