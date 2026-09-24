use kurbo::Affine;
pub use pax_engine::api::Size;
use pax_message::AppleLiquidGlassPatch;
use pax_runtime::api as pax_runtime_api;
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

// Writes inherited liquid-glass scope changes, including explicit clears.
pub fn patch_liquid_glass_if_needed(
    old_state: &mut Option<Option<AppleLiquidGlassPatch>>,
    patch: &mut Option<Option<AppleLiquidGlassPatch>>,
    expanded_node: &ExpandedNode,
) -> bool {
    patch_if_needed(
        old_state,
        patch,
        expanded_node
            .liquid_glass_scope
            .get()
            .map(|scope| scope.to_message()),
    )
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
    context.canvas_surface_transform_for_node(expanded_node)
}

// Values a primitive needs after the render context has selected live tile surfaces.
pub struct CanvasNodeRenderScope {
    pub layer_id: usize,
    pub node_id: u32,
    pub surface_transform: Affine,
    pub bounds: (f64, f64),
    /// Paint opacity within the canvas composition; the backend applies outer scopes once.
    pub paint_opacity: f64,
}

// Begin a bounded retained vector/image node.
//
// The runtime prepares coverage before replay selection. Drawing consumes that
// same record while the render context owns tile selection and stale-node removal.
pub fn begin_bounded_canvas_node(
    rc: &mut dyn RenderContext,
    expanded_node: &ExpandedNode,
    context: &RuntimeContext,
) -> Option<CanvasNodeRenderScope> {
    let geometry = context.canvas_geometry_for_node(expanded_node);
    let layer_id = geometry.layer;
    let node_id = expanded_node.id.to_u32();
    let coverage_bounds = geometry.coverage_bounds.unwrap_or(kurbo::Rect::new(
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::INFINITY,
    ));

    if !rc.begin_node_with_bounds(
        layer_id,
        node_id,
        expanded_node.occlusion.get().z_index,
        coverage_bounds,
        context.canvas_node_light_mask(expanded_node.id),
    ) {
        if rc.take_clean_skipped_node(layer_id, node_id) {
            context.clear_canvas_node_dirty(&expanded_node.id);
        }
        return None;
    }

    let paint_opacity = if rc.supports_subtree_opacity() {
        rc.set_node_opacity_scopes(
            layer_id,
            node_id,
            &expanded_node.computed_opacity_scopes.get(),
        );
        1.0
    } else {
        expanded_node.computed_opacity.get()
    };
    Some(CanvasNodeRenderScope {
        layer_id,
        node_id,
        surface_transform: geometry.surface_transform,
        bounds: geometry.bounds,
        paint_opacity,
    })
}
/// Extracts fill and stroke alpha with the same local geometry as visible paint.
pub(crate) fn alpha_mask_paints(
    node: &pax_runtime::ExpandedNode,
    path: kurbo::BezPath,
    fill: pax_runtime_api::Fill,
    stroke: pax_runtime_api::Stroke,
) -> Vec<pax_runtime_api::AlphaMaskPaint> {
    use pax_runtime_api::drawing::stroke_utils::stroked_outline_path;
    let transform = kurbo::Affine::from(node.transform_and_bounds.get().transform);
    let opacity = 1.0;
    let mut paints = Vec::new();
    if fill.coverage_alpha_0_1() > f64::EPSILON {
        paints.push(pax_runtime_api::AlphaMaskPaint {
            path: path.clone(),
            transform,
            fill,
            opacity,
        });
    }
    if let Some(path) = stroked_outline_path(&path, &stroke) {
        paints.push(pax_runtime_api::AlphaMaskPaint {
            path,
            transform,
            fill: pax_runtime_api::Fill::Solid(stroke.color.get()),
            opacity,
        });
    }
    paints
}
