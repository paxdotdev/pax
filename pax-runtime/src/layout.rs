use std::ops::Mul;

#[allow(unused)]
use pax_runtime_api::math::Generic;
use pax_runtime_api::math::{Point2, Space, TransformParts};
use pax_runtime_api::{Interpolatable, Percent, Property, Rotation, Window};

use crate::api::math::{Transform2, Vector2};
use crate::api::{Axis, Padding, Size, Transform2D};
use crate::node_interface::NodeLocal;
use crate::ContainerFrame;

/// Compute a reactive `TransformAndBounds` property from layout properties plus parent geometry.
pub fn compute_tab(
    layout_properties: Property<LayoutProperties>,
    extra_transform: Property<Option<Transform2D>>,
    container_transform_and_bounds: Property<TransformAndBounds<NodeLocal, Window>>,
) -> Property<TransformAndBounds<NodeLocal, Window>> {
    // get the size of this node (calc'd or otherwise) and use
    // it as the new accumulated bounds: both for this node's children (their parent container bounds)
    // and for this node itself (e.g. for specifying the size of a Rectangle node)
    let deps = [
        layout_properties.untyped(),
        container_transform_and_bounds.untyped(),
        extra_transform.untyped(),
    ];

    Property::computed(
        move || {
            let container_t_and_b = container_transform_and_bounds.get();
            layout_properties.read(|layout_properties| {
                let transform_and_bounds =
                    calculate_transform_and_bounds(&layout_properties, container_t_and_b.clone());
                let extra_transform = extra_transform.get();
                if let Some(transform) = extra_transform {
                    transform.apply(transform_and_bounds)
                } else {
                    transform_and_bounds
                }
            })
        },
        &deps,
    )
}

/// Apply a container-assigned child frame on top of the parent geometry.
///
/// This is the geometry seam where container-owned placement can cooperate
/// with descendant-authored layout and future bottom-up measurement.
pub fn apply_container_frame(
    container_transform_and_bounds: TransformAndBounds<NodeLocal, Window>,
    container_frame: Option<ContainerFrame>,
) -> TransformAndBounds<NodeLocal, Window> {
    match container_frame {
        Some(frame) => TransformAndBounds {
            transform: container_transform_and_bounds.transform * frame.transform,
            bounds: frame.bounds,
        },
        None => container_transform_and_bounds,
    }
}

/// Apply a node's padding to the container geometry seen by its children.
pub fn apply_padding_frame(
    container_transform_and_bounds: TransformAndBounds<NodeLocal, Window>,
    padding: Option<Padding>,
) -> TransformAndBounds<NodeLocal, Window> {
    match padding {
        Some(padding) => {
            let (padding_x, padding_y) = padding.evaluate(container_transform_and_bounds.bounds);
            TransformAndBounds {
                transform: container_transform_and_bounds.transform
                    * Transform2::translate(Vector2::new(padding_x, padding_y)),
                bounds: (
                    (container_transform_and_bounds.bounds.0 - (2.0 * padding_x)).max(0.0),
                    (container_transform_and_bounds.bounds.1 - (2.0 * padding_y)).max(0.0),
                ),
            }
        }
        None => container_transform_and_bounds,
    }
}

/// Solve an autosized outer axis from measured content and symmetric padding.
pub fn resolve_padded_autosize_axis(content_extent: f64, padding: Option<Size>) -> Option<f64> {
    let Some(padding) = padding else {
        return Some(content_extent.max(0.0));
    };

    let (pixel_component, percent_component) = match padding {
        Size::Pixels(pixels) => (pixels.to_float().max(0.0), 0.0),
        Size::Percent(percent) => (0.0, percent.to_float().max(0.0) / 100.0),
        Size::Combined(pixels, percent) => (
            pixels.to_float().max(0.0),
            percent.to_float().max(0.0) / 100.0,
        ),
    };

    let denominator = 1.0 - (2.0 * percent_component);
    (denominator > f64::EPSILON)
        .then_some(((content_extent.max(0.0) + (2.0 * pixel_component)) / denominator).max(0.0))
}

/// Per-axis local extents contributed by a node subtree for container measurement.
///
/// Coordinates are expressed in the node's local layout space. Validity is tracked
/// per axis so parent-dependent axes can be ignored without discarding the entire
/// subtree.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LayoutHull {
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
    pub valid_x: bool,
    pub valid_y: bool,
}

impl Interpolatable for LayoutHull {}

impl LayoutHull {
    pub fn from_bounds(bounds: (f64, f64)) -> Self {
        Self::from_axis_ranges(Some((0.0, bounds.0)), Some((0.0, bounds.1)))
    }

    pub fn from_axis_ranges(x: Option<(f64, f64)>, y: Option<(f64, f64)>) -> Self {
        let (min_x, max_x, valid_x) = match x {
            Some((min_x, max_x)) => {
                let (min_x, max_x) = order_pair(min_x, max_x);
                (min_x, max_x, true)
            }
            None => (0.0, 0.0, false),
        };
        let (min_y, max_y, valid_y) = match y {
            Some((min_y, max_y)) => {
                let (min_y, max_y) = order_pair(min_y, max_y);
                (min_y, max_y, true)
            }
            None => (0.0, 0.0, false),
        };
        Self {
            min_x,
            max_x,
            min_y,
            max_y,
            valid_x,
            valid_y,
        }
    }

    pub fn x_range(&self) -> Option<(f64, f64)> {
        self.valid_x.then_some((self.min_x, self.max_x))
    }

    pub fn y_range(&self) -> Option<(f64, f64)> {
        self.valid_y.then_some((self.min_y, self.max_y))
    }

    pub fn union(self, other: Self) -> Self {
        Self::from_axis_ranges(
            union_axis_ranges(self.x_range(), other.x_range()),
            union_axis_ranges(self.y_range(), other.y_range()),
        )
    }

    pub fn forward_extent_x(&self) -> Option<f64> {
        self.x_range().map(|(_, max_x)| max_x.max(0.0))
    }

    pub fn forward_extent_y(&self) -> Option<f64> {
        self.y_range().map(|(_, max_y)| max_y.max(0.0))
    }
}

/// Project a local layout hull through the provided transform and return the
/// axis-aligned hull in the destination coordinate space.
pub fn project_layout_hull<F: Space, T: Space>(
    transform: Transform2<F, T>,
    hull: LayoutHull,
) -> LayoutHull {
    let mixes_axes = transform.m[1].abs() > f64::EPSILON || transform.m[2].abs() > f64::EPSILON;
    if hull.valid_x && hull.valid_y {
        let corners = [
            transform * Point2::new(hull.min_x, hull.min_y),
            transform * Point2::new(hull.min_x, hull.max_y),
            transform * Point2::new(hull.max_x, hull.min_y),
            transform * Point2::new(hull.max_x, hull.max_y),
        ];
        let (min_x, max_x) = corners.iter().fold(
            (f64::INFINITY, f64::NEG_INFINITY),
            |(min_x, max_x), point| (min_x.min(point.x), max_x.max(point.x)),
        );
        let (min_y, max_y) = corners.iter().fold(
            (f64::INFINITY, f64::NEG_INFINITY),
            |(min_y, max_y), point| (min_y.min(point.y), max_y.max(point.y)),
        );
        return LayoutHull::from_axis_ranges(Some((min_x, max_x)), Some((min_y, max_y)));
    }

    if mixes_axes {
        return LayoutHull::default();
    }

    let x = hull
        .x_range()
        .map(|(min_x, max_x)| project_axis_range(min_x, max_x, transform.m[0], transform.m[4]));
    let y = hull
        .y_range()
        .map(|(min_y, max_y)| project_axis_range(min_y, max_y, transform.m[3], transform.m[5]));
    LayoutHull::from_axis_ranges(x, y)
}

/// Project a child's local hull into its parent's local layout space.
pub fn project_child_layout_hull_to_parent_space(
    parent: TransformAndBounds<NodeLocal, Window>,
    child: TransformAndBounds<NodeLocal, Window>,
    child_hull: LayoutHull,
) -> LayoutHull {
    let relative_transform = parent.transform.inverse() * child.transform;
    project_layout_hull(relative_transform, child_hull)
}

fn order_pair(a: f64, b: f64) -> (f64, f64) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

fn union_axis_ranges(lhs: Option<(f64, f64)>, rhs: Option<(f64, f64)>) -> Option<(f64, f64)> {
    match (lhs, rhs) {
        (Some((lhs_min, lhs_max)), Some((rhs_min, rhs_max))) => {
            Some((lhs_min.min(rhs_min), lhs_max.max(rhs_max)))
        }
        (Some(range), None) | (None, Some(range)) => Some(range),
        (None, None) => None,
    }
}

fn project_axis_range(min: f64, max: f64, scale: f64, translate: f64) -> (f64, f64) {
    order_pair(scale * min + translate, scale * max + translate)
}

/// Resolve one set of layout properties into concrete bounds and a window-space transform.
pub fn calculate_transform_and_bounds(
    LayoutProperties {
        width,
        height,
        anchor_x,
        anchor_y,
        x,
        y,
        rotate,
        scale_x,
        scale_y,
        skew_x,
        skew_y,
    }: &LayoutProperties,
    TransformAndBounds {
        transform: container_transform,
        bounds: container_bounds,
    }: TransformAndBounds<NodeLocal, Window>,
) -> TransformAndBounds<NodeLocal, Window> {
    let x = x.unwrap_or(Size::ZERO());
    let y = y.unwrap_or(Size::ZERO());
    let width = width
        .map(|v| v.evaluate(container_bounds, Axis::X))
        .unwrap_or(container_bounds.0);
    let height = height
        .map(|v| v.evaluate(container_bounds, Axis::Y))
        .unwrap_or(container_bounds.1);
    let origin = Vector2::new(
        x.evaluate(container_bounds, Axis::X),
        y.evaluate(container_bounds, Axis::Y),
    );

    let bounds = (width, height);

    // Anchor behavior:  if no anchor is specified and if x/y values are present
    // and have an explicit percent value or component, use those percent values (clamp 100% and 0%)
    // otherwise, default to 0
    let anchor_x = anchor_x.unwrap_or_else(|| match x {
        Size::Pixels(_) => Size::ZERO(),
        Size::Combined(_, per) | Size::Percent(per) => {
            if per.to_float() > 100.0 {
                Size::default()
            } else if per.to_float() < 0.0 {
                Size::ZERO()
            } else {
                Size::Percent(per)
            }
        }
    });

    let anchor_y = anchor_y.unwrap_or_else(|| match y {
        Size::Pixels(_) => Size::ZERO(),
        Size::Combined(_, per) | Size::Percent(per) => {
            if per.to_float() > 100.0 {
                Size::default()
            } else if per.to_float() < 0.0 {
                Size::ZERO()
            } else {
                Size::Percent(per)
            }
        }
    });

    let anchor_transform = Transform2::translate(Vector2::new(
        -anchor_x.evaluate(bounds, Axis::X),
        -anchor_y.evaluate(bounds, Axis::Y),
    ));

    let scale = Vector2::new(
        scale_x
            .as_ref()
            .map(|s| s.0.to_float() / 100.0)
            .unwrap_or(1.0),
        scale_y
            .as_ref()
            .map(|s| s.0.to_float() / 100.0)
            .unwrap_or(1.0),
    );

    let skew = Vector2::new(
        skew_x.map(|s| s.get_as_radians()).unwrap_or(0.0),
        skew_y.map(|s| s.get_as_radians()).unwrap_or(0.0),
    );

    let rotation = rotate.map(|s| s.get_as_radians()).unwrap_or(0.0);

    let parts = TransformParts {
        origin,
        scale,
        skew,
        rotation,
    };

    let combined_transform: Transform2<NodeLocal, NodeLocal> = parts.into();

    TransformAndBounds {
        transform: container_transform * combined_transform * anchor_transform,
        bounds,
    }
}

impl<F: Space, T: Space> Interpolatable for TransformAndBounds<F, T> {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        TransformAndBounds {
            transform: self.transform.interpolate(&other.transform, t),
            bounds: (
                self.bounds.0 + (other.bounds.0 - self.bounds.0) * t,
                self.bounds.1 + (other.bounds.1 - self.bounds.1) * t,
            ),
        }
    }
}

/// Pax's canonical representation of position, size, and transform, encoded
/// as a transform (translation, rotation, scale, skew) and a separate width/height (bounds) value.
/// Bounds are expressed as the (x1, y1) values of the axis-aligned pre-transform bounding box,
/// where (x0, y0) are the origin.
///
/// In this model, position is a derived property, calculated by applying the transform to the bounding box.
#[derive(PartialEq)]
pub struct TransformAndBounds<F, T = F> {
    pub transform: Transform2<F, T>,
    pub bounds: (f64, f64),
}

impl<F: Space, T: Space> std::fmt::Debug for TransformAndBounds<F, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransformAndBounds")
            .field("transform", &self.transform)
            .field("bounds", &self.bounds)
            .finish()
    }
}

impl PartialEq for TransformAndBounds<NodeLocal, Window> {
    fn eq(&self, other: &Self) -> bool {
        self.transform == other.transform && self.bounds == other.bounds
    }
}

impl<F: Space, T: Space> Default for TransformAndBounds<F, T> {
    fn default() -> Self {
        Self {
            transform: Default::default(),
            bounds: (100.0, 100.0),
        }
    }
}

impl<F, T> Clone for TransformAndBounds<F, T> {
    fn clone(&self) -> Self {
        Self {
            transform: self.transform.clone(),
            bounds: self.bounds.clone(),
        }
    }
}

impl<F, T> Copy for TransformAndBounds<F, T> {}

impl<W1: Space, W2: Space, W3: Space> Mul<TransformAndBounds<W1, W2>>
    for TransformAndBounds<W2, W3>
{
    type Output = TransformAndBounds<W1, W3>;

    // - if T(M) = M.transform * Transform::scale_sep(M.bounds.0, M.bounds.1),
    //   then T(A) * T(B) = T(A*B).
    // (
    // - some other rule regarding multiplying scaling and width/height values being constant,
    //   related to how width/height scale x/y change for A or B, how that affects A*B.
    // TODO figure this rule out, this is that would fix resize skew introducing un-needed scaling
    // (compare with figma)
    //)
    fn mul(self, rhs: TransformAndBounds<W1, W2>) -> Self::Output {
        let s_s = Transform2::scale_sep(Vector2::new(self.bounds.0, self.bounds.1));
        let r_s = Transform2::scale_sep(Vector2::new(rhs.bounds.0, rhs.bounds.1));

        let s_t = self.transform * s_s;
        let r_t = rhs.transform * r_s;
        let res = s_t * r_t * s_s.inverse() * r_s.inverse();

        TransformAndBounds {
            transform: res,
            bounds: (self.bounds.0 * rhs.bounds.0, self.bounds.1 * rhs.bounds.1),
        }
    }
}

impl<F: Space, T: Space> TransformAndBounds<F, T> {
    /// Center point of this transformed box.
    pub fn center(&self) -> Point2<T> {
        let (o, u, v) = self.transform.decompose();
        let u = u * self.bounds.0;
        let v = v * self.bounds.1;
        o + v / 2.0 + u / 2.0
    }

    /// Corners of this transformed box, starting at origin and proceeding around the rectangle.
    pub fn corners(&self) -> [Point2<T>; 4] {
        let (o, u, v) = self.transform.decompose();
        let u = u * self.bounds.0;
        let v = v * self.bounds.1;
        [o, o + v, o + u + v, o + u]
    }

    /// Test whether a point falls inside this transformed box.
    pub fn contains_point(&self, point: Point2<T>) -> bool {
        self.as_transform().contains_point(point)
    }

    /// Move scale from the transform into the bounds field.
    pub fn as_pure_size(self) -> Self {
        let mut parts: TransformParts = self.transform.into();
        let bounds_x = std::mem::replace(&mut parts.scale.x, 1.0);
        let bounds_y = std::mem::replace(&mut parts.scale.y, 1.0);
        TransformAndBounds {
            transform: parts.into(),
            bounds: (self.bounds.0 * bounds_x, self.bounds.1 * bounds_y),
        }
    }
    /// Move bounds into the transform as scale, leaving unit bounds.
    pub fn as_pure_scale(self) -> Self {
        TransformAndBounds {
            transform: self.transform
                * Transform2::scale_sep(Vector2::new(self.bounds.0, self.bounds.1)),
            bounds: (1.0, 1.0),
        }
    }

    /// Retype coordinate-space markers without changing numeric values.
    pub fn cast_spaces<A: Space, B: Space>(self) -> TransformAndBounds<A, B> {
        TransformAndBounds {
            transform: self.transform.cast_spaces(),
            bounds: self.bounds,
        }
    }

    /// Convert this split representation into a single affine transform.
    pub fn as_transform(&self) -> Transform2<F, T> {
        self.transform * Transform2::scale_sep(Vector2::new(self.bounds.0, self.bounds.1))
    }
}

#[test]
fn test_transform_and_bounds_mult() {
    let dvx = 0.6;
    let dvy = 0.3;

    let t_and_b_with_scale = TransformAndBounds::<Generic> {
        transform: Transform2::new([1.5, 1.1, 2.3, 3.2, 1.2, 1.0])
            * Transform2::scale_sep(Vector2::<Generic>::new(dvx, dvy)),
        bounds: (2.1, 1.2),
    };

    let t_and_b_with_size = TransformAndBounds::<Generic> {
        transform: Transform2::new([1.5, 1.1, 2.3, 3.2, 1.2, 1.0]),
        bounds: (2.1 * dvx, 1.2 * dvy),
    };
    let some_other_transform = TransformAndBounds::<Generic> {
        transform: Transform2::new([1.1, 1.2, 5.3, 9.2, 1.0, 2.0]),
        bounds: (1.9, 4.5),
    };

    let res_scale = t_and_b_with_scale * some_other_transform;
    let res_size = t_and_b_with_size * some_other_transform;

    let t_scale = res_scale.transform
        * Transform2::<Generic>::scale_sep(Vector2::new(res_scale.bounds.0, res_scale.bounds.1));
    let t_scale_c = t_scale.coeffs();
    let t_size = res_size.transform
        * Transform2::<Generic>::scale_sep(Vector2::new(res_size.bounds.0, res_size.bounds.1));
    let t_size_c = t_size.coeffs();
    let diff_sum = t_scale_c
        .iter()
        .zip(t_size_c)
        .map(|(a, b)| (a - b).abs())
        .sum::<f64>();
    assert!(diff_sum < 1e-4);

    let res_scale_other_way = some_other_transform * t_and_b_with_scale;
    let res_size_other_way = some_other_transform * t_and_b_with_size;

    let t_scale = res_scale_other_way.transform
        * Transform2::<Generic>::scale_sep(Vector2::new(
            res_scale_other_way.bounds.0,
            res_scale_other_way.bounds.1,
        ));
    let t_scale_c = t_scale.coeffs();
    let t_size = res_size_other_way.transform
        * Transform2::<Generic>::scale_sep(Vector2::new(
            res_size_other_way.bounds.0,
            res_size_other_way.bounds.1,
        ));
    let t_size_c = t_size.coeffs();
    let diff_sum = t_scale_c
        .iter()
        .zip(t_size_c)
        .map(|(a, b)| (a - b).abs())
        .sum::<f64>();
    assert!(diff_sum < 1e-4);
}

#[test]
fn test_apply_container_frame_uses_assigned_bounds() {
    let parent = TransformAndBounds::<NodeLocal, Window> {
        transform: Transform2::translate(Vector2::new(50.0, 60.0)),
        bounds: (100.0, 100.0),
    };
    let frame = ContainerFrame {
        transform: Transform2::translate(Vector2::new(10.0, 20.0)),
        bounds: (30.0, 40.0),
    };

    let result = apply_container_frame(parent, Some(frame));

    assert_eq!(result.bounds, (30.0, 40.0));
    assert_eq!(result.transform.m[4], 60.0);
    assert_eq!(result.transform.m[5], 80.0);
}

#[test]
fn test_apply_padding_frame_shrinks_and_offsets_child_bounds() {
    let parent = TransformAndBounds::<NodeLocal, Window> {
        transform: Transform2::translate(Vector2::new(50.0, 60.0)),
        bounds: (200.0, 100.0),
    };

    let result = apply_padding_frame(
        parent,
        Some(Padding::axes(
            Size::Pixels(10.into()),
            Size::Percent(20.into()),
        )),
    );

    assert_eq!(result.bounds, (180.0, 60.0));
    assert_eq!(result.transform.m[4], 60.0);
    assert_eq!(result.transform.m[5], 80.0);
}

#[test]
fn test_resolve_padded_autosize_axis_solves_percent_padding() {
    let resolved = resolve_padded_autosize_axis(60.0, Some(Size::Percent(20.into()))).unwrap();

    assert!((resolved - 100.0).abs() < 1e-9);
}

#[test]
fn test_project_layout_hull_with_translation_preserves_forward_extents() {
    let hull = LayoutHull::from_axis_ranges(Some((-10.0, 40.0)), Some((0.0, 20.0)));
    let projected = project_layout_hull(
        Transform2::<Generic>::translate(Vector2::new(30.0, -5.0)),
        hull,
    );

    assert_eq!(projected.x_range(), Some((20.0, 70.0)));
    assert_eq!(projected.y_range(), Some((-5.0, 15.0)));
    assert_eq!(projected.forward_extent_x(), Some(70.0));
    assert_eq!(projected.forward_extent_y(), Some(15.0));
}

#[test]
fn test_project_layout_hull_keeps_axis_independence_without_cross_axis_mixing() {
    let hull = LayoutHull::from_axis_ranges(Some((10.0, 30.0)), None);
    let projected = project_layout_hull(
        Transform2::<Generic>::new([-2.0, 0.0, 0.0, 3.0, 5.0, 7.0]),
        hull,
    );

    assert_eq!(projected.x_range(), Some((-55.0, -15.0)));
    assert_eq!(projected.y_range(), None);
}

#[test]
fn test_project_layout_hull_invalidates_partial_axes_when_transform_mixes_axes() {
    let hull = LayoutHull::from_axis_ranges(Some((0.0, 20.0)), None);
    let projected = project_layout_hull(
        Transform2::<Generic>::rotate(std::f64::consts::FRAC_PI_4),
        hull,
    );

    assert_eq!(projected, LayoutHull::default());
}

impl Interpolatable for LayoutProperties {}

#[derive(Debug, Default, Clone)]
/// Unresolved layout inputs copied out of common properties before geometry calculation.
pub struct LayoutProperties {
    pub x: Option<Size>,
    pub y: Option<Size>,
    pub width: Option<Size>,
    pub height: Option<Size>,
    pub rotate: Option<Rotation>,
    pub scale_x: Option<Percent>,
    pub scale_y: Option<Percent>,
    pub anchor_x: Option<Size>,
    pub anchor_y: Option<Size>,
    pub skew_x: Option<Rotation>,
    pub skew_y: Option<Rotation>,
}

impl LayoutProperties {
    /// Full-size defaults for nodes that should fill their containing bounds.
    pub fn fill() -> Self {
        Self {
            x: Some(Size::ZERO()),
            y: Some(Size::ZERO()),
            width: Some(Size::default()),
            height: Some(Size::default()),
            rotate: Some(Rotation::ZERO()),
            scale_x: Some(Percent(100.into())),
            scale_y: Some(Percent(100.into())),
            anchor_x: None,
            anchor_y: None,
            skew_x: Some(Rotation::ZERO()),
            skew_y: Some(Rotation::ZERO()),
        }
    }
}

impl<F: Space, T: Space> TransformAndBounds<F, T> {
    /// Invert the transform-and-bounds mapping.
    pub fn inverse(&self) -> TransformAndBounds<T, F> {
        let t_inv = self.transform.inverse();
        let b_inv = (1.0 / self.bounds.0, 1.0 / self.bounds.1);
        TransformAndBounds {
            transform: t_inv,
            bounds: b_inv,
        }
    }

    /// Test transformed-box intersection using the separating axis theorem.
    pub fn intersects(&self, other: &Self) -> bool {
        let corners_self = self.corners();
        let corners_other = other.corners();

        for i in 0..2 {
            let axis = (corners_self[i] - corners_self[(i + 1) % 4]).normal();

            let self_projections: Vec<_> = corners_self
                .iter()
                .map(|&p| p.to_vector().project_onto(axis).length())
                .collect();
            let other_projections: Vec<_> = corners_other
                .iter()
                .map(|&p| p.to_vector().project_onto(axis).length())
                .collect();

            let (min_self, max_self) = min_max_projections(&self_projections);
            let (min_other, max_other) = min_max_projections(&other_projections);

            // Check for non-overlapping projections
            if max_self < min_other || max_other < min_self {
                // By the separating axis theorem, non-overlap of projections on _any one_ of the axis-normals proves that these polygons do not intersect.
                return false;
            }
        }
        true
    }
}

fn min_max_projections(projections: &[f64]) -> (f64, f64) {
    let min_projection = *projections.iter().min_by(|a, b| a.total_cmp(b)).unwrap();
    let max_projection = *projections.iter().max_by(|a, b| a.total_cmp(b)).unwrap();
    (min_projection, max_projection)
}

pub trait ComputableTransform<F, T> {
    fn apply(&self, bounds: TransformAndBounds<F, T>) -> TransformAndBounds<F, T>;
}

impl ComputableTransform<NodeLocal, Window> for Transform2D {
    fn apply(
        &self,
        bounds: TransformAndBounds<NodeLocal, Window>,
    ) -> TransformAndBounds<NodeLocal, Window> {
        let layout_properties = LayoutProperties {
            x: self.translate.map(|v| v[0]),
            y: self.translate.map(|v| v[1]),
            width: Some(Size::Pixels(bounds.bounds.0.into())),
            height: Some(Size::Pixels(bounds.bounds.1.into())),
            rotate: self.rotate.clone(),
            scale_x: self
                .scale
                .as_ref()
                .map(|v| Percent((100.0 * v[0].clone().expect_percent()).into())),
            scale_y: self
                .scale
                .as_ref()
                .map(|v| Percent((100.0 * v[1].clone().expect_percent()).into())),
            anchor_x: self.anchor.map(|v| v[0]),
            anchor_y: self.anchor.map(|v| v[1]),
            skew_x: self.skew.as_ref().map(|v| v[0].clone()),
            skew_y: self.skew.as_ref().map(|v| v[1].clone()),
        };

        let curr = calculate_transform_and_bounds(&layout_properties, bounds.clone());
        match &self.previous {
            Some(previous) => (*previous).apply(curr),
            None => curr,
        }
    }
}
