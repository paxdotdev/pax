use std::iter;
use std::rc::Rc;

use crate::common::patch_if_needed;
use kurbo::{Affine, BezPath};
use pax_engine::*;
use pax_message::{AnyCreatePatch, FramePatch};
use pax_runtime::api::{Layer, Property, RenderContext};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use_RefCell!();
use pax_runtime::api::{borrow, borrow_mut, use_RefCell};

/// A primitive that gathers children underneath a single render node with a shared base transform,
/// like [`Group`], except [`Frame`] has the option of clipping rendering outside
/// of its bounds.
///
/// If clipping or the option of clipping is not required,
/// a [`Group`] will generally be a more performant and otherwise-equivalent
/// to [`Frame`], since `[Frame]` creates a clipping mask.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::core::frame::FrameInstance")]
#[custom(Default)]
pub struct Frame {
    pub _clip_content: Property<bool>,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            _clip_content: Property::new(true),
        }
    }
}

pub struct FrameInstance {
    base: BaseInstance,
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

    fn update(self: Rc<Self>, _expanded_node: &Rc<ExpandedNode>, _context: &Rc<RuntimeContext>) {}

    fn handle_pre_render(
        &self,
        expanded_node: &ExpandedNode,
        rtc: &Rc<RuntimeContext>,
        rcs: &mut dyn RenderContext,
    ) {
        let total_layer_count = rtc.layer_count.get();

        let mut run_pre_render = false;
        for i in 0..total_layer_count {
            run_pre_render |= rtc.is_canvas_dirty(&i);
        }

        if !run_pre_render {
            return;
        }

        if !expanded_node.with_properties_unwrapped(|frame: &mut Frame| frame._clip_content.get()) {
            return;
        }
        let t_and_b = expanded_node.transform_and_bounds.get();
        let transform = t_and_b.transform;
        let (width, height) = t_and_b.bounds;

        let mut bez_path = BezPath::new();
        bez_path.move_to((0.0, 0.0));
        bez_path.line_to((width, 0.0));
        bez_path.line_to((width, height));
        bez_path.line_to((0.0, height));
        bez_path.line_to((0.0, 0.0));
        bez_path.close_path();

        let transformed_bez_path = <Affine>::from(transform) * bez_path;

        let layers = rcs.layers();
        for layer in 0..layers {
            //our "save point" before clipping — restored to in the post_render
            rcs.save(layer);
            // rcs.clip(layer, transformed_bez_path.clone());
        }
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

        let total_layer_count = rtc.layer_count.get();

        let mut post_render = false;
        for i in 0..total_layer_count {
            post_render |= rtc.is_canvas_dirty(&i);
        }
        if !post_render {
            return;
        }

        let layers = rcs.layers();
        for layer in 0..layers {
            //pop the clipping context from the stack
            rcs.restore(layer);
        }
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
            occlusion_layer_id: 0,
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
            .chain([expanded_node.transform_and_bounds.untyped()])
            .collect();

        expanded_node
            .changed_listener
            .replace_with(Property::computed(
                move || {
                    let Some(expanded_node) = weak_self_ref.upgrade() else {
                        unreachable!()
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

                        let updates = [
                            patch_if_needed(
                                &mut old_state.clip_content,
                                &mut patch.clip_content,
                                properties._clip_content.get(),
                            ),
                            patch_if_needed(&mut old_state.size_x, &mut patch.size_x, width),
                            patch_if_needed(&mut old_state.size_y, &mut patch.size_y, height),
                            patch_if_needed(
                                &mut old_state.transform,
                                &mut patch.transform,
                                computed_tab.transform.coeffs().to_vec(),
                            ),
                        ];

                        if updates.into_iter().any(|v| v == true) {
                            cloned_context.enqueue_native_message(
                                pax_message::NativeMessage::FrameUpdate(patch),
                            );
                        }
                    });
                    cloned_context
                        .set_canvas_dirty(expanded_node.occlusion.get().occlusion_layer_id)
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
