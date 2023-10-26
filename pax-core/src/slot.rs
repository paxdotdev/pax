use core::cell::RefCell;
use core::option::Option;
use core::option::Option::{None, Some};
use std::collections::HashMap;
use std::rc::Rc;

use pax_properties_coproduct::{TypesCoproduct, PropertiesCoproduct};
use piet_common::RenderContext;

use crate::{InstantiationArgs, RenderNode, RenderNodePtr, RenderNodePtrList, RenderTreeContext, flatten_slot_invisible_nodes_recursive};
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
pub struct SlotInstance<R: 'static + RenderContext> {
    pub instance_id: u32,
    pub index: Box<dyn PropertyInstance<pax_runtime_api::Numeric>>,
    pub common_properties: CommonProperties,
    cached_computed_children: RenderNodePtrList<R>,
}

impl<R: 'static + RenderContext> RenderNode<R> for SlotInstance<R> {
    fn get_common_properties(&self) -> &CommonProperties {
        &self.common_properties
    }

    fn get_properties(&self) -> Rc<RefCell<PropertiesCoproduct>> {
        Rc::new(RefCell::new(PropertiesCoproduct::None))
    }

    fn get_instance_id(&self) -> u32 {
        self.instance_id
    }
    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>>
    where
        Self: Sized,
    {
        let mut instance_registry = args.instance_registry.borrow_mut();
        let instance_id = instance_registry.mint_instance_id();
        let ret = Rc::new(RefCell::new(Self {
            instance_id,
            common_properties: args.common_properties,
            index: args.slot_index.expect("index required for Slot"),
            cached_computed_children: Rc::new(RefCell::new(vec![])),
        }));
        instance_registry.register(instance_id, Rc::clone(&ret) as RenderNodePtr<R>);
        ret
    }


    fn handle_will_render(&mut self, rtc: &mut RenderTreeContext<R>, _rcs: &mut HashMap<String, R>) {
        self.cached_computed_children = if let Some(sc) = rtc.current_containing_component.borrow().get_slot_children() {
            flatten_slot_invisible_nodes_recursive(sc)
        } else {
            Rc::new(RefCell::new(vec![]))
        }
    }

    fn get_rendering_children(&self) -> RenderNodePtrList<R> {
        Rc::clone(&self.cached_computed_children)
    }

    fn get_size(&self) -> Option<(Size, Size)> {
        None
    }
    fn compute_size_within_bounds(&self, bounds: (f64, f64)) -> (f64, f64) {
        bounds
    }

    fn handle_compute_properties(&mut self, rtc: &mut RenderTreeContext<R>) {
        if let Some(index) = rtc.compute_vtable_value(self.index._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Numeric(v) = index {
                v
            } else {
                unreachable!()
            };
            self.index.set(new_value);
        }

    }

    fn get_layer_type(&mut self) -> Layer {
        Layer::DontCare
    }
}
