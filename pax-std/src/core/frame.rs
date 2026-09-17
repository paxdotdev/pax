use std::iter;
use std::rc::Rc;

use crate::common::{canvas_surface_transform, native_surface_opacity, patch_if_needed};
use kurbo::{Affine, BezPath, RoundedRect, Shape};
use pax_engine::*;
use pax_message::{AnyCreatePatch, FramePatch};
use pax_runtime::api::{bez_path_to_svg_path_data, Layer, Property, RenderContext};
use pax_runtime::{
    bind_content_measurement_effect, resolve_axis_autosize, sync_content_autosize_with_axes,
    BaseInstance, ContentMeasurementGeometry, ExpandedNode, InstanceFlags, InstanceNode,
    InstantiationArgs, RuntimeContext,
};
use_RefCell!();
use pax_runtime::api::{borrow, borrow_mut, use_RefCell};

fn mark_canvas_descendants_dirty(expanded_node: &ExpandedNode, context: &Rc<RuntimeContext>) {
    for child in expanded_node.children.get().iter() {
        if borrow!(child.instance_node).base().flags().layer == Layer::Canvas {
            context.mark_canvas_node_dirty(child.id);
            context.set_canvas_dirty(child.occlusion.get().render_layer_id);
        }
        mark_canvas_descendants_dirty(child, context);
    }
}

/// A primitive that gathers children underneath a single render node with a shared base transform,
/// like `Group`, except `Frame` has the option of clipping rendering outside
/// of its bounds.
///
/// If clipping or the option of clipping is not required,
/// a `Group` will generally be a more performant and otherwise-equivalent
/// to `Frame`, since `Frame` creates a clipping mask.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::core::frame::FrameInstance")]
#[custom(Default)]
pub struct Frame {
    /// Automatically sizes the frame to its direct content children when possible.
    pub autosize: Property<bool>,
    /// Optional override for whether autosize manages the `x` axis.
    pub autosize_x: Property<Option<bool>>,
    /// Optional override for whether autosize manages the `y` axis.
    pub autosize_y: Property<Option<bool>>,
    // Controls whether this frame clips descendants outside its bounds.
    pub _clip_content: Property<bool>,
    /// Corner radius used for the frame clipping mask, in pixels.
    pub corner_radius: Property<f64>,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            autosize: Property::new(false),
            autosize_x: Property::new(None),
            autosize_y: Property::new(None),
            _clip_content: Property::new(true),
            corner_radius: Property::new(0.0),
        }
    }
}

// Runtime instance backing `<Frame>`.
pub struct FrameInstance {
    base: BaseInstance,
}

impl FrameInstance {
    fn clip_path_in_space(expanded_node: &ExpandedNode, transform: Affine) -> Option<BezPath> {
        expanded_node.with_properties_unwrapped(|frame: &mut Frame| {
            frame._clip_content.get().then(|| {
                frame_clip_path(
                    expanded_node.transform_and_bounds.get().bounds,
                    frame.corner_radius.get(),
                    transform,
                )
            })
        })
    }
}

fn frame_clip_path(bounds: (f64, f64), corner_radius: f64, transform: Affine) -> BezPath {
    let (width, height) = bounds;
    let max_radius = 0.5 * width.max(0.0).min(height.max(0.0));
    let radius = corner_radius.clamp(0.0, max_radius);
    transform * RoundedRect::new(0.0, 0.0, width, height, radius).to_path(0.1)
}

impl InstanceNode for FrameInstance {
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
                    layer: Layer::DontCare,
                    is_component: false,
                    is_slot: false,
                },
            ),
        })
    }

    fn requires_non_reactive_update(&self, expanded_node: &ExpandedNode) -> bool {
        !expanded_node.content_measurement_bound.get()
    }

    fn update(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        if expanded_node.content_measurement_bound.get() {
            return;
        }
        let ctx = expanded_node.get_node_context(context);
        let (autosize, autosize_x, autosize_y) =
            expanded_node.with_properties_unwrapped(|frame: &mut Frame| {
                (
                    frame.autosize.clone(),
                    frame.autosize_x.clone(),
                    frame.autosize_y.clone(),
                )
            });
        let deps = [
            autosize.untyped(),
            autosize_x.untyped(),
            autosize_y.untyped(),
        ];
        bind_content_measurement_effect(
            expanded_node,
            &ctx,
            "frame autosize",
            ContentMeasurementGeometry::Placed,
            &deps,
            move |node, node_ctx| {
                sync_content_autosize_with_axes(
                    node,
                    node_ctx,
                    resolve_axis_autosize(autosize.get(), autosize_x.get(), true),
                    resolve_axis_autosize(autosize.get(), autosize_y.get(), true),
                );
            },
        );
    }

    fn resolve_effect_clip_path(&self, expanded_node: &ExpandedNode) -> Option<BezPath> {
        Self::clip_path_in_space(
            expanded_node,
            Affine::from(expanded_node.transform_and_bounds.get().transform),
        )
    }

    fn handle_pre_render(
        &self,
        expanded_node: &ExpandedNode,
        rtc: &Rc<RuntimeContext>,
        rcs: &mut dyn RenderContext,
    ) {
        // Only clip the node's own render layer; other layers can be hosted in different
        // DOM coordinate spaces (browser-owned scroller islands), so cross-layer clipping
        // can misalign and cull content.
        let layer_id = expanded_node.occlusion.get().render_layer_id;

        if !rtc.is_canvas_dirty(&layer_id) {
            return;
        }

        // Render clips share the leaf primitives' owning-surface coordinate space.
        // World-space paths remain available above for occlusion and hit testing.
        let Some(transformed_bez_path) =
            Self::clip_path_in_space(expanded_node, canvas_surface_transform(expanded_node, rtc))
        else {
            return;
        };

        // Clip stack nodes affect all descendant canvas draws on this layer. They intentionally
        // use the unbounded begin path; primitive tile culling happens at leaf draw nodes.
        if !rcs.begin_node(
            layer_id,
            expanded_node.id.to_u32(),
            expanded_node.occlusion.get().z_index,
            0,
        ) {
            return;
        }
        // our "save point" before clipping — restored to in the post_render
        rcs.save(layer_id);
        rcs.clip(layer_id, transformed_bez_path);
        let _ = rcs.end_node(layer_id, expanded_node.id.to_u32());
    }

    fn handle_post_render(
        &self,
        expanded_node: &ExpandedNode,
        rtc: &Rc<RuntimeContext>,
        rcs: &mut dyn RenderContext,
    ) {
        if !expanded_node.with_properties_unwrapped(|frame: &mut Frame| frame._clip_content.get()) {
            return;
        }

        let layer_id = expanded_node.occlusion.get().render_layer_id;

        if !rtc.is_canvas_dirty(&layer_id) {
            return;
        }

        // pop the clipping context from the stack
        rcs.restore(layer_id);
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        let id = expanded_node.id.clone();
        context.enqueue_native_message(pax_message::NativeMessage::FrameCreate(AnyCreatePatch {
            id: id.to_u32(),
            parent_frame: expanded_node.parent_frame.get().map(|v| v.to_u32()),
            render_layer_id: 0,
        }));

        // below is the same as default impl for adding children in instance_node
        let env = Rc::clone(&expanded_node.stack);
        let children = borrow!(self.base().get_instance_children());
        let children_with_envs = children.iter().cloned().zip(iter::repeat(env));

        // NOTE: overwrite frame to be a new prop for all deps
        let this_frame_prop = Property::new(Some(expanded_node.id));
        let new_children =
            expanded_node.generate_children(children_with_envs, context, &this_frame_prop, true);
        expanded_node.children.set(new_children);

        // send update message when relevant properties change
        let weak_self_ref = Rc::downgrade(&expanded_node);
        let cloned_context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(FramePatch {
            id: id.to_u32(),
            ..Default::default()
        }));

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
                    let id = expanded_node.id.to_u32();
                    let mut old_state = borrow_mut!(last_patch);

                    let mut patch = FramePatch {
                        id,
                        ..Default::default()
                    };
                    expanded_node.with_properties_unwrapped(|properties: &mut Frame| {
                        let computed_tab = expanded_node.transform_and_bounds.get();
                        let (width, height) = computed_tab.bounds;
                        let corner_radius = properties.corner_radius.get();
                        let max_radius = 0.5 * width.max(0.0).min(height.max(0.0));
                        let clamped_radius = corner_radius.clamp(0.0, max_radius);
                        let clip_path = if properties._clip_content.get()
                            && clamped_radius > f64::EPSILON
                        {
                            let rect = RoundedRect::new(0.0, 0.0, width, height, clamped_radius);
                            let bez_path = Affine::from(computed_tab.transform) * rect.to_path(0.1);
                            bez_path_to_svg_path_data(&bez_path)
                        } else {
                            String::new()
                        };

                        let updates = [
                            patch_if_needed(
                                &mut old_state.clip_content,
                                &mut patch.clip_content,
                                properties._clip_content.get(),
                            ),
                            patch_if_needed(
                                &mut old_state.corner_radius,
                                &mut patch.corner_radius,
                                corner_radius,
                            ),
                            patch_if_needed(&mut old_state.size_x, &mut patch.size_x, width),
                            patch_if_needed(&mut old_state.size_y, &mut patch.size_y, height),
                            patch_if_needed(
                                &mut old_state.transform,
                                &mut patch.transform,
                                computed_tab.transform.coeffs().to_vec(),
                            ),
                            patch_if_needed(
                                &mut old_state.clip_path,
                                &mut patch.clip_path,
                                clip_path,
                            ),
                            patch_if_needed(
                                &mut old_state.opacity,
                                &mut patch.opacity,
                                native_surface_opacity(&expanded_node, &cloned_context),
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
                        ];

                        if updates.into_iter().any(|v| v == true) {
                            cloned_context.enqueue_native_message(
                                pax_message::NativeMessage::FrameUpdate(patch),
                            );
                            mark_canvas_descendants_dirty(&expanded_node, &cloned_context);
                        }
                    });
                },
                &deps,
            ));
    }

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        let id = expanded_node.id.clone();
        // Reset so that native_message sending updates while unmounted
        expanded_node
            .changed_listener
            .replace_with(Property::default());
        context.enqueue_native_message(pax_message::NativeMessage::FrameDelete(id.to_u32()));
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        match expanded_node {
            Some(expanded_node) => expanded_node
                .with_properties_unwrapped(|_f: &mut Frame| f.debug_struct("Frame").finish()),
            None => f.debug_struct("Frame").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn clips_content(&self, expanded_node: &ExpandedNode) -> bool {
        expanded_node.with_properties_unwrapped(|props: &mut Frame| props._clip_content.get())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_clip_tracks_its_surface_without_reapplying_the_scroller_offset() {
        let scroller_world = Affine::translate((32.0, 80.0)) * Affine::scale(1.5);
        let tile_in_scroller = Affine::translate((288.0, 140.0));
        let tile_world = scroller_world * tile_in_scroller;
        let bounds = (288.0, 140.0);
        for radius in [0.0, 18.0, 500.0] {
            let world_path = frame_clip_path(bounds, radius, tile_world);
            let surface_path =
                frame_clip_path(bounds, radius, scroller_world.inverse() * tile_world);
            let expected = frame_clip_path(bounds, radius, tile_in_scroller);
            assert_eq!(surface_path.bounding_box(), expected.bounding_box());
            assert_eq!(
                (scroller_world * surface_path).bounding_box(),
                world_path.bounding_box()
            );
        }
    }
}
