//! Rotation and 2D transform types used by layout and rendering.

use super::*;

/// Encodes a rotation in various units
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Copy)]
pub enum Rotation {
    /// Radian units (2π rad for one full rotation)
    Radians(Numeric),
    /// Degree units (360 deg for one full rotation)
    Degrees(Numeric),
    /// Percentage units (100% for one full rotation)
    Percent(Numeric),
}

impl Display for Rotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rotation::Radians(rad) => write!(f, "{}rad", rad),
            Rotation::Degrees(deg) => write!(f, "{}deg", deg),
            Rotation::Percent(per) => write!(f, "{}%", per),
        }
    }
}

impl Default for Rotation {
    fn default() -> Self {
        Self::Degrees(Numeric::F64(0.0))
    }
}

impl Interpolatable for Rotation {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Self::Degrees(Numeric::F64(
            self.get_as_degrees() + (other.get_as_degrees() - self.get_as_degrees()) * t,
        ))
    }
}

impl Rotation {
    #[allow(non_snake_case)]
    /// Returns zero degrees.
    pub fn ZERO() -> Self {
        Self::Degrees(Numeric::F64(0.0))
    }

    /// Returns a normalized float proportional to `0deg : 0.0 :: 360deg : 1.0`.
    ///
    /// For example, `0rad` maps to `0.0`, `100%` maps to `1.0`, and `720deg`
    /// maps to `2.0`.
    pub fn to_float_0_1(&self) -> f64 {
        match self {
            Self::Radians(rad) => rad.to_float() / (std::f64::consts::PI * 2.0),
            Self::Degrees(deg) => deg.to_float() / 360.0_f64,
            Self::Percent(per) => per.to_float() / 100.0,
        }
    }

    /// Returns the rotation as radians, regardless of the original unit
    pub fn get_as_radians(&self) -> f64 {
        if let Self::Radians(num) = self {
            num.to_float()
        } else if let Self::Degrees(num) = self {
            num.to_float() * std::f64::consts::PI * 2.0 / 360.0
        } else if let Self::Percent(num) = self {
            num.to_float() * std::f64::consts::PI * 2.0 / 100.0
        } else {
            unreachable!()
        }
    }

    /// Returns the rotation as degrees, regardless of the original unit
    pub fn get_as_degrees(&self) -> f64 {
        if let Self::Radians(num) = self {
            num.to_float() * 180.0 / std::f64::consts::PI
        } else if let Self::Degrees(num) = self {
            num.to_float()
        } else if let Self::Percent(num) = self {
            num.to_float() * 360.0 / 100.0
        } else {
            unreachable!()
        }
    }
}
impl Neg for Rotation {
    type Output = Rotation;
    fn neg(self) -> Self::Output {
        match self {
            Rotation::Degrees(deg) => Rotation::Degrees(-deg),
            Rotation::Radians(rad) => Rotation::Radians(-rad),
            Rotation::Percent(per) => Rotation::Percent(-per),
        }
    }
}

impl Add for Rotation {
    type Output = Rotation;

    fn add(self, rhs: Self) -> Self::Output {
        let self_rad = self.get_as_radians();
        let other_rad = rhs.get_as_radians();
        Rotation::Radians(Numeric::F64(self_rad + other_rad))
    }
}

/// A sugared representation of an Affine transform combined with an `anchor` layout property.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Transform2D {
    /// Linked list of ancestral Transform2Ds
    pub previous: Option<Box<Transform2D>>,
    /// Represents affine rotation over z axis (single-dimensional for 2D rendering)
    pub rotate: Option<Rotation>,
    /// Represents affine translation across the x-y plane
    pub translate: Option<[Size; 2]>,
    /// Represents the alignment of the (0,0) position of this element as it relates to its own bounding box. (origin offset)
    pub anchor: Option<[Size; 2]>,
    /// Represents affine scale coefficients across the x-y plane
    pub scale: Option<[Size; 2]>,
    /// Represents affine skew over x and y axes
    pub skew: Option<[Rotation; 2]>,
}

impl Interpolatable for Transform2D {}

impl Mul for Transform2D {
    type Output = Transform2D;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut ret = rhs.clone();
        ret.previous = Some(Box::new(self));
        ret
    }
}

impl Transform2D {
    /// Scale coefficients over the x-y plane.
    pub fn scale(x: Size, y: Size) -> Self {
        let mut ret = Transform2D::default();
        ret.scale = Some([x, y]);
        ret
    }
    /// Rotation over the z axis.
    pub fn rotate(z: Rotation) -> Self {
        let mut ret = Transform2D::default();
        ret.rotate = Some(z);
        ret
    }
    /// Translation over the x-y plane.
    pub fn translate(x: Size, y: Size) -> Self {
        let mut ret = Transform2D::default();
        ret.translate = Some([x, y]);
        ret
    }
    /// Transform origin point for this element, relative to its own bounding box.
    pub fn anchor(x: Size, y: Size) -> Self {
        let mut ret = Transform2D::default();
        ret.anchor = Some([x, y]);
        ret
    }
}
