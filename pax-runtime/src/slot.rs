use core::option::Option;

use std::cell::RefCell;
use std::rc::Rc;

use pax_runtime_api::properties::Erasable;
use pax_runtime_api::{Numeric, Property};

use crate::api::Layer;
use crate::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};

/// A special "control-flow" primitive (a la `yield` or perhaps `goto`) — represents a slot into which
/// an slot_child can be rendered.  Slot relies on `slot_children` being present
/// on the [`Runtime`] stack and will not render any content if there are no `slot_children` found.
///
/// Consider a Stacker:  the owner of a Stacker passes the Stacker some nodes to render
/// inside the cells of the Stacker.  To the owner of the Stacker, those nodes might seem like
/// "children," but to the Stacker they are "slot_children" — children provided from
/// the outside.  Inside Stacker's template, there are a number of Slots — this primitive —
/// that become the final rendered home of those slot_children.  This same technique
/// is portable and applicable elsewhere via Slot.
pub struct SlotInstance {
    base: BaseInstance,
}

///Contains the index value for slot, either a literal or an expression.
#[derive(Default)]
pub struct SlotProperties {
    pub index: Property<Numeric>,
}

impl InstanceNode for SlotInstance {
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
                },
            ),
        })
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RefCell<RuntimeContext>>,
    ) {
        let cloned_expanded_node = Rc::clone(expanded_node);
        let cloned_context = Rc::clone(context);

        let dep = expanded_node
            .with_properties_unwrapped(|properties: &mut SlotProperties| properties.index.erase());
        expanded_node
            .children
            .replace_with(Property::computed_with_name(
                move || {
                    cloned_expanded_node.with_properties_unwrapped(
                        |properties: &mut SlotProperties| {
                            let index: usize = properties
                                .index
                                .get()
                                .to_int()
                                .try_into()
                                .expect("Slot index must be non-negative");

                            // TODO DAG: expanded_and_flattened_slot_children also need to be a property dependency
                            let node = cloned_expanded_node
                                .containing_component
                                .upgrade()
                                .as_ref()
                                .expect("slot has containing component during create")
                                .expanded_and_flattened_slot_children
                                .borrow()
                                .as_ref()
                                .and_then(|v| v.get(index))
                                .map(|v| Rc::clone(&v));

                            let ret = if let Some(node) = node {
                                let res = cloned_expanded_node
                                    .attach_children(vec![Rc::clone(&node)], &cloned_context);
                                res
                            } else {
                                cloned_expanded_node.generate_children(vec![], &cloned_context)
                            };
                            ret
                        },
                    )
                },
                &vec![&dep],
                &format!("slot_children (node id: {})", expanded_node.id_chain[0]),
            ));
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Slot").finish()
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}
