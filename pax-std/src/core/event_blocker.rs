use std::collections::HashMap;
use std::iter;
use std::rc::Rc;

use crate::patch_if_needed;
use pax_engine::pax;
use pax_message::{AnyCreatePatch, EventBlockerPatch};
use pax_runtime::api::{Layer, Property};
use pax_runtime::{
    BaseInstance, ExpandedNode, ExpandedNodeIdentifier, InstanceFlags, InstanceNode,
    InstantiationArgs, RuntimeContext,
};

use std::cell::RefCell;

#[pax]
#[primitive("pax_std::core::event_blocker::EventBlockerInstance")]
pub struct EventBlocker {}

pub struct EventBlockerInstance {
    base: BaseInstance,
    // Properties that listen to EventBlocker property changes, and computes
    // a patch in the case that they have changed + sends it as a native
    // message to the chassi. Since InstanceNode -> ExpandedNode has a one
    // to many relationship, needs to be a hashmap
    native_message_props: RefCell<HashMap<ExpandedNodeIdentifier, Property<()>>>,
}

impl InstanceNode for EventBlockerInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self>
    where
        Self: Sized,
    {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: false,
                    layer: Layer::Native,
                    is_component: false,
                },
            ),
            native_message_props: Default::default(),
        })
    }

    fn update(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, _context: &Rc<RuntimeContext>) {
        //trigger computation of property that computes + sends native message update
        self.native_message_props
            .borrow()
            .get(&expanded_node.id)
            .unwrap()
            .get();
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        let id = expanded_node.id.clone();
        context.enqueue_native_message(pax_message::NativeMessage::EventBlockerCreate(
            AnyCreatePatch {
                id: id.to_u32(),
                parent_frame: expanded_node.parent_frame.get().map(|v| v.to_u32()),
                occlusion_layer_id: 0,
            },
        ));

        // bellow is the same as default impl for adding children in instance_node
        let env = Rc::clone(&expanded_node.stack);
        let children = self.base().get_instance_children().borrow();
        let children_with_envs = children.iter().cloned().zip(iter::repeat(env));

        let new_children = expanded_node.generate_children(children_with_envs, context);
        expanded_node.children.set(new_children);

        // send update message when relevant properties change
        let weak_self_ref = Rc::downgrade(&expanded_node);
        let context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(EventBlockerPatch {
            id: id.to_u32(),
            ..Default::default()
        }));

        let deps: Vec<_> = expanded_node.properties_scope
            .borrow()
            .values()
            .cloned()
            .chain([expanded_node.transform_and_bounds.untyped()])
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

                    let mut patch = EventBlockerPatch {
                        id,
                        ..Default::default()
                    };
                    expanded_node.with_properties_unwrapped(|_properties: &mut EventBlocker| {
                        let computed_tab = expanded_node.transform_and_bounds.get();
                        let (width, height) = computed_tab.bounds;

                        let updates = [
                            patch_if_needed(&mut old_state.size_x, &mut patch.size_x, width),
                            patch_if_needed(&mut old_state.size_y, &mut patch.size_y, height),
                            patch_if_needed(
                                &mut old_state.transform,
                                &mut patch.transform,
                                computed_tab.transform.coeffs().to_vec(),
                            ),
                        ];

                        if updates.into_iter().any(|v| v == true) {
                            context.enqueue_native_message(
                                pax_message::NativeMessage::EventBlockerUpdate(patch),
                            );
                        }
                    });
                    ()
                },
                &deps,
            ),
        );
    }

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        let id = expanded_node.id.clone();
        context.enqueue_native_message(pax_message::NativeMessage::EventBlockerDelete(id.to_u32()));
        // Reset so that native_message sending updates while unmounted
        self.native_message_props.borrow_mut().remove(&id);
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        match expanded_node {
            Some(expanded_node) => {
                expanded_node.with_properties_unwrapped(|_f: &mut EventBlocker| {
                    f.debug_struct("EventBlocker").finish()
                })
            }
            None => f.debug_struct("EventBlocker").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}
