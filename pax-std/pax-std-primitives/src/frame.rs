use std::cell::RefCell;
use std::collections::HashMap;
use std::iter;
use std::rc::Rc;

use crate::patch_if_needed;
use kurbo::{Affine, BezPath};
use pax_message::{AnyCreatePatch, FramePatch};
use pax_runtime::api::{Layer, Property, RenderContext};
use pax_runtime::{
    BaseInstance, ExpandedNode, ExpandedNodeIdentifier, InstanceFlags, InstanceNode,
    InstantiationArgs, RuntimeContext,
};
use pax_std::primitives::Frame;

/// A primitive that gathers children underneath a single render node with a shared base transform,
/// like [`Group`], except [`Frame`] has the option of clipping rendering outside
/// of its bounds.
///
/// If clipping or the option of clipping is not required,
/// a [`Group`] will generally be a more performant and otherwise-equivalent
/// to [`Frame`], since `[Frame]` creates a clipping mask.
pub struct FrameInstance {
    base: BaseInstance,
    // Properties that listen to Text property changes, and computes
    // a patch in the case that they have changed + sends it as a native
    // message to the chassi. Since InstanceNode -> ExpandedNode has a one
    // to many relationship, needs to be a hashmap
    native_message_props: RefCell<HashMap<ExpandedNodeIdentifier, Property<()>>>,
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
                    layer: Layer::Canvas,
                    is_component: false,
                },
            ),
            native_message_props: Default::default(),
        })
    }

    fn update(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        _context: &Rc<RefCell<RuntimeContext>>,
    ) {
        //trigger computation of property that computes + sends native message update
        self.native_message_props
            .borrow()
            .get(&expanded_node.id)
            .unwrap()
            .get();
    }

    fn handle_pre_render(
        &self,
        expanded_node: &ExpandedNode,
        _context: &Rc<RefCell<RuntimeContext>>,
        rcs: &mut dyn RenderContext,
    ) {
        let transform = expanded_node.layout_properties.transform.get();
        let (width, height) = expanded_node.layout_properties.bounds.get();

        let mut bez_path = BezPath::new();
        bez_path.move_to((0.0, 0.0));
        bez_path.line_to((width, 0.0));
        bez_path.line_to((width, height));
        bez_path.line_to((0.0, height));
        bez_path.line_to((0.0, 0.0));
        bez_path.close_path();

        let transformed_bez_path = <Affine>::from(transform) * bez_path;

        let layers = rcs.layers();
        let layers: Vec<String> = layers.iter().map(|s| s.to_string()).collect();

        for layer in layers {
            //our "save point" before clipping — restored to in the post_render
            rcs.save(&layer);
            rcs.clip(&layer, transformed_bez_path.clone());
        }
    }

    fn handle_post_render(
        &self,
        _expanded_node: &ExpandedNode,
        _context: &Rc<RefCell<RuntimeContext>>,
        rcs: &mut dyn RenderContext,
    ) {
        let layers = rcs.layers();
        let layers: Vec<String> = layers.iter().map(|s| s.to_string()).collect();
        for layer in layers {
            //pop the clipping context from the stack
            rcs.restore(&layer);
        }
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RefCell<RuntimeContext>>,
    ) {
        let id = expanded_node.id.clone();
        context
            .borrow_mut()
            .enqueue_native_message(pax_message::NativeMessage::FrameCreate(AnyCreatePatch {
                id: id.to_u32(),
                parent_frame: expanded_node.parent_frame.get().map(|v| v.to_u32()),
                occlusion_layer_id: 0,
            }));

        // When a frame has been mounted (and it's sucessfully attached itself
        // already to it's own parent) it sets itself as it's parent frame, so
        // that children of this frame created below end up attaching to it
        let old_val = expanded_node.parent_frame.get();
        expanded_node.parent_frame.set(Some(expanded_node.id));

        // bellow is the same as default impl for adding children in instance_node
        let env = Rc::clone(&expanded_node.stack);
        let children = self.base().get_instance_children().borrow();
        let children_with_envs = children.iter().cloned().zip(iter::repeat(env));

        let new_children = expanded_node.generate_children(children_with_envs, context);
        expanded_node.children.set(new_children);

        // reset parent_frame. Needed for if multiple mounts/dissmounts of this
        // frame occurs
        expanded_node.parent_frame.set(old_val);

        // send update message when relevant properties change
        let weak_self_ref = Rc::downgrade(&expanded_node);
        let context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(FramePatch {
            id: id.to_u32(),
            ..Default::default()
        }));

        let deps: Vec<_> = expanded_node
            .properties_scope
            .borrow()
            .values()
            .cloned()
            .chain([
                expanded_node.layout_properties.transform.untyped(),
                expanded_node.layout_properties.bounds.untyped(),
            ])
            .collect();
        self.native_message_props.borrow_mut().insert(
            id,
            Property::computed(
                move || {
                    let Some(expanded_node) = weak_self_ref.upgrade() else {
                        unreachable!()
                    };
                    let id = expanded_node.id.to_u32();
                    let mut old_state = last_patch.borrow_mut();

                    let mut patch = FramePatch {
                        id,
                        ..Default::default()
                    };
                    expanded_node.with_properties_unwrapped(|_properties: &mut Frame| {
                        let computed_tab = &expanded_node.layout_properties;
                        let (width, height) = computed_tab.bounds.get();

                        let updates = [
                            patch_if_needed(&mut old_state.size_x, &mut patch.size_x, width),
                            patch_if_needed(&mut old_state.size_y, &mut patch.size_y, height),
                            patch_if_needed(
                                &mut old_state.transform,
                                &mut patch.transform,
                                computed_tab.transform.get().coeffs().to_vec(),
                            ),
                        ];

                        if updates.into_iter().any(|v| v == true) {
                            context.borrow_mut().enqueue_native_message(
                                pax_message::NativeMessage::FrameUpdate(patch),
                            );
                        }
                    });
                    ()
                },
                &deps,
            ),
        );
    }

    fn handle_unmount(
        &self,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RefCell<RuntimeContext>>,
    ) {
        let id = expanded_node.id.clone();
        context
            .borrow_mut()
            .enqueue_native_message(pax_message::NativeMessage::FrameDelete(id.to_u32()));
        // Reset so that native_message sending updates while unmounted
        self.native_message_props.borrow_mut().remove(&id);
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
}
