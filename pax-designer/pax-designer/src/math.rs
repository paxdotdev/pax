use std::{f64::consts::PI, ops::Mul};

use pax_engine::{
    layout::{LayoutProperties, TransformAndBounds},
    math::{Generic, Parts, Point2, Space, Transform2, Vector2},
    NodeLocal,
};
use pax_runtime_api::{Axis, Interpolatable, Percent, Rotation, Size};

use crate::math::coordinate_spaces::Glass;

use self::coordinate_spaces::World;

#[derive(PartialEq, Default)]
pub enum SizeUnit {
    Pixels,
    #[default]
    Percent,
}

#[derive(PartialEq, Default)]
pub enum RotationUnit {
    Radians,
    #[default]
    Degrees,
    Percent,
}

pub trait GetUnit {
    type UnitType;
    fn unit(&self) -> Self::UnitType;
}

impl GetUnit for Size {
    type UnitType = SizeUnit;

    fn unit(&self) -> Self::UnitType {
        match self {
            Size::Pixels(_) => SizeUnit::Pixels,
            Size::Percent(_) => SizeUnit::Percent,
            // TODO introduce combined type
            Size::Combined(_, _) => SizeUnit::Percent,
        }
    }
}

impl GetUnit for Rotation {
    type UnitType = RotationUnit;

    fn unit(&self) -> Self::UnitType {
        match self {
            Rotation::Radians(_) => RotationUnit::Radians,
            Rotation::Degrees(_) => RotationUnit::Degrees,
            Rotation::Percent(_) => RotationUnit::Percent,
        }
    }
}

impl<T: GetUnit> GetUnit for Option<T>
where
    T::UnitType: Default,
{
    type UnitType = T::UnitType;

    fn unit(&self) -> Self::UnitType {
        match self {
            Some(v) => v.unit(),
            None => T::UnitType::default(),
        }
    }
}

pub mod coordinate_spaces {

    use pax_engine::math;

    pub struct Glass;

    impl math::Space for Glass {}

    pub struct World;

    impl math::Space for World {}

    pub struct SelectionSpace;

    impl math::Space for SelectionSpace {}
}

// min (-1.0, -1.0) for top left
// max (1.0, 1.0) for bottom right
pub struct BoxPoint;

impl Space for BoxPoint {}

impl Interpolatable for AxisAlignedBox {}

pub struct AxisAlignedBox<W = Glass> {
    min: Point2<W>,
    max: Point2<W>,
}

impl<W: Space> Default for AxisAlignedBox<W> {
    fn default() -> Self {
        AxisAlignedBox::new(Point2::default(), Point2::default())
    }
}

impl<W: Space> PartialEq for AxisAlignedBox<W> {
    fn eq(&self, other: &Self) -> bool {
        self.min == other.min && self.max == other.max
    }
}

impl<W: Space> Clone for AxisAlignedBox<W> {
    fn clone(&self) -> Self {
        Self {
            min: self.min,
            max: self.max,
        }
    }
}

impl<W: Space> std::fmt::Debug for AxisAlignedBox<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AxisAlignedBox")
            .field("top_left", &self.min)
            .field("bottom_right", &self.max)
            .finish()
    }
}

impl<W: Space> AxisAlignedBox<W> {
    pub fn new(p1: Point2<W>, p2: Point2<W>) -> Self {
        let min = Point2::new(p1.x.min(p2.x), p1.y.min(p2.y));
        let max = Point2::new(p1.x.max(p2.x), p1.y.max(p2.y));
        Self { min, max }
    }

    pub fn bound_of_boxes(boxes: impl IntoIterator<Item = AxisAlignedBox<W>>) -> Self {
        Self::bound_of_points(
            boxes
                .into_iter()
                .flat_map(|b| [b.top_left(), b.bottom_right()]),
        )
    }

    pub fn bound_of_points(points: impl IntoIterator<Item = Point2<W>>) -> Self {
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for p in points {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }
        AxisAlignedBox::new(Point2::new(min_x, min_y), Point2::new(max_x, max_y))
    }

    pub fn top_left(&self) -> Point2<W> {
        self.min
    }

    pub fn bottom_right(&self) -> Point2<W> {
        self.max
    }

    pub fn width(&self) -> f64 {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f64 {
        self.max.y - self.min.y
    }

    pub fn corners(&self) -> [Point2<W>; 4] {
        [
            self.min,
            Point2::new(self.min.x, self.max.y),
            self.max,
            Point2::new(self.max.x, self.min.y),
        ]
    }

    pub fn try_into_space<T: Space>(
        &self,
        transform: Transform2<W, T>,
    ) -> Option<AxisAlignedBox<T>> {
        // return none if transform does rotation or scales negatively
        if transform.coeffs()[1].abs() > 0.01
            || transform.coeffs()[2].abs() > 0.01
            || transform.coeffs()[0] < 0.0
            || transform.coeffs()[3] < 0.0
        {
            None
        } else {
            Some(AxisAlignedBox {
                min: transform * self.min,
                max: transform * self.max,
            })
        }
    }

    pub fn from_inner_space(&self, point: Point2<BoxPoint>) -> Point2<W> {
        let point = point.to_vector();
        debug_assert!(point.x >= -1.0 && point.x <= 1.0);
        debug_assert!(point.y >= -1.0 && point.y <= 1.0);
        let lerp_range = (point + 1.0) / 2.0;
        let x_pos = self.min.x + (self.max.x - self.min.x) * lerp_range.x;
        let y_pos = self.min.y + (self.max.y - self.min.y) * lerp_range.y;
        Point2::new(x_pos, y_pos)
    }

    pub fn to_inner_space(&self, origin: Point2<W>) -> Point2<BoxPoint> {
        let p = self.min;
        let v = self.max - self.min;
        // p + t*v = origin =>
        let t = (origin - p) / v;
        (t * 2.0 - 1.0).to_point().cast_space()
    }

    pub fn morph_constrained(
        &self,
        morph_point: Point2<W>,
        anchor: Point2<W>,
        fixed_center: bool,
        keep_aspect: bool,
    ) -> Self {
        let keep_aspect_modifier = |v: Vector2<W>| {
            let aspect_ratio = self.bottom_right() - self.top_left();
            v.coord_abs()
                .project_axis_aligned(aspect_ratio)
                .to_signums_of(v)
        };

        if fixed_center {
            let center = self.from_inner_space(Point2::new(0.0, 0.0));
            let mut v = (center - morph_point).coord_abs();
            if keep_aspect {
                v = keep_aspect_modifier(v);
            }
            AxisAlignedBox::new(center + v, center - v)
        } else {
            let mut v = morph_point - anchor;
            if keep_aspect {
                v = keep_aspect_modifier(v);
            }
            AxisAlignedBox::new(anchor + v, anchor)
        }
    }

    /// Returns the transform that moves the unit
    /// vectors to this box
    pub fn as_transform(&self) -> Transform2<Generic, W> {
        let origin = self.min;
        let vx = Vector2::new(self.width(), 0.0);
        let vy = Vector2::new(0.0, self.height());
        Transform2::compose(origin.cast_space(), vx, vy)
    }
}

#[cfg(test)]
mod tests {
    use pax_engine::math::{Generic, Point2};

    use super::AxisAlignedBox;

    #[test]
    fn to_from_inner_space() {
        let b = AxisAlignedBox::<Generic>::new(Point2::new(1.0, 4.0), Point2::new(2.0, 1.0));

        let point = Point2::new(1.3, 1.4);
        let inner = b.to_inner_space(point);
        let p_back = b.from_inner_space(inner);
        assert!((point - p_back).length() < 0.01);
    }
}

/// Describes all needed information
/// to go from a transform back to
/// layout properties
/// TODO is this behaviour correct? Might want to
/// always return correct values, that can then be handled during write
/// NOTE:
/// - If an optional unit is set to some, the returned
///   property struct might still return none in the
///   case that the computed new value is the same as the default
///   value for that parameter
///   if an optional value is none, this parameter will NEVER
///   be returned as some, this means the overall transform is not
///   recoverable from this again
pub(crate) struct InversionConfiguration {
    // Actual data needed about object
    pub(crate) anchor_x: Option<Size>,
    pub(crate) anchor_y: Option<Size>,
    pub(crate) container_bounds: (f64, f64),
    // Configuration values needed for what units to output
    pub(crate) unit_width: SizeUnit,
    pub(crate) unit_height: SizeUnit,
    pub(crate) unit_rotation: RotationUnit,
    pub(crate) unit_skew_x: RotationUnit,
    pub(crate) unit_x_pos: SizeUnit,
    pub(crate) unit_y_pos: SizeUnit,
}

pub enum ScaleOrDimFixed {
    Scale {
        fixed_scale: Percent,
        dim_unit: SizeUnit,
    },
    Dim(Size),
}

// This inversion method goes from:
// * InversionConfiguration - this objects old property values
// * TransformAndBounds - describing the new targeted area for this object
// to:
// * LayoutProperties - x, y, width, scale, skew, etc. that can we written
// to ORM, to get an identical bounding box to TransformAndBounds
// NOTE: this inverts the operations specified in: pax_runtime/src/layout.rs,
// function calculate_transform_and_bounds
pub(crate) fn transform_and_bounds_inversion(
    inv_config: InversionConfiguration,
    target_box: TransformAndBounds<NodeLocal, World>,
) -> LayoutProperties {
    let parts: Parts = target_box.transform.into();
    let object_bounds = target_box.bounds;
    let container_bounds = inv_config.container_bounds;

    #[allow(non_snake_case)]
    let A = target_box.transform.coeffs();
    #[allow(non_snake_case)]
    let M = [
        // All transformation coefficients
        // not related to translation,
        // (skew, scale, rotation),
        // to be used to figure out anchor point
        [A[0], A[2]],
        [A[1], A[3]],
    ];
    let dx = parts.origin.x;
    let dy = parts.origin.y;
    // width ratio
    let w_r = object_bounds.0 / container_bounds.0;
    // height ratio
    let h_r = object_bounds.1 / container_bounds.1;

    // The code below is solving the system of equations for x and y:

    // p = d + Ma
    // where:
    // p = [x, y]^T
    // d = [dx , dy]^T
    // M = skew, scale, rotation components of TransformAndBounds
    // a = [ax, ay]^T <-- anchor points

    // If x or y is in pixels, but anchor isn't set,
    // anchor is defaulted to 0%
    let anchor_x = inv_config
        .anchor_x
        .or((inv_config.unit_x_pos == SizeUnit::Pixels).then_some(Size::ZERO()));
    let anchor_y = inv_config
        .anchor_y
        .or((inv_config.unit_y_pos == SizeUnit::Pixels).then_some(Size::ZERO()));

    // for the four different cases of ax and ay
    // either being a function of x/y, or being "constants".
    // (see annotation of each case)
    let (x, y) = match (anchor_x, anchor_y) {
        // ax = w*x, ay = h*y
        (None, None) => {
            let denom =
                -h_r * w_r * M[0][1] * M[1][0] + (1.0 - h_r * M[1][1]) * (1.0 - w_r * M[0][0]);
            let x = (dx * (1.0 - h_r * M[1][1]) + dy * h_r * M[0][1]) / denom;
            let y = (dy * (1.0 - w_r * M[0][0]) + dx * w_r * M[1][0]) / denom;
            (x, y)
        }
        // ax = w*x, ay fixed
        (None, Some(anchor_y)) => {
            let ay = anchor_y.evaluate(object_bounds, Axis::Y);
            let x = (dx + M[0][1] * ay) / (1.0 - M[0][0] * w_r);
            let y = dy + M[1][1] * ay + M[1][0] * w_r * x;
            (x, y)
        }
        // ax fixed, ay = h*y
        (Some(anchor_x), None) => {
            let ax = anchor_x.evaluate(object_bounds, Axis::X);
            let y = (dy + M[1][0] * ax) / (1.0 - M[1][1] * h_r);
            let x = dx + M[0][0] * ax + M[0][1] * h_r * y;
            (x, y)
        }
        // ax and ay fixed
        (Some(anchor_x), Some(anchor_y)) => {
            let ax = anchor_x.evaluate(object_bounds, Axis::X);
            let ay = anchor_y.evaluate(object_bounds, Axis::Y);
            let x = dx + ax * M[0][0] + ay * M[0][1];
            let y = dy + ax * M[1][0] + ay * M[1][1];
            (x, y)
        }
    };

    // use config units for all values

    let width = match inv_config.unit_width {
        SizeUnit::Pixels => Size::Pixels(object_bounds.0.into()),
        SizeUnit::Percent => Size::Percent((100.0 * w_r).into()),
    };
    let height = match inv_config.unit_height {
        SizeUnit::Pixels => Size::Pixels(object_bounds.1.into()),
        SizeUnit::Percent => Size::Percent((100.0 * h_r).into()),
    };

    let scale_x = Percent((100.0 * parts.scale.x).into());
    let scale_y = Percent((100.0 * parts.scale.y).into());

    let x = match inv_config.unit_x_pos {
        SizeUnit::Pixels => Size::Pixels(x.into()),
        SizeUnit::Percent => Size::Percent((100.0 * x / container_bounds.0).into()),
    };
    let y = match inv_config.unit_y_pos {
        SizeUnit::Pixels => Size::Pixels(y.into()),
        SizeUnit::Percent => Size::Percent((100.0 * y / container_bounds.1).into()),
    };

    let rotation = match inv_config.unit_rotation {
        RotationUnit::Radians => Rotation::Radians(parts.rotation.into()),
        RotationUnit::Degrees => Rotation::Degrees(parts.rotation.to_degrees().into()),
        RotationUnit::Percent => Rotation::Percent((100.0 * parts.rotation / 2.0 / PI).into()),
    };

    let skew_x = Some(match inv_config.unit_skew_x {
        RotationUnit::Radians => Rotation::Radians(parts.skew.x.into()),
        RotationUnit::Degrees => Rotation::Degrees(parts.skew.x.to_degrees().into()),
        RotationUnit::Percent => Rotation::Percent((100.0 * parts.skew.x / 2.0 / PI).into()),
    });

    // First assume everything is in pixels, then after that
    // convert into percent using bounds if the old property value was of that type
    LayoutProperties {
        x: Some(x),
        y: Some(y),
        width: Some(width),
        height: Some(height),
        anchor_x,
        anchor_y,
        // TODO
        rotate: Some(rotation),
        scale_x: Some(scale_x),
        scale_y: Some(scale_y),
        // TODO return the skew
        skew_x,
        skew_y: Some(Rotation::Radians(0.0.into())),
    }
}
