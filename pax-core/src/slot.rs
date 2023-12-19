use core::cell::RefCell;
use core::option::Option;
use std::any::Any;

use std::rc::Rc;

use crate::{
    handle_vtable_update, with_properties_unwrapped, BaseInstance, ExpandedNode, InstanceFlags,
    InstanceNode, InstantiationArgs, PropertiesTreeContext,
};
use pax_runtime_api::{Layer, Numeric};

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
    pub index: Box<dyn pax_runtime_api::PropertyInstance<pax_runtime_api::Numeric>>,
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
                },
            ),
        })
    }

    fn expand(self: Rc<Self>, ptc: &mut PropertiesTreeContext) -> Rc<RefCell<ExpandedNode>> {
        let this_expanded_node = self
            .base()
            .expand_from_instance(Rc::clone(&self) as Rc<dyn InstanceNode>, ptc);
        let properties_wrapped = this_expanded_node.borrow().get_properties();

        //Similarly to Repeat, mark all existing expanded nodes for unmount, which will tactically be reverted later in this
        //method for attached nodes.  This enables changes / shifts in slot index + firing mount / unmount lifecycle events along the way.
        for cen in this_expanded_node.borrow().get_children_expanded_nodes() {
            ptc.engine
                .node_registry
                .borrow_mut()
                .mark_for_unmount(cen.borrow().id_chain.clone());
        }

        let current_index: usize = with_properties_unwrapped!(
            &properties_wrapped,
            SlotProperties,
            |properties: &mut SlotProperties| {
                handle_vtable_update!(ptc, this_expanded_node, properties.index, Numeric);
                properties
                    .index
                    .get()
                    .get_as_int()
                    .try_into()
                    .expect("Slot index must be non-negative")
            }
        );

        let ccc = ptc.current_containing_component.upgrade().unwrap();
        let cccb = ccc.borrow();
        let containing_component_flattened_slot_children =
            cccb.get_expanded_and_flattened_slot_children();

        if let Some(slot_children) = containing_component_flattened_slot_children {
            if let Some(child_to_forward) = slot_children.get(current_index) {
                this_expanded_node
                    .borrow_mut()
                    .append_child_expanded_node(Rc::clone(child_to_forward));

                child_to_forward.borrow_mut().parent_expanded_node =
                    Rc::downgrade(&this_expanded_node);

                ptc.engine
                    .node_registry
                    .borrow_mut()
                    .revert_mark_for_unmount(&child_to_forward.borrow().id_chain);
            }
        }

        this_expanded_node
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
