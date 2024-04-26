use std::cell::RefCell;

use pax_message::{AnyCreatePatch, TextboxPatch};
use pax_runtime::api::{Layer, Property};
use pax_runtime::{
    BaseInstance, ExpandedNode, ExpandedNodeIdentifier, InstanceFlags, InstanceNode,
    InstantiationArgs, RuntimeContext,
};
use pax_std::primitives::Textbox;
use std::collections::HashMap;
use std::rc::Rc;

use crate::patch_if_needed;

pub struct TextboxInstance {
    base: BaseInstance,
    // Properties that listen to Text property changes, and computes
    // a patch in the case that they have changed + sends it as a native
    // message to the chassi. Since InstanceNode -> ExpandedNode has a one
    // to many relationship, needs to be a hashmap
    native_message_props: RefCell<HashMap<ExpandedNodeIdentifier, Property<()>>>,
}

impl InstanceNode for TextboxInstance {
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

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RefCell<RuntimeContext>>,
    ) {
        // Send creation message
        let id = expanded_node.id.clone();
        context
            .borrow_mut()
            .enqueue_native_message(pax_message::NativeMessage::TextboxCreate(AnyCreatePatch {
                id_chain: id.to_backwards_compatible_id_chain(),
                parent_frame: expanded_node.parent_frame.get().map(|v| v.to_u32()),
                occlusion_layer_id: 0,
            }));

        // send update message when relevant properties change
        let weak_self_ref = Rc::downgrade(&expanded_node);
        let context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(TextboxPatch {
            id_chain: id.to_backwards_compatible_id_chain(),
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
                    let id = expanded_node.id.clone();
                    let mut old_state = last_patch.borrow_mut();

                    let mut patch = TextboxPatch {
                        id_chain: id.to_backwards_compatible_id_chain(),
                        ..Default::default()
                    };
                    expanded_node.with_properties_unwrapped(|properties: &mut Textbox| {
                        let computed_tab = &expanded_node.layout_properties;
                        let (width, height) = computed_tab.bounds.get();
                        let updates = [
                            patch_if_needed(
                                &mut old_state.text,
                                &mut patch.text,
                                properties.text.get().string.clone(),
                            ),
                            patch_if_needed(&mut old_state.size_x, &mut patch.size_x, width),
                            patch_if_needed(&mut old_state.size_y, &mut patch.size_y, height),
                            patch_if_needed(
                                &mut old_state.transform,
                                &mut patch.transform,
                                computed_tab.transform.get().coeffs().to_vec(),
                            ),
                            patch_if_needed(
                                &mut old_state.style,
                                &mut patch.style,
                                (&properties.style.get()).into(),
                            ),
                            patch_if_needed(
                                &mut old_state.stroke_color,
                                &mut patch.stroke_color,
                                (&properties.stroke.get().color.get()).into(),
                            ),
                            patch_if_needed(
                                &mut old_state.stroke_width,
                                &mut patch.stroke_width,
                                properties.stroke.get().width.get().get_pixels(width),
                            ),
                            patch_if_needed(
                                &mut old_state.background,
                                &mut patch.background,
                                (&properties.background.get()).into(),
                            ),
                            patch_if_needed(
                                &mut old_state.border_radius,
                                &mut patch.border_radius,
                                properties.border_radius.get().to_float(),
                            ),
                            patch_if_needed(
                                &mut old_state.focus_on_mount,
                                &mut patch.focus_on_mount,
                                properties.focus_on_mount.get(),
                            ),
                        ];
                        if updates.into_iter().any(|v| v == true) {
                            context.borrow_mut().enqueue_native_message(
                                pax_message::NativeMessage::TextboxUpdate(patch),
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
            .enqueue_native_message(pax_message::NativeMessage::TextboxDelete(
                id.to_backwards_compatible_id_chain(),
            ));
        // Reset so that native_message sending updates while unmounted
        self.native_message_props.borrow_mut().remove(&id);
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Textbox").finish_non_exhaustive()
    }
}
