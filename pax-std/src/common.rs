use kurbo::Affine;
pub use pax_engine::api::Size;
use pax_engine::*;
use pax_runtime::{ExpandedNode, RuntimeContext};

#[pax]
#[engine_import_path("pax_engine")]
#[derive(Copy)]
pub struct Point {
    pub x: Size,
    pub y: Size,
}

impl Point {
    pub fn new(x: Size, y: Size) -> Self {
        Self { x, y }
    }

    pub fn to_kurbo_point(self, bounds: (f64, f64)) -> kurbo::Point {
        let x = self.x.evaluate(bounds, api::Axis::X);
        let y = self.y.evaluate(bounds, api::Axis::Y);
        kurbo::Point { x, y }
    }
}

pub fn patch_if_needed<T: PartialEq + Clone>(
    old_state: &mut Option<T>,
    patch: &mut Option<T>,
    new_value: T,
) -> bool {
    if !old_state.as_ref().is_some_and(|v| v == &new_value) {
        *patch = Some(new_value.clone());
        *old_state = Some(new_value);
        true
    } else {
        false
    }
}

pub fn native_surface_opacity(expanded_node: &ExpandedNode, context: &RuntimeContext) -> f64 {
    let opacity = expanded_node.computed_opacity.get().clamp(0.0, 1.0);
    let Some(parent_frame_id) = expanded_node.parent_frame.get() else {
        return opacity;
    };
    let Some(parent_frame) = context.get_expanded_node_by_eid(parent_frame_id) else {
        return opacity;
    };
    let parent_opacity = parent_frame.computed_opacity.get().clamp(0.0, 1.0);
    if parent_opacity <= f64::EPSILON {
        0.0
    } else {
        (opacity / parent_opacity).clamp(0.0, 1.0)
    }
}

pub fn canvas_surface_transform(expanded_node: &ExpandedNode, context: &RuntimeContext) -> Affine {
    let transform = Affine::from(expanded_node.transform_and_bounds.get().transform);
    let own_layer = expanded_node.occlusion.get().occlusion_layer_id;
    let mut parent_frame_id = expanded_node.parent_frame.get();
    while let Some(current_parent_frame_id) = parent_frame_id {
        let Some(parent_frame) = context.get_expanded_node_by_eid(current_parent_frame_id) else {
            break;
        };
        if parent_frame.occlusion.get().occlusion_layer_id != own_layer {
            // Descendant canvas layers can be mounted into browser-owned scroller hosts whose DOM
            // coordinate space is local to the owning ancestor frame, not to the root canvas. We
            // originally tried compensating at the scroller traversal layer, but retained
            // vector/image nodes still carried world-space transforms into those nested surfaces.
            // Walk up to the nearest ancestor on a different occlusion layer so nested same-layer
            // groups still localize into the browser-hosted canvas that actually owns them.
            return Affine::from(parent_frame.transform_and_bounds.get().transform.inverse())
                * transform;
        }
        parent_frame_id = parent_frame.parent_frame.get();
    }
    transform
}
