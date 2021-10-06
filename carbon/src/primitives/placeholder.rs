use std::rc::Rc;

use core::cell::RefCell;
use core::option::Option;
use core::option::Option::{None, Some};
use kurbo::Affine;
use piet_web::WebRenderContext;
use crate::{RenderNodePtrList, RenderNode, PropertyTreeContext, Size, RenderTreeContext, rendering};

pub struct Placeholder {
    pub id: String,
    pub transform: Affine,
    children: RenderNodePtrList,
}

impl Placeholder {
    pub fn new(id: String, transform: Affine) -> Self {
        Placeholder {
            id,
            transform,
            children: Rc::new(RefCell::new(vec![])),
        }
    }
}


//TODO:  should `Placeholder` expose an explicit index property, so that
//       consumers can specify which index adoptee the placeholder should accept?
//       like <Placeholder index={{i}} />
//       or should we stick with this side-effectful "first come first served" approach?
//       Seems like the former is more robust, and the latter is a bit more "magical"
//       (one fewer button to press! but one more trick to learn.)
impl RenderNode for Placeholder {
    fn eval_properties_in_place(&mut self, _: &PropertyTreeContext) {
        //TODO: handle each of Group's `Expressable` properties
    }

    fn get_align(&self) -> (f64, f64) { (0.0,0.0) }
    fn get_children(&self) -> RenderNodePtrList {
        //NOTE: this relies on side-effects from elsewhere.
        //      Returning &self.children works because a child
        //      is side-effectfully added there during pre_render.
        Rc::clone(&self.children)
    }
    fn get_size(&self) -> Option<(Size<f64>, Size<f64>)> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
    fn get_id(&self) -> &str {
        &self.id.as_str()
    }
    fn get_origin(&self) -> (Size<f64>, Size<f64>) { (Size::Pixel(0.0), Size::Pixel(0.0)) }
    fn get_transform(&self) -> &Affine {
        &self.transform
    }
    fn pre_render(&mut self, rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {
        // grab the first adoptee from the current stack frame
        // and make it Placeholder's own child.
        //
        // this might be more elegant as a dynamic lookup inside the get_children
        // method, but at the time of authoring that would require refactoring
        // get_children to accept the RenderNodeContext, which zb opted not to do.
        self.children = match rtc.runtime.borrow_mut().peek_stack_frame() {
            Some(stack_frame) => {
                match stack_frame.borrow_mut().next_adoptee() {
                    Some(adoptee) => {
                        rendering::wrap_render_node_ptr_into_list(adoptee)
                    },
                    None => {Rc::new(RefCell::new(vec![]))}
                }
            },
            None => {Rc::new(RefCell::new(vec![]))}
        }
    }
    fn render(&self, _rtc: &mut RenderTreeContext, _rc: &mut WebRenderContext) {}
    fn post_render(&self, _rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {}
}
