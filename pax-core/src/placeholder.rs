use core::cell::RefCell;
use core::option::Option;
use core::option::Option::{None, Some};
use std::rc::Rc;
use pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};

use crate::{InstantiationArgs, RenderNode, RenderNodePtrList, RenderTreeContext};
use pax_runtime_api::{Property, Transform, Size2D};



/// A special "control-flow" primitive (a la `yield`) — represents a slot into which
/// an adoptee can be rendered.  Placeholder relies on `adoptees` being present
/// on the [`Runtime`] stack and will not render any content if there are no `adoptees` found.
///
/// Consider a Spread:  the owner of a Spread passes the Spread some nodes to render
/// inside the cells of the Spread.  To the owner of the Spread, those nodes might seem like
/// "children," but to the Spread they are "adoptees" — children provided from
/// the outside.  Inside Spread's template, there are a number of Placeholders — this primitive —
/// that become the final rendered home of those adoptees.  This same technique
/// is portable and applicable elsewhere via Placeholder.
pub struct Placeholder {
    pub transform: Rc<RefCell<dyn Property<Transform>>>,
    pub index: Box<dyn Property<usize>>,
    cached_computed_children: RenderNodePtrList,
}


impl RenderNode for Placeholder {

    fn instantiate(args: InstantiationArgs) -> Rc<RefCell<Self>> where Self: Sized {
        let new_id = pax_runtime_api::mint_unique_id();
        Rc::new(RefCell::new(Self {
            transform: args.transform,
            index: args.placeholder_index.expect("index required for Placeholder"),
            cached_computed_children: Rc::new(RefCell::new(vec![]))
        }))
    }

    fn get_rendering_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.cached_computed_children)
    }

    fn get_size(&self) -> Option<Size2D> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }

    fn get_transform(&mut self) -> Rc<RefCell<dyn Property<Transform>>> { Rc::clone(&self.transform) }

    fn compute_properties(&mut self, rtc: &mut RenderTreeContext) {

        if let Some(index) = rtc.get_computed_value(self.index._get_vtable_id()) {
            let new_value = if let TypesCoproduct::usize(v) = index { v } else { unreachable!() };
            self.index.set(new_value);
        }

        // The following sort of children-caching is done by "control flow" primitives
        // (Placeholder, Repeat, If) —
        self.cached_computed_children = match rtc.runtime.borrow_mut().peek_stack_frame() {
            Some(stack_frame) => {
                // Grab the adoptee from the current stack_frame at Placeholder's specified `index`
                // then make it Placeholder's own child.
                match stack_frame.borrow().get_adoptees().borrow().get(*self.index.get()) {
                    Some(rnp) => Rc::new(RefCell::new(vec![Rc::clone(&rnp)])),
                    None => Rc::new(RefCell::new(vec![])),
                }
            },
            None => {Rc::new(RefCell::new(vec![]))}
        }
    }
}
