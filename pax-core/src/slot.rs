use core::cell::RefCell;
use core::option::Option;
use core::option::Option::{None, Some};
use std::rc::Rc;
use std::collections::HashMap;
use pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use piet_common::RenderContext;

use crate::{InstantiationArgs, RenderNodePtr, RenderNodePtrList, RenderNode, RenderTreeContext, HandlerRegistry};
use pax_runtime_api::{PropertyInstance, Transform2D, Size2D, Layer};



/// A special "control-flow" primitive (a la `yield`) — represents a slot into which
/// an adoptee can be rendered.  Slot relies on `adoptees` being present
/// on the [`Runtime`] stack and will not render any content if there are no `adoptees` found.
///
/// Consider a Stacker:  the owner of a Stacker passes the Stacker some nodes to render
/// inside the cells of the Stacker.  To the owner of the Stacker, those nodes might seem like
/// "children," but to the Stacker they are "adoptees" — children provided from
/// the outside.  Inside Stacker's template, there are a number of Slots — this primitive —
/// that become the final rendered home of those adoptees.  This same technique
/// is portable and applicable elsewhere via Slot.
pub struct SlotInstance<R: 'static + RenderContext> {
    pub instance_id: u64,
    pub transform: Rc<RefCell<dyn PropertyInstance<Transform2D>>>,
    pub index: Box<dyn PropertyInstance<pax_runtime_api::Numeric>>,
    cached_computed_children: RenderNodePtrList<R>,

}


impl<R: 'static + RenderContext> RenderNode<R> for SlotInstance<R> {

    fn get_instance_id(&self) -> u64 {
        self.instance_id
    }
    fn instantiate(mut args: InstantiationArgs<R>) -> Rc<RefCell<Self>> where Self: Sized {
        let mut instance_registry = args.instance_registry.borrow_mut();
        let instance_id = instance_registry.mint_id();
        let ret  = Rc::new(RefCell::new(Self {
            instance_id,
            transform: args.transform,
            index: args.slot_index.expect("index required for Slot"),
            cached_computed_children: Rc::new(RefCell::new(vec![])),

        }));
        instance_registry.register(instance_id, Rc::clone(&ret) as RenderNodePtr<R>);
        ret
    }

    fn get_rendering_children(&self) -> RenderNodePtrList<R> {
        Rc::clone(&self.cached_computed_children)
    }

    fn get_size(&self) -> Option<Size2D> { None }
    fn compute_size_within_bounds(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }

    fn get_transform(&mut self) -> Rc<RefCell<dyn PropertyInstance<Transform2D>>> { Rc::clone(&self.transform) }

    fn compute_properties(&mut self, rtc: &mut RenderTreeContext<R>) {

        if let Some(index) = rtc.compute_vtable_value(self.index._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Numeric(v) = index { v } else { unreachable!() };
            self.index.set(new_value);
        }

        // The following sort of children-caching is done by "control flow" primitives
        // (Slot, Repeat, If) —
        self.cached_computed_children = match rtc.runtime.borrow_mut().peek_stack_frame() {
            Some(stack_frame) => {
                // Grab the adoptee from the current stack_frame at Slot's specified `index`
                // then make it Slot's own child.
                match stack_frame.borrow().nth_adoptee(self.index.get().get_as_int() as usize) {
                    Some(rnp) => Rc::new(RefCell::new(vec![Rc::clone(&rnp)])),
                    None => Rc::new(RefCell::new(vec![])),
                }
            },
            None => {Rc::new(RefCell::new(vec![]))}
        }
    }

    fn get_layer_type(&mut self) -> Layer {
        Layer::DontCare
    }
}
