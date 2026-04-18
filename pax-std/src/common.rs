use kurbo::{Affine, Shape};
pub use pax_engine::api::Size;
use pax_runtime::api::RenderContext;
use pax_runtime::{ExpandedNode, RuntimeContext};

// Resolves Pax units against bounds into a kurbo point.
pub(crate) fn to_kurbo_point(x: Size, y: Size, bounds: (f64, f64)) -> kurbo::Point {
    let x = x.evaluate(bounds, pax_engine::api::Axis::X);
    let y = y.evaluate(bounds, pax_engine::api::Axis::Y);
    kurbo::Point { x, y }
}

// Writes a patch field only when the new value differs from cached old state.
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

// Converts computed world opacity into the relative opacity expected by native hosts.
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

// Resolves the transform a canvas primitive should use inside its owning canvas surface.
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

fn canvas_surface_bounds_for_transform(transform: Affine, bounds: (f64, f64)) -> kurbo::Rect {
    const TILE_CULL_BOUNDS_PAD: f64 = 64.0;
    let (width, height) = bounds;
    let bounds_path = kurbo::Rect::new(0.0, 0.0, width, height).to_path(0.1);
    (transform * bounds_path)
        .bounding_box()
        .inflate(TILE_CULL_BOUNDS_PAD, TILE_CULL_BOUNDS_PAD)
}

// Values a primitive needs after the render context has selected live tile surfaces.
pub struct CanvasNodeRenderScope {
    pub layer_id: usize,
    pub node_id: u32,
    pub surface_transform: Affine,
    pub bounds: (f64, f64),
}

// Begin a bounded retained vector/image node.
//
// The render context owns tile selection and stale-node removal, but `pax-std`
// owns deriving conservative canvas-space coverage from an expanded node.
pub fn begin_bounded_canvas_node(
    rc: &mut dyn RenderContext,
    expanded_node: &ExpandedNode,
    context: &RuntimeContext,
) -> Option<CanvasNodeRenderScope> {
    let layer_id = expanded_node.occlusion.get().occlusion_layer_id;
    let node_id = expanded_node.id.to_u32();
    let tab = expanded_node.transform_and_bounds.get();
    let surface_transform = canvas_surface_transform(expanded_node, context);
    let coverage_bounds = canvas_surface_bounds_for_transform(surface_transform, tab.bounds);

    if !rc.begin_node_with_bounds(
        layer_id,
        node_id,
        expanded_node.occlusion.get().z_index,
        coverage_bounds,
    ) {
        return None;
    }

    Some(CanvasNodeRenderScope {
        layer_id,
        node_id,
        surface_transform,
        bounds: tab.bounds,
    })
}
