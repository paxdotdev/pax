use std::ops::Mul;

use pax_engine::{
    math::{Generic, Parts, Point2, Space, Transform2, Vector2},
    NodeLocal, Properties,
};
use pax_runtime_api::{Axis, Interpolatable, Size};

use crate::math::coordinate_spaces::Glass;

#[derive(PartialEq)]
pub enum Unit {
    Pixels,
    Percent,
}

pub mod coordinate_spaces {

    use pax_engine::math;

    pub struct Glass;

    impl math::Space for Glass {}

    pub struct World;

    impl math::Space for World {}
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

// This inversion method goes from a general transform2d describing the bounding box, some optional parameters,
// and the old property values to the new property values if the object is moved to the new location.
// This needs to be updated whenever the layout calculation is updated in the engine.
// TODO expose method for layout calc in engine, right tests in the designer to make sure that this
// is doing inversion correctly.
pub(crate) fn transform_to_properties(
    bounds: (f64, f64),
    target_box: Transform2<Glass, NodeLocal>,
    old_props: &Properties,
) -> Properties {
    let parts: Parts = target_box.into();

    // TODO don't assume scale is always 1, instead accept boolean flag
    // if scale or width/height should be resized (probably config object to be able to freeze dims, etc)
    let width_px = parts.scale.x;
    let height_px = parts.scale.y;
    let width = old_props
        .width
        .map(|s| s.with_same_unit(bounds.0, width_px));
    let height = old_props
        .height
        .map(|s| s.with_same_unit(bounds.1, height_px));

    // TODO expose way to set anchor of object (target box not enough, maybe point relative to target box?)
    let anchor_x = old_props.anchor_x;
    let anchor_y = old_props.anchor_y;

    let dx = parts.origin.x;
    let dy = parts.origin.y;

    let x = if let Some(anchor_x) = old_props.anchor_x {
        dx + anchor_x.evaluate((width_px, height_px), Axis::X)
    } else {
        if old_props.x.is_some_and(|s| matches!(s, Size::Percent(_))) {
            // if anchor is not set to figure out the new "virtual"
            // anchor point based on wanted top left position and width/height.
            // (same thing as bellow is done for the y case)
            // equation for new position (since anchor depends on x, solving for x):
            // x = dx + (width/bounds.0)*x =>
            // x*(1 - (width/bounds.0)) = dx => (if width != bounds.0)
            // x = dx/(1 - (width/bounds.0))
            dx / (1.0 - (width_px / bounds.0))
        } else {
            dx
        }
    };
    let y = if let Some(anchor_y) = old_props.anchor_y {
        dy + anchor_y.evaluate((width_px, height_px), Axis::Y)
    } else {
        if old_props.y.is_some_and(|s| matches!(s, Size::Percent(_))) {
            // same thing here
            dy / (1.0 - (height_px / bounds.1))
        } else {
            dy
        }
    };
    let x = if x.is_normal() { x } else { 0.0 };
    let y = if y.is_normal() { y } else { 0.0 };

    // use same unit as old value
    let x = old_props.x.map(|s| s.with_same_unit(bounds.0, x));
    let y = old_props.y.map(|s| s.with_same_unit(bounds.1, y));

    // First assume everything is in pixels, then after that
    // convert into percent using bounds if the old property value was of that type
    // (TODO expose overrides for pix/perc)
    Properties {
        x,
        y,
        width,
        height,
        anchor_x,
        anchor_y,
        // TODO
        local_rotation: None,
        scale_x: None,
        scale_y: None,
        skew_x: None,
        skew_y: None,
    }
}

trait UpdateSize {
    fn with_same_unit(&self, dim: f64, val: f64) -> Self;
}

impl UpdateSize for Size {
    fn with_same_unit(&self, dim: f64, val: f64) -> Self {
        match self {
            Size::Pixels(_) => Size::Pixels(val.into()),
            Size::Percent(_) => Size::Percent((100.0 * val / dim).into()),
            Size::Combined(_, _) => panic!("can't update combined, is this an expression?"),
        }
    }
}
