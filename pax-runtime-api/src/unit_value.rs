//! Normalized unit-domain values.

use super::*;

/// A scalar value in a normalized unit domain.
///
/// `UnitValue` accepts both unitless numbers and percents. It is useful for
/// properties whose authoring domain is "part of a whole", such as path drawing
/// progress where `0.5` and `50%` describe the same position.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(crate = "crate::serde")]
pub enum UnitValue {
    /// Unitless normalized value, where `0.5` means halfway through the domain.
    Unitless(Numeric),
    /// Percent value, where `50%` means halfway through the domain.
    Percent(Numeric),
}

impl Default for UnitValue {
    fn default() -> Self {
        Self::Unitless(Numeric::F64(0.0))
    }
}

impl Display for UnitValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unitless(value) => write!(f, "{}", value),
            Self::Percent(value) => write!(f, "{}%", value),
        }
    }
}

impl UnitValue {
    /// Returns the wrapped value as a normalized unitless float.
    ///
    /// `UnitValue::Unitless(0.5)` returns `0.5`, and
    /// `UnitValue::Percent(50)` returns `0.5`.
    pub fn to_unit_float(&self) -> f64 {
        match self {
            Self::Unitless(value) => value.to_float(),
            Self::Percent(value) => value.to_float() / 100.0,
        }
    }

    /// Returns `to_unit_float()` clamped into the closed unit interval.
    pub fn to_clamped_unit_float(&self) -> f64 {
        self.to_unit_float().clamp(0.0, 1.0)
    }
}

impl Interpolatable for UnitValue {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        let start = self.to_unit_float();
        let end = other.to_unit_float();
        Self::Unitless(Numeric::F64(start + (end - start) * t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CoercionRules, PaxValue, Percent};

    #[test]
    fn numeric_and_percent_resolve_to_same_unit_float() {
        let numeric = UnitValue::Unitless(Numeric::F64(0.5));
        let percent = UnitValue::Percent(Numeric::F64(50.0));

        assert_eq!(numeric.to_unit_float(), percent.to_unit_float());
    }

    #[test]
    fn coerces_numeric_and_percent_values() {
        let numeric = UnitValue::try_coerce(PaxValue::Numeric(Numeric::F64(0.25))).unwrap();
        let percent =
            UnitValue::try_coerce(PaxValue::Percent(Percent(Numeric::F64(25.0)))).unwrap();

        assert_eq!(numeric, UnitValue::Unitless(Numeric::F64(0.25)));
        assert_eq!(percent, UnitValue::Percent(Numeric::F64(25.0)));
        assert_eq!(numeric.to_unit_float(), percent.to_unit_float());
    }

    #[test]
    fn clamped_unit_float_limits_out_of_range_values() {
        assert_eq!(
            UnitValue::Unitless(Numeric::F64(-0.25)).to_clamped_unit_float(),
            0.0
        );
        assert_eq!(
            UnitValue::Percent(Numeric::F64(125.0)).to_clamped_unit_float(),
            1.0
        );
    }

    #[test]
    fn interpolation_resolves_mixed_units_to_unitless() {
        let start = UnitValue::Unitless(Numeric::F64(0.25));
        let end = UnitValue::Percent(Numeric::F64(75.0));

        assert_eq!(
            start.interpolate(&end, 0.5),
            UnitValue::Unitless(Numeric::F64(0.5))
        );
    }
}
