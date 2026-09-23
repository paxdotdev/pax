use crate::common::{native_surface_opacity, patch_if_needed};
use kurbo::{Affine, BezPath, RoundedRect, Shape};
use pax_engine::api::{Property, Size};
use pax_engine::*;
use pax_message::{AnyCreatePatch, NativeInterrupt, ScrollerPatch};
use pax_runtime::api::{borrow, borrow_mut, use_RefCell, Layer, NodeContext};
use pax_runtime::{
    bind_content_measurement_effect, measure_content_children_forward_extents, BaseInstance,
    ContentMeasurementGeometry, ExpandedNode, ExpandedNodeIdentifier, InstanceFlags, InstanceNode,
    InstantiationArgs, RuntimeContext, ScrollerSurfaceState, ScrollerSurfaceStateChange,
};
use std::iter;
use std::rc::Rc;

use_RefCell!();

/// A scrolling container, which clips its bounds and offers platform-native scrolling
/// on either or both of the horizontal and vertical axes.
#[pax]
#[engine_import_path("pax_engine")]
#[inlined(
    <ScrollerHost
        scroll_pos_x=bind:scroll_pos_x
        scroll_pos_y=bind:scroll_pos_y
        scroll_width={self._resolved_scroll_width}
        scroll_height={self._resolved_scroll_height}
        corner_radius={self.corner_radius}
        snap_positions_x={self.snap_positions_x}
        snap_positions_y={self.snap_positions_y}
        _clip_content={!$suspended}
    >
        for i in 0..self._projected_children_count {
            slot(i)
        }
    </ScrollerHost>

    @settings {
        @mount: on_mount
    }

)]
#[custom(Default)]
pub struct Scroller {
    /// Horizontal scroll offset, in pixels.
    pub scroll_pos_x: Property<f64>,
    /// Vertical scroll offset, in pixels.
    pub scroll_pos_y: Property<f64>,
    /// Width of the scrollable content pane.
    pub scroll_width: Property<Size>,
    /// Height of the scrollable content pane.
    pub scroll_height: Property<Size>,
    /// Automatically sizes the scroll pane on its default axes when possible.
    ///
    /// For `Scroller`, the default autosized axis is `y`; `x` remains bound to
    /// `scroll_width` unless `autosize_x` explicitly opts in.
    pub autosize: Property<bool>,
    /// Optional override for whether autosize manages the `x` axis.
    pub autosize_x: Property<Option<bool>>,
    /// Optional override for whether autosize manages the `y` axis.
    pub autosize_y: Property<Option<bool>>,
    /// Corner radius for the scroller clipping region, in pixels.
    pub corner_radius: Property<f64>,
    /// Scroll snap anchors expressed in px/% along each axis.
    /// Web maps to CSS scroll-snap-type + scroll-snap-align; Apple chassis map these
    /// offsets to native scroll end-points while keeping engine scroll state authoritative.
    pub snap_positions_x: Property<Vec<Size>>,
    /// Vertical scroll snap anchors expressed in px/%.
    pub snap_positions_y: Property<Vec<Size>>,

    // Controls whether the scroller clips content to its own bounds.
    // Used by pax create; might become public once the API is settled.
    pub _clip_content: Property<bool>,

    // Private template bookkeeping.
    pub _projected_children_count: Property<usize>,
    // Host-facing resolved content width after applying autosize semantics.
    pub _resolved_scroll_width: Property<Size>,
    // Host-facing resolved content height after applying autosize semantics.
    pub _resolved_scroll_height: Property<Size>,
}

impl Default for Scroller {
    fn default() -> Self {
        Self {
            scroll_pos_x: Default::default(),
            scroll_pos_y: Default::default(),
            scroll_width: Default::default(),
            scroll_height: Default::default(),
            autosize: Property::new(false),
            autosize_x: Property::new(None),
            autosize_y: Property::new(None),
            corner_radius: Property::new(0.0),
            snap_positions_x: Default::default(),
            snap_positions_y: Default::default(),
            _clip_content: Property::new(true),
            _projected_children_count: Default::default(),
            _resolved_scroll_width: Default::default(),
            _resolved_scroll_height: Default::default(),
        }
    }
}

#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::core::scroller::ScrollerHostInstance")]
#[custom(Default)]
// Internal native-host component used by `Scroller`.
pub struct ScrollerHost {
    // Bound horizontal scroll offset mirrored to/from the platform-native host.
    pub scroll_pos_x: Property<f64>,
    // Bound vertical scroll offset mirrored to/from the platform-native host.
    pub scroll_pos_y: Property<f64>,
    // Native inner pane width.
    pub scroll_width: Property<Size>,
    // Native inner pane height.
    pub scroll_height: Property<Size>,
    // Native clipping corner radius.
    pub corner_radius: Property<f64>,
    // Horizontal native snap anchors.
    pub snap_positions_x: Property<Vec<Size>>,
    // Vertical native snap anchors.
    pub snap_positions_y: Property<Vec<Size>>,
    // Scroll offset actually presented by the native host, including tile-window offsets.
    pub _presentation_scroll_x: Property<f64>,
    // Scroll offset actually presented by the native host, including tile-window offsets.
    pub _presentation_scroll_y: Property<f64>,
    // Controls native host clipping.
    pub _clip_content: Property<bool>,
}

impl Default for ScrollerHost {
    fn default() -> Self {
        Self {
            scroll_pos_x: Default::default(),
            scroll_pos_y: Default::default(),
            scroll_width: Default::default(),
            scroll_height: Default::default(),
            corner_radius: Property::new(0.0),
            snap_positions_x: Default::default(),
            snap_positions_y: Default::default(),
            _presentation_scroll_x: Default::default(),
            _presentation_scroll_y: Default::default(),
            _clip_content: Property::new(true),
        }
    }
}

// Runtime instance backing `<ScrollerHost>`.
pub struct ScrollerHostInstance {
    base: BaseInstance,
}

fn mark_canvas_descendants_dirty(expanded_node: &ExpandedNode, context: &Rc<RuntimeContext>) {
    for child in expanded_node.children.get().iter() {
        if borrow!(child.instance_node).base().flags().layer == Layer::Canvas {
            context.mark_canvas_node_dirty(child.id);
            context.set_canvas_dirty(child.occlusion.get().render_layer_id);
        }
        mark_canvas_descendants_dirty(child, context);
    }
}

fn resolve_scroller_island_layer(expanded_node: &ExpandedNode) -> Option<usize> {
    if let Some(layer_id) = expanded_node.browser_content_layer_id.get() {
        return Some(layer_id as usize);
    }

    let own_layer = expanded_node.occlusion.get().render_layer_id;
    fn find_descendant_layer(node: &ExpandedNode, own_layer: usize) -> Option<usize> {
        let mut resolved: Option<usize> = None;
        for child in node.children.get().iter() {
            let child_layer = child.occlusion.get().render_layer_id;
            if child_layer != own_layer {
                resolved = Some(match resolved {
                    Some(current) => current.min(child_layer),
                    None => child_layer,
                });
            }
            if let Some(descendant_layer) = find_descendant_layer(child, own_layer) {
                resolved = Some(match resolved {
                    Some(current) => current.min(descendant_layer),
                    None => descendant_layer,
                });
            }
        }
        resolved
    }

    find_descendant_layer(expanded_node, own_layer)
}

fn scroller_clip_path(
    expanded_node: &ExpandedNode,
    clip_content: bool,
    corner_radius: f64,
) -> Option<BezPath> {
    if !clip_content {
        return None;
    }

    let t_and_b = expanded_node.transform_and_bounds.get();
    let transform = t_and_b.transform;
    let (width, height) = t_and_b.bounds;
    let max_radius = 0.5 * width.max(0.0).min(height.max(0.0));
    let radius = corner_radius.clamp(0.0, max_radius);

    let bez_path = if radius > f64::EPSILON {
        RoundedRect::new(0.0, 0.0, width, height, radius).to_path(0.1)
    } else {
        let mut bez_path = BezPath::new();
        bez_path.move_to((0.0, 0.0));
        bez_path.line_to((width, 0.0));
        bez_path.line_to((width, height));
        bez_path.line_to((0.0, height));
        bez_path.line_to((0.0, 0.0));
        bez_path.close_path();
        bez_path
    };

    Some(<Affine>::from(transform) * bez_path)
}

fn clamp_offset(value: f64, content: f64, viewport: f64) -> f64 {
    if content <= viewport {
        return 0.0;
    }
    if !value.is_finite() {
        return 0.0;
    }
    value.max(0.0).min((content - viewport).max(0.0))
}

fn root_scroller_delegates_to_page_scroll(
    expanded_node: &ExpandedNode,
    context: &RuntimeContext,
) -> bool {
    context.get_root_scroller_id() == Some(expanded_node.id.to_u32())
        && context.get_visual_viewport_state().is_some()
}

fn effective_presentation_scroll(
    expanded_node: &ExpandedNode,
    context: &RuntimeContext,
) -> (f64, f64) {
    let (scroll_x, scroll_y) = context
        .get_scroller_surface_scroll(expanded_node.id.to_u32())
        .unwrap_or_else(|| {
            expanded_node.with_properties_unwrapped(|scroller: &mut ScrollerHost| {
                (
                    scroller._presentation_scroll_x.get(),
                    scroller._presentation_scroll_y.get(),
                )
            })
        });
    if context.get_root_scroller_id() == Some(expanded_node.id.to_u32()) {
        if let Some(visual) = context.get_visual_viewport_state() {
            let visual_x = visual.page_scroll_x + visual.offset_x;
            let visual_y = visual.page_scroll_y + visual.offset_y;
            if visual_x.is_finite() && visual_y.is_finite() {
                let computed_tab = expanded_node.transform_and_bounds.get();
                let (fallback_viewport_width, fallback_viewport_height) = computed_tab.bounds;
                let viewport_width = if visual.width.is_finite() {
                    visual.width
                } else {
                    fallback_viewport_width
                };
                let viewport_height = if visual.height.is_finite() {
                    visual.height
                } else {
                    fallback_viewport_height
                };
                let (content_width, content_height) =
                    expanded_node.with_properties_unwrapped(|scroller: &mut ScrollerHost| {
                        (
                            scroller
                                .scroll_width
                                .get()
                                .get_pixels(fallback_viewport_width),
                            scroller
                                .scroll_height
                                .get()
                                .get_pixels(fallback_viewport_height),
                        )
                    });
                return (
                    clamp_offset(visual_x, content_width, viewport_width),
                    clamp_offset(visual_y, content_height, viewport_height),
                );
            }
        }
    }
    (scroll_x, scroll_y)
}

impl InstanceNode for ScrollerHostInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self>
    where
        Self: Sized,
    {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    layer: Layer::Native,
                    is_component: false,
                    is_slot: false,
                },
            ),
        })
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        let id = expanded_node.id.to_u32();
        if expanded_node.parent_frame.get().is_none() {
            context.set_root_scroller_id(Some(id));
        }
        context.enqueue_native_message(pax_message::NativeMessage::ScrollerCreate(
            AnyCreatePatch {
                id,
                parent_frame: expanded_node.parent_frame.get().map(|v| v.to_u32()),
                render_layer_id: 0,
            },
        ));

        let env = Rc::clone(&expanded_node.stack);
        let children = borrow!(self.base().get_instance_children());
        let children_with_envs = children.iter().cloned().zip(iter::repeat(env));
        let child_parent_frame: Property<Option<ExpandedNodeIdentifier>> =
            Property::new(Some(expanded_node.id));
        let new_children =
            expanded_node.generate_children(children_with_envs, context, &child_parent_frame, true);
        expanded_node.children.set(new_children);

        let weak_self_ref = Rc::downgrade(&expanded_node);
        let context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(ScrollerPatch {
            id,
            ..Default::default()
        }));
        let last_presentation_scroll = Rc::new(RefCell::new((0.0, 0.0)));

        let deps: Vec<_> = borrow!(expanded_node.properties_scope)
            .values()
            .cloned()
            .map(|v| v.get_untyped_property().clone())
            .chain([
                expanded_node.transform_and_bounds.untyped(),
                expanded_node.computed_opacity.untyped(),
                expanded_node.occlusion.untyped(),
            ])
            .collect();
        expanded_node
            .changed_listener
            .replace_with(Property::computed(
                move || {
                    let Some(expanded_node) = weak_self_ref.upgrade() else {
                        return;
                    };
                    let mut old_state = borrow_mut!(last_patch);
                    let mut previous_presentation_scroll = borrow_mut!(last_presentation_scroll);
                    let mut patch = ScrollerPatch {
                        id,
                        ..Default::default()
                    };

                    expanded_node.with_properties_unwrapped(|properties: &mut ScrollerHost| {
                        let computed_tab = expanded_node.transform_and_bounds.get();
                        let (width, height) = computed_tab.bounds;
                        let corner_radius = properties.corner_radius.get();
                        let max_radius = 0.5 * width.max(0.0).min(height.max(0.0));
                        let clamped_radius = corner_radius.clamp(0.0, max_radius);
                        let scroll_width = properties.scroll_width.get().get_pixels(width);
                        let scroll_height = properties.scroll_height.get().get_pixels(height);
                        let snap_points_x: Vec<f64> = properties
                            .snap_positions_x
                            .get()
                            .iter()
                            .map(|pos| pos.get_pixels(width))
                            .collect();
                        let snap_points_y: Vec<f64> = properties
                            .snap_positions_y
                            .get()
                            .iter()
                            .map(|pos| pos.get_pixels(height))
                            .collect();
                        let scroll_enabled_x = scroll_width > width + 0.5;
                        let scroll_enabled_y = scroll_height > height + 0.5;
                        let logical_scroll =
                            (properties.scroll_pos_x.get(), properties.scroll_pos_y.get());
                        // A property write is a new scroll request, not a replay of the last
                        // native observation. Move presentation by the same delta, preserving
                        // any host-relative offset. Before the first observation, start at the
                        // authored position so native hosts and tile plans agree on the viewport.
                        let presentation_scroll = context
                            .get_scroller_surface_state(id)
                            .map(|previous| {
                                let advance = |requested: f64, logical: f64, presented: f64| {
                                    if presented.is_finite() {
                                        presented + (requested - logical)
                                    } else {
                                        requested
                                    }
                                };
                                (
                                    advance(
                                        logical_scroll.0,
                                        previous.scroll_x,
                                        previous.presentation_scroll_x,
                                    ),
                                    advance(
                                        logical_scroll.1,
                                        previous.scroll_y,
                                        previous.presentation_scroll_y,
                                    ),
                                )
                            })
                            .unwrap_or(logical_scroll);
                        properties
                            ._presentation_scroll_x
                            .set_if_neq(presentation_scroll.0);
                        properties
                            ._presentation_scroll_y
                            .set_if_neq(presentation_scroll.1);
                        let presentation_changed =
                            (presentation_scroll.0 - previous_presentation_scroll.0).abs() > 1e-4
                                || (presentation_scroll.1 - previous_presentation_scroll.1).abs()
                                    > 1e-4;
                        *previous_presentation_scroll = presentation_scroll;
                        let scroller_island_layer = resolve_scroller_island_layer(&expanded_node);
                        let has_scroller_island = scroller_island_layer.is_some();
                        let surface_change = context.set_scroller_surface_state(
                            id,
                            ScrollerSurfaceState {
                                viewport_width: width,
                                viewport_height: height,
                                content_width: scroll_width,
                                content_height: scroll_height,
                                scroll_x: properties.scroll_pos_x.get(),
                                scroll_y: properties.scroll_pos_y.get(),
                                presentation_scroll_x: presentation_scroll.0,
                                presentation_scroll_y: presentation_scroll.1,
                                clip_content: properties._clip_content.get(),
                            },
                        );
                        if surface_change == ScrollerSurfaceStateChange::ScrollOnly
                            && !has_scroller_island
                        {
                            context.mark_occlusion_dirty();
                        }
                        let updates = [
                            patch_if_needed(&mut old_state.size_x, &mut patch.size_x, width),
                            patch_if_needed(&mut old_state.size_y, &mut patch.size_y, height),
                            patch_if_needed(
                                &mut old_state.corner_radius,
                                &mut patch.corner_radius,
                                clamped_radius,
                            ),
                            patch_if_needed(
                                &mut old_state.parent_frame,
                                &mut patch.parent_frame,
                                expanded_node.parent_frame.get().map(|v| v.to_u32()),
                            ),
                            patch_if_needed(
                                &mut old_state.z_index,
                                &mut patch.z_index,
                                expanded_node.occlusion.get().z_index,
                            ),
                            patch_if_needed(
                                &mut old_state.size_inner_pane_x,
                                &mut patch.size_inner_pane_x,
                                scroll_width,
                            ),
                            patch_if_needed(
                                &mut old_state.size_inner_pane_y,
                                &mut patch.size_inner_pane_y,
                                scroll_height,
                            ),
                            patch_if_needed(
                                &mut old_state.snap_points_x,
                                &mut patch.snap_points_x,
                                snap_points_x,
                            ),
                            patch_if_needed(
                                &mut old_state.snap_points_y,
                                &mut patch.snap_points_y,
                                snap_points_y,
                            ),
                            patch_if_needed(
                                &mut old_state.scroll_x,
                                &mut patch.scroll_x,
                                properties.scroll_pos_x.get(),
                            ),
                            patch_if_needed(
                                &mut old_state.scroll_y,
                                &mut patch.scroll_y,
                                properties.scroll_pos_y.get(),
                            ),
                            patch_if_needed(
                                &mut old_state.presentation_scroll_x,
                                &mut patch.presentation_scroll_x,
                                presentation_scroll.0,
                            ),
                            patch_if_needed(
                                &mut old_state.presentation_scroll_y,
                                &mut patch.presentation_scroll_y,
                                presentation_scroll.1,
                            ),
                            patch_if_needed(
                                &mut old_state.scroll_enabled_x,
                                &mut patch.scroll_enabled_x,
                                scroll_enabled_x,
                            ),
                            patch_if_needed(
                                &mut old_state.scroll_enabled_y,
                                &mut patch.scroll_enabled_y,
                                scroll_enabled_y,
                            ),
                            patch_if_needed(
                                &mut old_state.transform,
                                &mut patch.transform,
                                computed_tab.transform.coeffs().to_vec(),
                            ),
                            patch_if_needed(
                                &mut old_state.opacity,
                                &mut patch.opacity,
                                native_surface_opacity(&expanded_node, &context),
                            ),
                            patch_if_needed(
                                &mut old_state.clip_content,
                                &mut patch.clip_content,
                                properties._clip_content.get(),
                            ),
                        ];
                        let scroll_updated = patch.scroll_x.is_some()
                            || patch.scroll_y.is_some()
                            || patch.presentation_scroll_x.is_some()
                            || patch.presentation_scroll_y.is_some();
                        let visual_update = patch.size_x.is_some()
                            || patch.size_y.is_some()
                            || patch.size_inner_pane_x.is_some()
                            || patch.size_inner_pane_y.is_some()
                            || patch.scroll_enabled_x.is_some()
                            || patch.scroll_enabled_y.is_some()
                            || patch.transform.is_some()
                            || patch.opacity.is_some()
                            || patch.clip_content.is_some();
                        if updates.into_iter().any(|updated| updated) {
                            context.enqueue_native_message(
                                pax_message::NativeMessage::ScrollerUpdate(patch),
                            );
                            if visual_update {
                                // Browser-owned scroller islands now keep tiled canvases mounted in
                                // content coordinates, so ordinary scroll motion is handled by the
                                // browser moving the host. Reserve descendant canvas invalidation
                                // for real visual/layout changes; tile-window shifts are handled by
                                // the chassis surface-refresh path instead of rerendering every
                                // canvas node on each scroll tick.
                                mark_canvas_descendants_dirty(&expanded_node, &context);
                            } else if scroll_updated && !has_scroller_island {
                                // Root/non-island scrollers still render into a fixed surface; when
                                // scroll changes, their vector content needs a redraw to stay aligned.
                                mark_canvas_descendants_dirty(&expanded_node, &context);
                            } else if scroll_updated {
                                // Scroller islands keep geometry in content coordinates, but
                                // inherited lights must be re-expressed through the current scroll
                                // transform and uploaded to the retained surface.
                                if let Some(layer) = scroller_island_layer {
                                    context.set_canvas_dirty(layer);
                                }
                            }
                        } else if presentation_changed && !scroll_updated {
                            mark_canvas_descendants_dirty(&expanded_node, &context);
                        }
                    });
                },
                &deps,
            ));
    }

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        expanded_node
            .changed_listener
            .replace_with(Property::default());
        if context.get_root_scroller_id() == Some(expanded_node.id.to_u32()) {
            context.set_root_scroller_id(None);
        }
        context.remove_scroller_surface_state(expanded_node.id.to_u32());
        context.enqueue_native_message(pax_message::NativeMessage::ScrollerDelete(
            expanded_node.id.to_u32(),
        ));
    }

    fn handle_native_interrupt(
        &self,
        expanded_node: &Rc<ExpandedNode>,
        interrupt: &NativeInterrupt,
    ) {
        if let NativeInterrupt::ScrollerPosition(args) = interrupt {
            expanded_node.with_properties_unwrapped(|props: &mut ScrollerHost| {
                if (props.scroll_pos_x.get() - args.scroll_x).abs() > 1e-4 {
                    props.scroll_pos_x.set(args.scroll_x);
                }
                if (props.scroll_pos_y.get() - args.scroll_y).abs() > 1e-4 {
                    props.scroll_pos_y.set(args.scroll_y);
                }
            });
        }
    }

    fn resolve_effect_clip_path(&self, expanded_node: &ExpandedNode) -> Option<BezPath> {
        let (clip_content, corner_radius) =
            expanded_node.with_properties_unwrapped(|scroller: &mut ScrollerHost| {
                (scroller._clip_content.get(), scroller.corner_radius.get())
            });
        scroller_clip_path(expanded_node, clip_content, corner_radius)
    }

    fn handle_pre_render(
        &self,
        expanded_node: &ExpandedNode,
        rtc: &Rc<RuntimeContext>,
        rcs: &mut dyn pax_runtime::api::RenderContext,
    ) {
        let layers = rcs.layers();
        let has_dirty_layer = (0..layers).any(|layer| rtc.is_canvas_dirty(&layer));
        if !has_dirty_layer {
            return;
        }

        if resolve_scroller_island_layer(expanded_node).is_some()
            || root_scroller_delegates_to_page_scroll(expanded_node, rtc)
        {
            return;
        }

        let clip_content = expanded_node
            .with_properties_unwrapped(|scroller: &mut ScrollerHost| scroller._clip_content.get());
        let (scroll_x, scroll_y) = effective_presentation_scroll(expanded_node, rtc);
        let should_translate = scroll_x.abs() > f64::EPSILON || scroll_y.abs() > f64::EPSILON;
        if !clip_content && !should_translate {
            return;
        }
        let transformed_bez_path = clip_content
            .then(|| self.resolve_effect_clip_path(expanded_node))
            .flatten();

        #[cfg(debug_assertions)]
        let mut applied_layers = 0;

        for layer in 0..layers {
            if !rtc.is_canvas_dirty(&layer) {
                continue;
            }
            // Scroller clip/translation is inherited by descendants. Bounds-aware culling belongs
            // to leaf draw nodes so the transform stack remains balanced on every active renderer.
            if !rcs.begin_node(
                layer,
                expanded_node.id.to_u32(),
                expanded_node.occlusion.get().z_index,
                0,
            ) {
                continue;
            }
            rcs.save(layer);
            if let Some(transformed_bez_path) = transformed_bez_path.clone() {
                rcs.clip(layer, transformed_bez_path);
            }
            if should_translate {
                rcs.transform(layer, Affine::translate((-scroll_x, -scroll_y)));
            }
            let _ = rcs.end_node(layer, expanded_node.id.to_u32());
            #[cfg(debug_assertions)]
            {
                applied_layers += 1;
            }
        }

        #[cfg(debug_assertions)]
        log::trace!(
            "scroller layer pre_render: node={}, total_layers={}, applied_layers={}, clip={}, translate={}",
            expanded_node.id.to_u32(),
            layers,
            applied_layers,
            clip_content,
            should_translate,
        );
    }

    fn handle_post_render(
        &self,
        expanded_node: &ExpandedNode,
        rtc: &Rc<RuntimeContext>,
        rcs: &mut dyn pax_runtime::api::RenderContext,
    ) {
        if resolve_scroller_island_layer(expanded_node).is_some()
            || root_scroller_delegates_to_page_scroll(expanded_node, rtc)
        {
            return;
        }

        let clip_content = expanded_node
            .with_properties_unwrapped(|scroller: &mut ScrollerHost| scroller._clip_content.get());
        let (scroll_x, scroll_y) = effective_presentation_scroll(expanded_node, rtc);
        if !clip_content && scroll_x.abs() <= f64::EPSILON && scroll_y.abs() <= f64::EPSILON {
            return;
        }

        let layers = rcs.layers();
        let has_dirty_layer = (0..layers).any(|layer| rtc.is_canvas_dirty(&layer));
        if !has_dirty_layer {
            return;
        }

        #[cfg(debug_assertions)]
        let mut restored_layers = 0;

        for layer in 0..layers {
            if !rtc.is_canvas_dirty(&layer) {
                continue;
            }
            rcs.restore(layer);
            #[cfg(debug_assertions)]
            {
                restored_layers += 1;
            }
        }

        #[cfg(debug_assertions)]
        log::trace!(
            "scroller layer post_render: node={}, total_layers={}, restored_layers={}",
            expanded_node.id.to_u32(),
            layers,
            restored_layers,
        );
    }

    fn clips_content(&self, expanded_node: &ExpandedNode) -> bool {
        expanded_node
            .with_properties_unwrapped(|scroller: &mut ScrollerHost| scroller._clip_content.get())
    }

    fn scrolls_content(&self, _expanded_node: &ExpandedNode) -> bool {
        true
    }

    fn property_requires_occlusion_recompute(&self, property_name: &str) -> bool {
        // Scroll deltas update presentation state without changing scroller ownership, clipping,
        // or native mask structure; structural scroller properties still recompute occlusion.
        !matches!(
            property_name,
            "scroll_pos_x" | "scroll_pos_y" | "_presentation_scroll_x" | "_presentation_scroll_y"
        )
    }

    fn resolve_scroll_offset(&self, expanded_node: &ExpandedNode) -> Option<(f64, f64)> {
        Some(
            expanded_node.with_properties_unwrapped(|scroller: &mut ScrollerHost| {
                (
                    scroller._presentation_scroll_x.get(),
                    scroller._presentation_scroll_y.get(),
                )
            }),
        )
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        match expanded_node {
            Some(expanded_node) => {
                expanded_node.with_properties_unwrapped(|_scroller: &mut ScrollerHost| {
                    f.debug_struct("ScrollerHost").finish()
                })
            }
            None => f.debug_struct("ScrollerHost").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}

impl Scroller {
    // Binds slot count and the reactive autosize measurement for the inline template.
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let projected_children_count = ctx.projected_children_count.clone();
        let deps = [projected_children_count.untyped()];
        self._projected_children_count
            .replace_with(Property::computed(
                move || projected_children_count.get(),
                &deps,
            ));

        let Some(expanded_node) = ctx.expanded_node.upgrade() else {
            return;
        };

        self._resolved_scroll_width.set(self.scroll_width.get());
        self._resolved_scroll_height.set(self.scroll_height.get());

        let autosize = self.autosize.clone();
        let autosize_x = self.autosize_x.clone();
        let autosize_y = self.autosize_y.clone();
        let scroll_width = self.scroll_width.clone();
        let scroll_height = self.scroll_height.clone();
        let resolved_scroll_width = self._resolved_scroll_width.clone();
        let resolved_scroll_height = self._resolved_scroll_height.clone();
        let deps = [
            autosize.untyped(),
            autosize_x.untyped(),
            autosize_y.untyped(),
            scroll_width.untyped(),
            scroll_height.untyped(),
        ];
        bind_content_measurement_effect(
            &expanded_node,
            ctx,
            "scroller autosize",
            ContentMeasurementGeometry::Placed,
            &deps,
            move |_node, node_ctx| {
                sync_scroller_autosize(
                    node_ctx,
                    &autosize,
                    &autosize_x,
                    &autosize_y,
                    &scroll_width,
                    &scroll_height,
                    &resolved_scroll_width,
                    &resolved_scroll_height,
                );
            },
        );
    }
}

fn resolve_axis_autosize(
    autosize: bool,
    axis_override: Option<bool>,
    default_when_enabled: bool,
) -> bool {
    axis_override.unwrap_or(autosize && default_when_enabled)
}

fn sync_scroller_autosize(
    ctx: &NodeContext,
    autosize: &Property<bool>,
    autosize_x: &Property<Option<bool>>,
    autosize_y: &Property<Option<bool>>,
    scroll_width: &Property<Size>,
    scroll_height: &Property<Size>,
    resolved_scroll_width: &Property<Size>,
    resolved_scroll_height: &Property<Size>,
) {
    let (content_width, content_height) = measure_content_children_forward_extents(ctx);
    let manage_width = resolve_axis_autosize(autosize.get(), autosize_x.get(), false);
    let manage_height = resolve_axis_autosize(autosize.get(), autosize_y.get(), true);

    let resolved_width = if manage_width {
        content_width
            .map(|width| Size::Pixels(width.into()))
            .unwrap_or_else(|| scroll_width.get())
    } else {
        scroll_width.get()
    };
    if resolved_scroll_width.get() != resolved_width {
        resolved_scroll_width.set(resolved_width);
    }

    let resolved_height = if manage_height {
        content_height
            .map(|height| Size::Pixels(height.into()))
            .unwrap_or_else(|| scroll_height.get())
    } else {
        scroll_height.get()
    };
    if resolved_scroll_height.get() != resolved_height {
        resolved_scroll_height.set(resolved_height);
    }
}

#[cfg(test)]
mod scroll_position_tests;

#[cfg(test)]
mod tests {
    use super::resolve_axis_autosize;

    #[test]
    fn autosize_defaults_only_manage_y_for_scroller() {
        assert!(!resolve_axis_autosize(true, None, false));
        assert!(resolve_axis_autosize(true, None, true));
    }

    #[test]
    fn axis_override_precedence_beats_default_autosize_semantics() {
        assert!(resolve_axis_autosize(false, Some(true), false));
        assert!(!resolve_axis_autosize(true, Some(false), true));
    }
}
