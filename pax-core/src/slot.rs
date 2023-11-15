use core::cell::RefCell;
use core::option::Option;
use core::option::Option::{None, Some};
use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;

use piet_common::RenderContext;

use crate::{
    flatten_slot_invisible_nodes_recursive, handle_vtable_update, with_properties_unwrapped,
    ExpandedNode, InstanceNode, InstanceNodePtr, InstanceNodePtrList, InstantiationArgs,
    PropertiesTreeContext, RenderTreeContext,
};
use pax_runtime_api::{CommonProperties, Layer, Numeric, PropertyInstance, Size};

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
    pub instance_id: u32,
    // pub index: Box<dyn PropertyInstance<pax_runtime_api::Numeric>>,
    // cached_computed_children: InstanceNodePtrList<R>,
    instance_prototypical_properties: Rc<RefCell<dyn Any>>,
    instance_prototypical_common_properties: Rc<RefCell<CommonProperties>>,
}

///Contains the index value for slot, either a literal or an expression.
#[derive(Default)]
pub struct SlotProperties {
    pub index: Box<dyn pax_runtime_api::PropertyInstance<pax_runtime_api::Numeric>>,
}

impl<R: 'static + RenderContext> InstanceNode<R> for SlotInstance {
    fn get_instance_id(&self) -> u32 {
        self.instance_id
    }
    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>>
    where
        Self: Sized,
    {
        let mut node_registry = args.node_registry.borrow_mut();
        let instance_id = node_registry.mint_instance_id();
        let ret = Rc::new(RefCell::new(Self {
            instance_id,
            instance_prototypical_common_properties: args.common_properties,
            instance_prototypical_properties: args.properties,
        }));
        node_registry.register(instance_id, Rc::clone(&ret) as InstanceNodePtr<R>);
        ret
    }

    fn handle_pre_render(&mut self, rtc: &mut RenderTreeContext<R>, _rcs: &mut HashMap<String, R>) {
    }

    /// Slot has strictly zero instance_children, but will likely have ExpandedNode children
    fn get_instance_children(&self) -> InstanceNodePtrList<R> {
        Rc::new(RefCell::new(vec![]))
    }

    /// Slot manages own subtree because it wants to strictly terminate — properties for its children should
    /// already have been computed
    fn manages_own_subtree_for_expansion(&self) -> bool {
        true
    }

    fn expand_node_and_compute_properties(
        &mut self,
        ptc: &mut PropertiesTreeContext<R>,
    ) -> Rc<RefCell<ExpandedNode<R>>> {
        let this_expanded_node = ExpandedNode::get_or_create_with_prototypical_properties(
            ptc,
            &self.instance_prototypical_properties,
            &self.instance_prototypical_common_properties,
        );
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
                handle_vtable_update!(ptc, properties.index, Numeric);
                properties
                    .index
                    .get()
                    .get_as_int()
                    .try_into()
                    .expect("Slot index must be non-negative")
            }
        );

        let ccc = ptc.current_containing_component.as_ref().unwrap();
        let cccb = ccc.borrow();
        let containing_component_flattened_slot_children =
            cccb.get_expanded_and_flattened_slot_children();

        if let Some(slot_children) = containing_component_flattened_slot_children {
            if let Some(child_to_forward) = slot_children.get(current_index) {
                this_expanded_node
                    .borrow_mut()
                    .append_child_expanded_node(Rc::clone(child_to_forward));
                ptc.engine
                    .node_registry
                    .borrow_mut()
                    .revert_mark_for_unmount(&child_to_forward.borrow().id_chain);
            }
        }

        this_expanded_node
    }

    fn get_layer_type(&mut self) -> Layer {
        Layer::DontCare
    }
}
