use core::option::Option::Some;
use pax_runtime::{BaseInstance, ExpandedNodeIdentifier, InstanceFlags, RuntimeContext};
use pax_runtime_api::{borrow, borrow_mut, use_RefCell};
use pax_std::primitives::Scrollbar;
use_RefCell!();
use std::collections::HashMap;
use std::rc::Rc;

use pax_message::{AnyCreatePatch, ScrollerPatch};
use pax_runtime::api::{Layer, Property};
use pax_runtime::{ExpandedNode, InstanceNode, InstantiationArgs};

use crate::patch_if_needed;

/// A combination of a clipping area (nearly identical to a `Frame`,) and an
/// inner panel that can be scrolled on zero or more axes.  `Scroller` coordinates with each chassis to
/// create native scrolling containers, which pass native scroll events back to Engine.  In turn,
/// `Scroller` translates its children to reflect the current scroll position.
/// When both scrolling axes are disabled, `Scroller` acts exactly like a `Frame`, with a possibly-
/// transformed `Group` surrounding its contents.
pub struct ScrollbarInstance {
    base: BaseInstance,
    // Properties that listen to Text property changes, and computes
    // a patch in the case that they have changed + sends it as a native
    // message to the chassi. Since InstanceNode -> ExpandedNode has a one
    // to many relationship, needs to be a hashmap
    native_message_props: RefCell<HashMap<ExpandedNodeIdentifier, Property<()>>>,
}

impl InstanceNode for ScrollbarInstance {
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
        expanded_node.property_scope_manager.run_with_scope(|| {
        // Send creation message
        let id = expanded_node.id.to_u32();
        context.enqueue_native_message(pax_message::NativeMessage::ScrollerCreate(
            AnyCreatePatch {
                id,
                parent_frame: expanded_node.parent_frame.get().map(|v| v.to_u32()),
                occlusion_layer_id: 0,
            },
        ));

        // send update message when relevant properties change
        let weak_self_ref = Rc::downgrade(&expanded_node);
        let context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(ScrollerPatch {
            id,
            ..Default::default()
        }));

        let deps: Vec<_> = borrow!(expanded_node.properties_scope)
            .values()
            .cloned()
            .chain([expanded_node.transform_and_bounds.get_id()])
            .collect();
        borrow_mut!(self.native_message_props).insert(
            expanded_node.id,
            Property::expression(
                move || {
                    let Some(expanded_node) = weak_self_ref.upgrade() else {
                        unreachable!()
                    };
                    let mut old_state = borrow_mut!(last_patch);

                    let mut patch = ScrollerPatch {
                        id,
                        ..Default::default()
                    };
                    expanded_node.with_properties_unwrapped(|properties: &mut Scrollbar| {
                        let computed_tab = expanded_node.transform_and_bounds.get();
                        let (width, height) = computed_tab.bounds;
                        let updates = [
                            patch_if_needed(
                                &mut old_state.size_inner_pane_x,
                                &mut patch.size_inner_pane_x,
                                properties.size_inner_pane_x.get().get_pixels(width),
                            ),
                            patch_if_needed(
                                &mut old_state.size_inner_pane_y,
                                &mut patch.size_inner_pane_y,
                                properties.size_inner_pane_y.get().get_pixels(height),
                            ),
                            patch_if_needed(&mut old_state.size_x, &mut patch.size_x, width),
                            patch_if_needed(&mut old_state.size_y, &mut patch.size_y, height),
                            patch_if_needed(
                                &mut old_state.scroll_x,
                                &mut patch.scroll_x,
                                properties.scroll_x.get(),
                            ),
                            patch_if_needed(
                                &mut old_state.scroll_y,
                                &mut patch.scroll_y,
                                properties.scroll_y.get(),
                            ),
                            patch_if_needed(
                                &mut old_state.scroll_x,
                                &mut patch.scroll_x,
                                properties.scroll_x.get(),
                            ),
                            patch_if_needed(
                                &mut old_state.transform,
                                &mut patch.transform,
                                computed_tab.transform.coeffs().to_vec(),
                            ),
                        ];
                        if updates.into_iter().any(|v| v == true) {
                            context.enqueue_native_message(
                                pax_message::NativeMessage::ScrollerUpdate(patch),
                            );
                        }
                    });
                    ()
                },
                &deps, "ScrollerInstance::update"
            ),
        );
        });
    }

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        let id = expanded_node.id.to_u32();
        context.enqueue_native_message(pax_message::NativeMessage::ScrollerDelete(id));
        // Reset so that native_message sending updates while unmounted
        borrow_mut!(self.native_message_props).remove(&expanded_node.id);
    }
    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Scrollbar").finish_non_exhaustive()
    }
}