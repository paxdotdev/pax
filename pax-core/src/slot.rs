use core::cell::RefCell;
use core::option::Option;
use core::option::Option::{None, Some};
use std::collections::HashMap;
use std::rc::Rc;

use pax_properties_coproduct::{TypesCoproduct, PropertiesCoproduct};
use piet_common::RenderContext;

use crate::{InstantiationArgs, InstanceNode, InstanceNodePtr, InstanceNodePtrList, RenderTreeContext, flatten_slot_invisible_nodes_recursive, ExpandedNode, PropertiesTreeContext};
use pax_runtime_api::{CommonProperties, Layer, PropertyInstance, Size};

/// A special "control-flow" primitive (a la `yield`) — represents a slot into which
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

    instance_prototypical_properties: Rc<RefCell<PropertiesCoproduct>>,
    instance_prototypical_common_properties: Rc<RefCell<CommonProperties>>,
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
            instance_prototypical_common_properties: Rc::new(RefCell::new(args.common_properties)),
            instance_prototypical_properties: Rc::new(RefCell::new(args.properties)),
            // index: args.slot_index.expect("index required for Slot"),
            // cached_computed_children: Rc::new(RefCell::new(vec![])),
        }));
        node_registry.register(instance_id, Rc::clone(&ret) as InstanceNodePtr<R>);
        ret
    }


    fn handle_pre_render(&mut self, rtc: &mut RenderTreeContext<R>, _rcs: &mut HashMap<String, R>) {
        // self.cached_computed_children = if let Some(sc) = rtc.current_containing_component.borrow().get_slot_children() {
        //     flatten_slot_invisible_nodes_recursive(sc)
        // } else {
        //     Rc::new(RefCell::new(vec![]))
        // }
    }


    /// Slot has strictly zero instance_children, but will likely have ExpandedNode children
    fn get_instance_children(&self) -> InstanceNodePtrList<R> {
        Rc::new(RefCell::new(vec![]))
    }

    fn expand_node(&mut self, ptc: &mut PropertiesTreeContext<R>) -> Rc<RefCell<ExpandedNode<R>>> {
        // if let Some(index) = ptc.compute_vtable_value(self.index._get_vtable_id()) {
        //     let new_value = if let TypesCoproduct::Numeric(v) = index {
        //         v
        //     } else {
        //         unreachable!()
        //     };
        //     self.index.set(new_value);
        // }

        todo!()
    }

    fn get_layer_type(&mut self) -> Layer {
        Layer::DontCare
    }
}