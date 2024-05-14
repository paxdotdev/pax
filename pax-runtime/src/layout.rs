use std::rc::Rc;

use pax_runtime_api::{borrow, Numeric, Property, Window};

use crate::api::math::{Generic, Transform2, Vector2};
use crate::api::{Axis, Size, Transform2D};
use crate::node_interface::NodeLocal;
use crate::ExpandedNode;

/// For the `current_expanded_node` attached to `ptc`, calculates and returns a new [`crate::rendering::TransformAndBounds`] a.k.a. "tab".
/// Intended as a helper method to be called during properties computation, for creating a new tab to attach to `ptc` for downstream calculations.
pub fn compute_tab(
    node: &Rc<ExpandedNode>,
    container_transform: Property<Transform2<NodeLocal, Window>>,
    container_bounds: Property<(f64, f64)>,
) -> (
    Property<Transform2<NodeLocal, Window>>,
    Property<(f64, f64)>,
) {
    //get the size of this node (calc'd or otherwise) and use
    //it as the new accumulated bounds: both for this node's children (their parent container bounds)
    //and for this node itself (e.g. for specifying the size of a Rectangle node)

    let cp_container_bounds = container_bounds.clone();
    let common_props = node.get_common_properties();
    let common_props = borrow!(common_props);
    let cp_width = common_props.width.clone();
    let cp_height = common_props.height.clone();

    let deps = vec![
        container_bounds.untyped(),
        cp_width.untyped(),
        cp_height.untyped(),
    ];

    let bounds = Property::computed_with_name(
        move || {
            let p_bounds = cp_container_bounds.get();
            let width = cp_width.get().evaluate(p_bounds, Axis::X);
            let height = cp_height.get().evaluate(p_bounds, Axis::Y);
            (width, height)
        },
        &deps,
        &format!("bounds of node {}", node.id.0),
    );

    let cp_bounds = bounds.clone();
    let cp_transform = common_props.transform.clone();
    let cp_container_bounds = container_bounds.clone();
    let cp_x = common_props.x.clone();
    let cp_y = common_props.y.clone();
    let cp_anchor_x = common_props.anchor_x.clone();
    let cp_anchor_y = common_props.anchor_y.clone();
    let cp_scale_x = common_props.scale_x.clone();
    let cp_scale_y = common_props.scale_y.clone();
    let cp_skew_x = common_props.skew_x.clone();
    let cp_skew_y = common_props.skew_y.clone();
    let cp_rotate = common_props.rotate.clone();

    let size_props = [
        &cp_x,
        &cp_y,
        &cp_anchor_x,
        &cp_anchor_y,
        &cp_scale_x,
        &cp_scale_y,
    ]
    .map(|v| v.as_ref().map(|p| p.untyped()))
    .into_iter()
    .flatten();

    let other_props = [
        cp_skew_x.as_ref().map(|p| p.untyped()),
        cp_skew_y.as_ref().map(|p| p.untyped()),
        cp_rotate.as_ref().map(|p| p.untyped()),
    ]
    .into_iter()
    .flatten();

    let all_transform_deps: Vec<_> = size_props
        .chain(other_props)
        .chain(
            [
                cp_transform.untyped(),
                cp_bounds.untyped(),
                cp_container_bounds.untyped(),
                container_transform.untyped(),
            ]
            .into_iter(),
        )
        .collect();

    let transform = Property::computed_with_name(
        move || {
            let node_transform_property_computed = {
                cp_transform
                    .get()
                    .compute_transform2d_matrix(cp_bounds.get(), cp_container_bounds.get())
                    .cast_spaces::<NodeLocal, NodeLocal>()
            };
            // From a combination of the sugared TemplateNodeDefinition properties like `width`, `height`, `x`, `y`, `scale_x`, etc.
            let desugared_transform = {
                //Extract common_properties, pack into Transform2D, decompose / compute, and combine with node_computed_transform
                let mut desugared_transform2d = Transform2D::default();

                let translate = [
                    if let Some(ref val) = cp_x {
                        val.get().clone()
                    } else {
                        Size::ZERO()
                    },
                    if let Some(ref val) = cp_y {
                        val.get().clone()
                    } else {
                        Size::ZERO()
                    },
                ];
                desugared_transform2d.translate = Some(translate.clone());

                //Anchor behavior:
                //  if no anchor is specified:
                //     if x/y values are present and have an explicit percent value or component, use those percent values
                //     otherwise, default to 0
                let anchor = [
                    if let Some(ref val) = cp_anchor_x {
                        val.get().clone()
                    } else {
                        match translate[0] {
                            Size::Pixels(_) => Size::ZERO(),
                            Size::Percent(per) => Size::Percent(per),
                            Size::Combined(_, per) => Size::Percent(per),
                        }
                    },
                    if let Some(ref val) = cp_anchor_y {
                        val.get().clone()
                    } else {
                        match translate[1] {
                            Size::Pixels(_) => Size::ZERO(),
                            Size::Percent(per) => Size::Percent(per),
                            Size::Combined(_, per) => Size::Percent(per),
                        }
                    },
                ];
                desugared_transform2d.anchor = Some(anchor);

                let scale = [
                    if let Some(ref val) = cp_scale_x {
                        val.get().clone()
                    } else {
                        Size::Percent(Numeric::F64(100.0))
                    },
                    if let Some(ref val) = cp_scale_y {
                        val.get().clone()
                    } else {
                        Size::Percent(Numeric::F64(100.0))
                    },
                ];
                desugared_transform2d.scale = Some(scale);

                let skew = [
                    if let Some(ref val) = cp_skew_x {
                        val.get()
                    } else {
                        0.0
                    },
                    if let Some(ref val) = cp_skew_y {
                        val.get()
                    } else {
                        0.0
                    },
                ];
                desugared_transform2d.skew = Some(skew);

                let rotate = if let Some(ref val) = cp_rotate {
                    val.get().clone()
                } else {
                    Default::default()
                };
                desugared_transform2d.rotate = Some(rotate);

                desugared_transform2d
                    .compute_transform2d_matrix(cp_bounds.get(), container_bounds.get())
                    .cast_spaces::<NodeLocal, NodeLocal>()
            };
            container_transform.get() * desugared_transform * node_transform_property_computed
        },
        &all_transform_deps,
        &format!("transform of node {}", node.id.0),
    );

    (transform, bounds)
}

pub trait ComputableTransform {
    fn compute_transform2d_matrix(
        &self,
        node_size: (f64, f64),
        container_bounds: (f64, f64),
    ) -> Transform2;
}

impl ComputableTransform for Transform2D {
    //Distinction of note: scale, translate, rotate, anchor, and align are all AUTHOR-TIME properties
    //                     node_size and container_bounds are (computed) RUNTIME properties
    fn compute_transform2d_matrix(
        &self,
        node_size: (f64, f64),
        container_bounds: (f64, f64),
    ) -> Transform2 {
        //Three broad strokes:
        // a.) compute anchor
        // b.) decompose "vanilla" affine matrix
        // c.) combine with previous transform chain (assembled via multiplication of two Transform2Ds, e.g. in PAXEL)

        // Compute anchor
        let anchor_transform = match &self.anchor {
            Some(anchor) => Transform2::translate(Vector2::<Generic>::new(
                match anchor[0] {
                    Size::Pixels(pix) => -pix.to_float(),
                    Size::Percent(per) => -node_size.0 * (per.to_float() / 100.0),
                    Size::Combined(pix, per) => {
                        -pix.to_float() + (-node_size.0 * (per.to_float() / 100.0))
                    }
                },
                match anchor[1] {
                    Size::Pixels(pix) => -pix.to_float(),
                    Size::Percent(per) => -node_size.1 * (per.to_float() / 100.0),
                    Size::Combined(pix, per) => {
                        -pix.to_float() + (-node_size.0 * (per.to_float() / 100.0))
                    }
                },
            )),
            //No anchor applied: treat as 0,0; identity matrix
            None => Transform2::default(),
        };

        //decompose vanilla affine matrix and pack into `Affine`
        let (scale_x, scale_y) = if let Some(scale) = &self.scale {
            (scale[0].expect_percent(), scale[1].expect_percent())
        } else {
            (1.0, 1.0)
        };

        let (skew_x, skew_y) = if let Some(skew) = self.skew {
            (skew[0], skew[1])
        } else {
            (0.0, 0.0)
        };

        let (translate_x, translate_y) = if let Some(translate) = &self.translate {
            (
                translate[0].evaluate(container_bounds, Axis::X),
                translate[1].evaluate(container_bounds, Axis::Y),
            )
        } else {
            (0.0, 0.0)
        };

        let rotate_rads = if let Some(rotate) = &self.rotate {
            rotate.to_float_0_1() * std::f64::consts::PI * 2.0
        } else {
            0.0
        };

        let cos_theta = rotate_rads.cos();
        let sin_theta = rotate_rads.sin();

        // Elements for a combined scale and rotation
        let a = scale_x * cos_theta - scale_y * skew_x * sin_theta;
        let b = scale_x * sin_theta + scale_y * skew_x * cos_theta;
        let c = -scale_y * sin_theta + scale_x * skew_y * cos_theta;
        let d = scale_y * cos_theta + scale_x * skew_y * sin_theta;

        // Translation
        let e = translate_x;
        let f = translate_y;

        let coeffs = [a, b, c, d, e, f];
        let transform = Transform2::new(coeffs);

        // Compute and combine previous_transform
        let previous_transform = match &self.previous {
            Some(previous) => (*previous).compute_transform2d_matrix(node_size, container_bounds),
            None => Transform2::default(),
        };

        transform * anchor_transform * previous_transform
    }
}
