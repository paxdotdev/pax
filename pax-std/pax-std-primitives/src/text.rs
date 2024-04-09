use pax_message::{AnyCreatePatch, TextPatch};
use pax_runtime::api::{Layer, Property, RenderContext};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use pax_std::primitives::Text;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::patch_if_needed;

pub struct TextInstance {
    base: BaseInstance,
    // Properties that listen to Text property changes, and computes
    // a patch in the case that they have changed + sends it as a native
    // message to the chassi. Since InstanceNode -> ExpandedNode has a one
    // to many relationship, needs to be a hashmap
    native_message_props: RefCell<HashMap<u32, Property<()>>>,
}

impl InstanceNode for TextInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self>
    where
        Self: Sized,
    {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: true, //TODO make this optional?
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
            .get(&expanded_node.id_chain[0])
            .unwrap()
            .get();
    }

    fn render(
        &self,
        _expanded_node: &ExpandedNode,
        _context: &Rc<RefCell<RuntimeContext>>,
        _rc: &mut dyn RenderContext,
    ) {
        //no-op -- only native rendering for Text (unless/until we support rasterizing text, which Piet should be able to handle!)
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RefCell<RuntimeContext>>,
    ) {
        // Send creation message
        let id_chain = expanded_node.id_chain.clone();
        context
            .borrow_mut()
            .enqueue_native_message(pax_message::NativeMessage::TextCreate(AnyCreatePatch {
                id_chain: id_chain.clone(),
                clipping_ids: vec![],
                scroller_ids: vec![],
                z_index: 0,
            }));

        // send update message when relevant properties change
        let weak_self_ref = Rc::downgrade(&expanded_node);
        let context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(TextPatch {
            id_chain: id_chain.clone(),
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
            id_chain[0],
            Property::computed(
                move || {
                    let Some(expanded_node) = weak_self_ref.upgrade() else {
                        unreachable!()
                    };
                    let id_chain = expanded_node.id_chain.clone();
                    let mut old_state = last_patch.borrow_mut();

                    let mut patch = TextPatch {
                        id_chain: id_chain.clone(),
                        ..Default::default()
                    };
                    expanded_node.with_properties_unwrapped(|properties: &mut Text| {
                        let computed_tab = &expanded_node.layout_properties;
                        let (width, height) = computed_tab.bounds.get();

                        let updates = [
                            // Content
                            patch_if_needed(
                                &mut old_state.content,
                                &mut patch.content,
                                properties.text.get().string.clone(),
                            ),
                            // Styles
                            patch_if_needed(
                                &mut old_state.style,
                                &mut patch.style,
                                (&properties.style.get()).into(),
                            ),
                            patch_if_needed(
                                &mut old_state.style_link,
                                &mut patch.style_link,
                                (&properties.style_link.get()).into(),
                            ),
                            // Transform and bounds
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
                                pax_message::NativeMessage::TextUpdate(patch),
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
        let id_chain = expanded_node.id_chain.clone();
        let id = id_chain[0];
        context
            .borrow_mut()
            .enqueue_native_message(pax_message::NativeMessage::TextDelete(id_chain));
        // Reset so that native_message sending updates while unmounted
        self.native_message_props.borrow_mut().remove(&id);
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        match expanded_node {
            Some(expanded_node) => expanded_node.with_properties_unwrapped(|r: &mut Text| {
                f.debug_struct("Text").field("text", &r.text.get()).finish()
            }),
            None => f.debug_struct("Text").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}
