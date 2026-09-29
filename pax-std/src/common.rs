use kurbo::{Affine, Shape};
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
    if expanded_node.is_render_source() {
        return Affine::from(expanded_node.transform_and_bounds.get().transform);
    }
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
    let layer_id = rc.alpha_source_layer().unwrap_or(geometry.layer);
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
/// Extracts ordered paint layers; paint coordinates use the unexpanded centerline.
pub(crate) fn alpha_mask_paints(
    node: &ExpandedNode,
    path: kurbo::BezPath,
    fill: Vec<pax_runtime_api::Fill>,
    stroke: Vec<pax_runtime_api::Stroke>,
) -> Vec<pax_runtime_api::AlphaMaskPaint> {
    alpha_mask_paints_with_range(node, path, fill, stroke, 0.0, 1.0)
}

pub(crate) fn alpha_mask_paints_with_range(
    node: &ExpandedNode,
    path: kurbo::BezPath,
    fills: Vec<pax_runtime_api::Fill>,
    strokes: Vec<pax_runtime_api::Stroke>,
    draw_start: f64,
    draw_end: f64,
) -> Vec<pax_runtime_api::AlphaMaskPaint> {
    use pax_runtime_api::drawing::{path_trim::trim_bez_path, stroke_utils::stroked_outline_path};
    let transform = kurbo::Affine::from(node.transform_and_bounds.get().transform);
    let mut paints = Vec::new();
    for fill in fills.iter().rev() {
        paints.push(pax_runtime_api::AlphaMaskPaint {
            path: path.clone(),
            transform,
            fill: fill.paint.get(),
            opacity: fill.opacity.get().to_float_0_1(),
            paint_bounds: path.bounding_box(),
            composition: Some((node.id.to_u32(), 1.0)),
        });
    }
    let paint_bounds = path.bounding_box();
    let trimmed = trim_bez_path(&path, draw_start, draw_end);
    for stroke in strokes.iter().rev() {
        if let Some(path) = stroked_outline_path(&trimmed, stroke) {
            paints.push(pax_runtime_api::AlphaMaskPaint {
                path,
                transform,
                fill: stroke.paint.get(),
                opacity: stroke.opacity.get().to_float_0_1(),
                paint_bounds,
                composition: Some((node.id.to_u32(), 1.0)),
            });
        }
    }
    paints
}

pub(crate) fn draw_appearance(
    rc: &mut dyn RenderContext,
    layer: usize,
    path: kurbo::BezPath,
    fills: &[pax_runtime_api::Fill],
    strokes: &[pax_runtime_api::Stroke],
    opacity: f64,
    smoothing: pax_runtime_api::PathSmoothing,
    draw_start: f64,
    draw_end: f64,
) {
    for fill in fills.iter().rev() {
        rc.fill_with_material_and_opacity_and_smoothing(
            layer,
            path.clone(),
            &fill.paint.get(),
            &fill.material.get(),
            opacity * fill.opacity.get().to_float_0_1(),
            smoothing,
        );
    }
    for stroke in strokes
        .iter()
        .rev()
        .filter(|stroke| stroke.width_pixels() > f64::EPSILON)
    {
        if draw_start < draw_end {
            rc.stroke_with_draw_range_and_material_and_opacity_and_smoothing(
                layer,
                path.clone(),
                stroke,
                &stroke.material.get(),
                opacity * stroke.opacity.get().to_float_0_1(),
                draw_start,
                draw_end,
                smoothing,
            );
        }
    }
}

pub(crate) fn appearance_coverage_path(
    path: &kurbo::BezPath,
    fills: &[pax_runtime_api::Fill],
    strokes: &[pax_runtime_api::Stroke],
) -> kurbo::BezPath {
    let mut coverage = kurbo::BezPath::new();
    if fills.iter().any(|fill| {
        fill.paint.get().coverage_alpha_0_1() * fill.opacity.get().to_float_0_1() > f64::EPSILON
    }) {
        coverage.extend(path.elements().iter().copied());
    }
    for stroke in strokes {
        if stroke.paint.get().coverage_alpha_0_1() * stroke.opacity.get().to_float_0_1()
            > f64::EPSILON
        {
            if let Some(outline) =
                pax_runtime_api::drawing::stroke_utils::stroked_outline_path(path, stroke)
            {
                coverage.extend(outline.elements().iter().copied());
            }
        }
    }
    coverage
}

pub(crate) fn appearance_coverage_alpha(
    fills: &[pax_runtime_api::Fill],
    strokes: &[pax_runtime_api::Stroke],
) -> f64 {
    // Coverage is a union of dissimilar outlines. Its opacity cannot exceed the
    // least opaque contributing region without incorrectly hiding content below.
    fills
        .iter()
        .map(|fill| fill.paint.get().coverage_alpha_0_1() * fill.opacity.get().to_float_0_1())
        .chain(
            strokes
                .iter()
                .filter(|s| s.width_pixels() > 0.0)
                .map(|s| s.paint.get().coverage_alpha_0_1() * s.opacity.get().to_float_0_1()),
        )
        .filter(|alpha| *alpha > f64::EPSILON)
        .reduce(f64::min)
        .unwrap_or(0.0)
}

// Own nested subscriptions with the outer appearance. Replacing a stack drops
// its old layer/material watchers; weak scene references avoid extending mount lifetimes.
// Callers retain the returned property in their listener: dependency edges do not own it.
pub(crate) fn watch_appearance(
    node: &std::rc::Rc<ExpandedNode>,
    context: &std::rc::Rc<RuntimeContext>,
    fills: pax_runtime_api::Property<Vec<pax_runtime_api::Fill>>,
    strokes: pax_runtime_api::Property<Vec<pax_runtime_api::Stroke>>,
) -> pax_runtime_api::Property<()> {
    use pax_runtime_api::{Material, Property};
    use std::{cell::RefCell, rc::Rc};
    let deps = [fills.untyped(), strokes.untyped()];
    let weak_node = Rc::downgrade(node);
    let weak_context = Rc::downgrade(context);
    let watchers = RefCell::new(Vec::<Property<()>>::new());
    let appearance = Property::computed(
        move || {
            let (Some(node), Some(context)) = (weak_node.upgrade(), weak_context.upgrade()) else {
                return;
            };
            let mut layers = Vec::new();
            for fill in fills.get() {
                layers.push((
                    fill.material.clone(),
                    vec![
                        fill.paint.untyped(),
                        fill.material.untyped(),
                        fill.opacity.untyped(),
                    ],
                ));
            }
            for stroke in strokes.get() {
                layers.push((
                    stroke.material.clone(),
                    vec![
                        stroke.paint.untyped(),
                        stroke.material.untyped(),
                        stroke.opacity.untyped(),
                        stroke.width.untyped(),
                        stroke.cap.untyped(),
                        stroke.join.untyped(),
                    ],
                ));
            }
            let mut next = Vec::new();
            for (material, dependencies) in layers {
                let weak_node = Rc::downgrade(&node);
                let weak_context = Rc::downgrade(&context);
                let material_watch = Property::default();
                let watch = Property::computed(
                    move || {
                        let (Some(node), Some(context)) =
                            (weak_node.upgrade(), weak_context.upgrade())
                        else {
                            return;
                        };
                        let material_dependencies = match material.get() {
                            Material::Unlit => vec![],
                            Material::Lit(p) => vec![
                                p.ambient.untyped(),
                                p.diffuse.untyped(),
                                p.specular.untyped(),
                                p.roughness.untyped(),
                                p.metallic.untyped(),
                                p.emissive.untyped(),
                                p.emissive_intensity.untyped(),
                            ],
                        };
                        let weak_node = Rc::downgrade(&node);
                        let weak_context = Rc::downgrade(&context);
                        material_watch.replace_with(Property::computed(
                            move || {
                                if let (Some(node), Some(context)) =
                                    (weak_node.upgrade(), weak_context.upgrade())
                                {
                                    node.changed_listener.invalidate();
                                    context.mark_canvas_node_dirty(node.id);
                                    context.mark_node_occlusion_dirty(node.id.to_u32());
                                }
                            },
                            &material_dependencies,
                        ));
                        context.register_expanded_node_effect_property_named(
                            &node,
                            &material_watch,
                            "appearance material",
                        );
                        material_watch.get();
                    },
                    &dependencies,
                );
                context.register_expanded_node_effect_property_named(
                    &node,
                    &watch,
                    "appearance layer",
                );
                watch.get();
                next.push(watch);
            }
            *watchers.borrow_mut() = next;
        },
        &deps,
    );
    // Being an inbound dependency propagates dirtiness, but does not evaluate
    // this subscription builder. Keep it active when the outer stack changes.
    context.register_expanded_node_effect_property_named(node, &appearance, "appearance stack");
    appearance.get();
    appearance
}

// Centered miters have a four-half-width limit; include all layers before tile selection.
pub(crate) fn stroke_coverage_bounds(
    bounds: kurbo::Rect,
    strokes: &[pax_runtime_api::Stroke],
) -> kurbo::Rect {
    let pad = strokes
        .iter()
        .map(|stroke| 2.0 * stroke.width_pixels())
        .fold(0.0, f64::max);
    bounds.inflate(pad, pad)
}

// Native controls still accept solid outlines only. Do not silently approximate
// a vector gradient when the same public Stroke struct crosses this boundary.
pub(crate) fn native_stroke_color(stroke: &pax_runtime_api::Stroke) -> pax_runtime_api::Color {
    match stroke.paint.get() {
        pax_runtime_api::Paint::Solid(color) => color,
        _ => {
            log::warn!("Native control outlines require solid paint; gradient stroke omitted");
            pax_runtime_api::Color::TRANSPARENT
        }
    }
}
