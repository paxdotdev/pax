use core::cell::RefCell;
use core::option::Option;
use core::option::Option::{None, Some};
use std::rc::Rc;

use crate::{PropertyValue, RenderNode, RenderNodePtrList, RenderTreeContext, Transform};
use crate::rendering::Size2D;

pub struct Placeholder {
    pub transform: Rc<RefCell<Transform>>,
    pub index: Box<dyn PropertyValue<usize>>,
    children: RenderNodePtrList,
}

/// A special "control-flow" primitive: represents a slot into which
/// an adoptee can be rendered.  Placeholder relies on `adoptees` being present
/// on the [`Runtime`] stack and will not render any content of there are no `adoptees` found.
///
/// Consider a Spread:  the owner of a Spread passes the Spread some nodes to render
/// inside the cells of the Spread.  To the owner of the Spread, those nodes might seem like
/// "children," but to the Spread they are "adoptees" — children provided from
/// the outside.  Inside Spread's template, there are a number of Placeholders — this primitive —
/// that become the final rendered home of those adoptees.  This same technique
/// is portable and applicable elsewhere via Placeholder.
impl Placeholder {
    pub fn new(transform: Transform, index: Box<dyn PropertyValue<usize>>) -> Self {
        Placeholder {
            transform: Rc::new(RefCell::new(transform)),
            index,
            children: Rc::new(RefCell::new(vec![])),
        }
    }
}

impl RenderNode for Placeholder {

    fn compute_properties(&mut self, rtc: &mut RenderTreeContext) {
        self.index.compute_in_place(rtc);
        // The following sort of children-caching is done by "control flow" primitives
        // (Placeholder, Repeat, If) —
        self.children = match rtc.runtime.borrow_mut().peek_stack_frame() {
            Some(stack_frame) => {
                // Grab the adoptee from the current stack_frame at Placeholder's specified `index`
                // then make it Placeholder's own child.
                match stack_frame.borrow().get_adoptees().borrow().get(*self.index.read()) {
                    Some(rnp) => Rc::new(RefCell::new(vec![Rc::clone(&rnp)])),
                    None => Rc::new(RefCell::new(vec![])),
                }
            },
            None => {Rc::new(RefCell::new(vec![]))}
        }
    }

    fn get_rendering_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.children)
    }

    fn get_size(&self) -> Option<Size2D> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }

    fn get_transform(&mut self) -> Rc<RefCell<Transform>> { Rc::clone(&self.transform) }
}
