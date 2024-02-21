use crate::api::{Axis, Size, Transform2D};
use crate::math::{Generic, Transform2, Vector2};
use crate::node_interface::NodeLocal;
use crate::{ExpandedNode, TransformAndBounds};

/// For the `current_expanded_node` attached to `ptc`, calculates and returns a new [`crate::rendering::TransformAndBounds`] a.k.a. "tab".
/// Intended as a helper method to be called during properties computation, for creating a new tab to attach to `ptc` for downstream calculations.
pub fn compute_tab(node: &ExpandedNode, container_tab: &TransformAndBounds) -> TransformAndBounds {
    //get the size of this node (calc'd or otherwise) and use
    //it as the new accumulated bounds: both for this node's children (their parent container bounds)
    //and for this node itself (e.g. for specifying the size of a Rectangle node)
    let new_accumulated_bounds_and_current_node_size =
        { node.get_size_computed(container_tab.bounds) };

    let node_transform_property_computed = {
        node.get_common_properties()
            .borrow()
            .transform
            .get()
            .compute_transform2d_matrix(
                new_accumulated_bounds_and_current_node_size.clone(),
                container_tab.bounds,
            )
            .cast_spaces::<NodeLocal, NodeLocal>()
    };

    // From a combination of the sugared TemplateNodeDefinition properties like `width`, `height`, `x`, `y`, `scale_x`, etc.
    let desugared_transform = {
        //Extract common_properties, pack into Transform2D, decompose / compute, and combine with node_computed_transform
        let comm = node.get_common_properties();
        let comm = comm.borrow();
        let mut desugared_transform2d = Transform2D::default();

        let translate = [
            if let Some(ref val) = comm.x {
                val.get().clone()
            } else {
                Size::ZERO()
            },
            if let Some(ref val) = comm.y {
                val.get().clone()
            } else {
                Size::ZERO()
            },
        ];
        desugared_transform2d.translate = Some(translate);

        let anchor = [
            if let Some(ref val) = comm.anchor_x {
                val.get().clone()
            } else {
                Size::ZERO()
            },
            if let Some(ref val) = comm.anchor_y {
                val.get().clone()
            } else {
                Size::ZERO()
            },
        ];
        desugared_transform2d.anchor = Some(anchor);

        let scale = [
            if let Some(ref val) = comm.scale_x {
                val.get().clone()
            } else {
                Size::Percent(crate::numeric::Numeric::from(100.0))
            },
            if let Some(ref val) = comm.scale_y {
                val.get().clone()
            } else {
                Size::Percent(crate::numeric::Numeric::from(100.0))
            },
        ];
        desugared_transform2d.scale = Some(scale);

        let skew = [
            if let Some(ref val) = comm.skew_x {
                val.get().get_as_float()
            } else {
                0.0
            },
            if let Some(ref val) = comm.skew_y {
                val.get().get_as_float()
            } else {
                0.0
            },
        ];
        desugared_transform2d.skew = Some(skew);

        let rotate = if let Some(ref val) = comm.rotate {
            val.get().clone()
        } else {
            crate::api::Rotation::ZERO()
        };
        desugared_transform2d.rotate = Some(rotate);

        desugared_transform2d
            .compute_transform2d_matrix(
                new_accumulated_bounds_and_current_node_size.clone(),
                container_tab.bounds,
            )
            .cast_spaces::<NodeLocal, NodeLocal>()
    };

    let new_accumulated_transform =
        container_tab.transform * desugared_transform * node_transform_property_computed;

    // let new_scroller_normalized_accumulated_transform =
    //     accumulated_scroller_normalized_transform
    //         * desugared_transform
    //         * node_transform_property_computed;

    // rtc.transform_scroller_reset = new_scroller_normalized_accumulated_transform.clone();

    TransformAndBounds {
        transform: new_accumulated_transform,
        bounds: new_accumulated_bounds_and_current_node_size,
    }
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
                    Size::Pixels(pix) => -pix.get_as_float(),
                    Size::Percent(per) => -node_size.0 * (per / 100.0),
                    Size::Combined(pix, per) => {
                        -pix.get_as_float() + (-node_size.0 * (per / 100.0))
                    }
                },
                match anchor[1] {
                    Size::Pixels(pix) => -pix.get_as_float(),
                    Size::Percent(per) => -node_size.1 * (per / 100.0),
                    Size::Combined(pix, per) => {
                        -pix.get_as_float() + (-node_size.0 * (per / 100.0))
                    }
                },
            )),
            //No anchor applied: treat as 0,0; identity matrix
            None => Transform2::default(),
        };

        //decompose vanilla affine matrix and pack into `Affine`
        let (scale_x, scale_y) = if let Some(scale) = self.scale {
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
            rotate.get_as_radians()
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
