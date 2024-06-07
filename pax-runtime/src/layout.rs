use std::ops::Mul;

use pax_runtime_api::math::{Parts, Point2, Space};
use pax_runtime_api::{Interpolatable, Percent, Property, Rotation, Window};

use crate::api::math::{Transform2, Vector2};
use crate::api::{Axis, Size, Transform2D};
use crate::node_interface::NodeLocal;

/// For the `current_expanded_node` attached to `ptc`, calculates and returns a new [`crate::rendering::TransformAndBounds`] a.k.a. "tab".
/// Intended as a helper method to be called during properties computation, for creating a new tab to attach to `ptc` for downstream calculations.
pub fn compute_tab(
    layout_properties: Property<LayoutProperties>,
    extra_transform: Property<Option<Transform2D>>,
    container_transform_and_bounds: Property<TransformAndBounds<NodeLocal, Window>>,
) -> Property<TransformAndBounds<NodeLocal, Window>> {
    //get the size of this node (calc'd or otherwise) and use
    //it as the new accumulated bounds: both for this node's children (their parent container bounds)
    //and for this node itself (e.g. for specifying the size of a Rectangle node)

    let deps = [
        layout_properties.untyped(),
        container_transform_and_bounds.untyped(),
        extra_transform.untyped(),
    ];

    Property::computed(
        move || {
            let container_t_and_b = container_transform_and_bounds.get();
            let transform_and_bounds =
                calculate_transform_and_bounds(layout_properties.get(), container_t_and_b.clone());
            let extra_transform = extra_transform.get();
            if let Some(transform) = extra_transform {
                transform.apply(transform_and_bounds)
            } else {
                transform_and_bounds
            }
        },
        &deps,
    )
}

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
    }: LayoutProperties,
    TransformAndBounds {
        transform: container_transform,
        bounds: container_bounds,
    }: TransformAndBounds<NodeLocal, Window>,
) -> TransformAndBounds<NodeLocal, Window> {
    let width = width
        .map(|v| v.evaluate(container_bounds, Axis::X))
        .unwrap_or(container_bounds.0);
    let height = height
        .map(|v| v.evaluate(container_bounds, Axis::Y))
        .unwrap_or(container_bounds.1);

    let bounds = (width, height);

    let x = x.unwrap_or(Size::ZERO());
    let y = y.unwrap_or(Size::ZERO());
    let [anchor_x, anchor_y] = get_position_adjusted_anchor(anchor_x, anchor_y, x, y);
    let anchor_transform = Transform2::translate(Vector2::new(
        -anchor_x.evaluate(bounds, Axis::X),
        -anchor_y.evaluate(bounds, Axis::Y),
    ));

    let origin = Vector2::new(
        x.evaluate(container_bounds, Axis::X),
        y.evaluate(container_bounds, Axis::Y),
    );

    let scale = Vector2::new(
        scale_x.map(|s| s.0.to_float() / 100.0).unwrap_or(1.0),
        scale_y.map(|s| s.0.to_float() / 100.0).unwrap_or(1.0),
    );

    let skew = Vector2::new(
        skew_x.map(|s| s.get_as_radians()).unwrap_or(0.0),
        skew_y.map(|s| s.get_as_radians()).unwrap_or(0.0),
    );

    let rotation = rotate.map(|s| s.get_as_radians()).unwrap_or(0.0);

    let parts = Parts {
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

//Anchor behavior:
//  if no anchor is specified:
//     if x/y values are present and have an explicit percent value or component, use those percent values
//     otherwise, default to 0
pub fn get_position_adjusted_anchor(
    anchor_x: Option<Size>,
    anchor_y: Option<Size>,
    x: Size,
    y: Size,
) -> [Size; 2] {
    let anchor = [
        if let Some(val) = anchor_x {
            val
        } else {
            match x {
                Size::Pixels(_) => Size::ZERO(),
                Size::Percent(per) => Size::Percent(per),
                Size::Combined(_, per) => Size::Percent(per),
            }
        },
        if let Some(val) = anchor_y {
            val
        } else {
            match y {
                Size::Pixels(_) => Size::ZERO(),
                Size::Percent(per) => Size::Percent(per),
                Size::Combined(_, per) => Size::Percent(per),
            }
        },
    ];
    anchor
}

/// Properties that are currently re-computed each frame before rendering.

impl<F, T> Interpolatable for TransformAndBounds<F, T> {}

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

impl<W1: Space, W2: Space, W3: Space> Mul<TransformAndBounds<W1, W2>>
    for TransformAndBounds<W2, W3>
{
    type Output = TransformAndBounds<W1, W3>;

    fn mul(self, rhs: TransformAndBounds<W1, W2>) -> Self::Output {
        TransformAndBounds {
            transform: self.transform * rhs.transform,
            bounds: (self.bounds.0 * rhs.bounds.0, self.bounds.1 * rhs.bounds.1),
        }
    }
}

impl Interpolatable for LayoutProperties {}

#[derive(Debug, Default, Clone)]
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

impl<F: Space, T: Space> TransformAndBounds<F, T> {
    pub fn corners(&self) -> [Point2<T>; 4] {
        let (width, height) = self.bounds;

        let top_left = self.transform * Point2::new(0.0, 0.0);
        let top_right = self.transform * Point2::new(width, 0.0);
        let bottom_left = self.transform * Point2::new(0.0, height);
        let bottom_right = self.transform * Point2::new(width, height);

        let res = [top_left, top_right, bottom_right, bottom_left];
        res
    }

    //Applies the separating axis theorem to determine whether two `TransformAndBounds` intersect.
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
    let min_projection = *projections
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
    let max_projection = *projections
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();
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
            scale_x: self.scale.as_ref().map(|v| v[0].clone()),
            scale_y: self.scale.as_ref().map(|v| v[1].clone()),
            anchor_x: self.anchor.map(|v| v[0]),
            anchor_y: self.anchor.map(|v| v[1]),
            skew_x: self.skew.as_ref().map(|v| v[0].clone()),
            skew_y: self.skew.as_ref().map(|v| v[1].clone()),
        };

        let curr = calculate_transform_and_bounds(layout_properties, bounds.clone());
        match &self.previous {
            Some(previous) => (*previous).apply(curr),
            None => curr,
        }
    }
}
