use core::cell::RefCell;
use core::option::Option;
use core::option::Option::{None, Some};
use std::rc::Rc;

use kurbo::Affine;
use piet_web::WebRenderContext;

use crate::{Property, PropertyTreeContext, rendering, RenderNode, RenderNodePtrList, RenderTreeContext, Size, Transform, wrap_render_node_ptr_into_list};
use crate::rendering::Size2D;

pub struct Placeholder {
    pub transform: Rc<RefCell<Transform>>,
    pub index: Box<dyn Property<usize>>,
    children: RenderNodePtrList,
}

impl Placeholder {
    pub fn new(transform: Transform, index: Box<Property<usize>>) -> Self {
        Placeholder {
            transform: Rc::new(RefCell::new(transform)),
            index,
            children: Rc::new(RefCell::new(vec![])),
        }
    }
}

impl RenderNode for Placeholder {
    fn eval_properties_in_place(&mut self, ptc: &PropertyTreeContext) {
        //TODO: handle each of Placeholder's `Expressable` properties

        self.index.eval_in_place(ptc);
        // The following sort of children-caching is done by "control flow" primitives
        // (Placeholder, Repeat, If) —
        self.children = match ptc.runtime.borrow_mut().peek_stack_frame() {
            Some(stack_frame) => {
                // Grab the adoptee from the current stack_frame at Placeholder's specified `index`
                // then make it Placeholder's own child.
                match stack_frame.borrow().get_adoptees().borrow().get(*self.index.read()) {
                    Some(rnp) => wrap_render_node_ptr_into_list(Rc::clone(&rnp)),
                    None => Rc::new(RefCell::new(vec![])),
                }
            },
            None => {Rc::new(RefCell::new(vec![]))}
        }
    }

    fn get_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.children)
    }

    fn get_size(&self) -> Option<Size2D> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }

    fn get_transform(&mut self) -> Rc<RefCell<Transform>> { Rc::clone(&self.transform) }
}
