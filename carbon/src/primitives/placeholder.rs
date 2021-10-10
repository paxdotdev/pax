use std::rc::Rc;

use core::cell::RefCell;
use core::option::Option;
use core::option::Option::{None, Some};
use kurbo::Affine;
use piet_web::WebRenderContext;
use crate::{RenderNodePtrList, RenderNode, PropertyTreeContext, Size, RenderTreeContext, rendering, wrap_render_node_ptr_into_list, Transform, Property};

pub struct Placeholder {
    pub id: String,
    pub transform: Transform,
    pub index: Box<Property<usize>>,
    children: RenderNodePtrList,
}

impl Placeholder {
    pub fn new(id: String, transform: Transform, index: Box<Property<usize>>) -> Self {
        Placeholder {
            id,
            transform,
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
                    //no adoptees found
                    //TODO:  should we recurse up the stack continuing our
                    //       quest for adoptees?  use-case:
                    //       we have a Spread with adoptees, attached to its top-level component
                    //       stack frame.  We use repeat, which pushes "puppeteer" stack frames
                    //       for each repeated datum.  Inside repeat, we have placeholders,
                    //       which expect to access the top-level component's adoptees.
                    //       YES, we need to traverse stack frames in this case.

                    //       What about in the case of an application component exposing
                    None => Rc::new(RefCell::new(vec![])),
                }
            },
            None => {Rc::new(RefCell::new(vec![]))}
        }
    }

    fn get_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.children)
    }

    fn get_size(&self) -> Option<(Size<f64>, Size<f64>)> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }

    fn get_transform_computed(&self) -> &Affine {
        &self.transform.cached_computed_transform
    }

    fn get_transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }

    fn pre_render(&mut self, rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {

    }
    fn render(&self, _rtc: &mut RenderTreeContext, _rc: &mut WebRenderContext) {}
    fn post_render(&self, _rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {}
}
