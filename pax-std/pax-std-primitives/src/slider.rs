use pax_message::{AnyCreatePatch, NativeInterrupt, SliderPatch};
use pax_runtime::api::{use_RefCell, Layer, Property};
use pax_runtime::{
    BaseInstance, ExpandedNode, ExpandedNodeIdentifier, InstanceFlags, InstanceNode,
    InstantiationArgs, RuntimeContext,
};
use_RefCell!();
use pax_runtime_api::{borrow, borrow_mut};
use pax_std::primitives::Slider;
use std::collections::HashMap;
use std::rc::Rc;

use crate::patch_if_needed;

pub struct SliderInstance {
    base: BaseInstance,
    // Properties that listen to Text property changes, and computes
    // a patch in the case that they have changed + sends it as a native
    // message to the chassi. Since InstanceNode -> ExpandedNode has a one
    // to many relationship, needs to be a hashmap
    native_message_props: RefCell<HashMap<ExpandedNodeIdentifier, Property<()>>>,
}

impl InstanceNode for SliderInstance {
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
        borrow!(self.native_message_props)
            .get(&expanded_node.id)
            .unwrap()
            .get();
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        // Send creation message
        let id = expanded_node.id.clone();
        context.enqueue_native_message(pax_message::NativeMessage::SliderCreate(AnyCreatePatch {
            id: id.to_u32(),
            parent_frame: expanded_node.parent_frame.get().map(|v| v.to_u32()),
            occlusion_layer_id: 0,
        }));

        // send update message when relevant properties change
        let weak_self_ref = Rc::downgrade(&expanded_node);
        let context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(SliderPatch {
            id: id.to_u32(),
            ..Default::default()
        }));

        let deps: Vec<_> = borrow_mut!(expanded_node.properties_scope)
            .values()
            .cloned()
            .chain([
                expanded_node.layout_properties.transform.untyped(),
                expanded_node.layout_properties.bounds.untyped(),
            ])
            .collect();
        borrow_mut!(self.native_message_props).insert(
            id,
            Property::computed(
                move || {
                    let Some(expanded_node) = weak_self_ref.upgrade() else {
                        unreachable!()
                    };
                    let id = expanded_node.id.clone();
                    let mut old_state = borrow_mut!(last_patch);

                    let mut patch = SliderPatch {
                        id: id.to_u32(),
                        ..Default::default()
                    };
                    expanded_node.with_properties_unwrapped(|properties: &mut Slider| {
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
                            patch_if_needed(
                                &mut old_state.accent,
                                &mut patch.accent,
                                (&properties.accent.get()).into(),
                            ),
                            patch_if_needed(
                                &mut old_state.value,
                                &mut patch.value,
                                properties.value.get(),
                            ),
                            patch_if_needed(
                                &mut old_state.step,
                                &mut patch.step,
                                properties.step.get(),
                            ),
                            patch_if_needed(
                                &mut old_state.min,
                                &mut patch.min,
                                properties.min.get(),
                            ),
                            patch_if_needed(
                                &mut old_state.max,
                                &mut patch.max,
                                properties.max.get(),
                            ),
                        ];
                        if updates.into_iter().any(|v| v == true) {
                            context.enqueue_native_message(
                                pax_message::NativeMessage::SliderUpdate(patch),
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
        context.enqueue_native_message(pax_message::NativeMessage::SliderDelete(id.to_u32()));
        // Reset so that native_message sending updates while unmounted
        borrow_mut!(self.native_message_props).remove(&id);
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Slider").finish_non_exhaustive()
    }

    fn handle_native_interrupt(
        &self,
        expanded_node: &Rc<ExpandedNode>,
        interrupt: &NativeInterrupt,
    ) {
        if let NativeInterrupt::FormSliderChange(args) = interrupt {
            log::debug!("slider value set to {}", args.value);
            expanded_node
                .with_properties_unwrapped(|props: &mut Slider| props.value.set(args.value));
        }
    }
}
