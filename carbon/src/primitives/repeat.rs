use std::cell::RefCell;
use std::rc::Rc;

use piet_web::WebRenderContext;

use crate::{Affine, PropertyTreeContext, RenderNode, RenderNodePtr, RenderNodePtrList, RenderTreeContext, Size};

pub struct Repeat<T> {
    pub children: Rc<RefCell<Vec<RenderNodePtr>>>,
    pub list: Vec<T>,
    pub id: String,
    pub transform: Affine,
}

impl<T> RenderNode for Repeat<T> {
    fn eval_properties_in_place(&mut self, _: &PropertyTreeContext) {
        //TODO: handle each of Repeat's `Expressable` properties


        self.children = Rc::new(RefCell::new(
            self.list.iter().enumerate().map(|(i, datum)|{
                //TODO: assemble stack frame scope: index, datum.
                //      How do we pass that scope to the duplicated nodes?
                //      Should we construct a "puppeteer" node that
                //         1. pushes the stack frame for us, and
                //         2. delegates the rendering to our duplicated node

                // 1. construct a `puppeteer` node,
                //     - pass it the stack_frame details (i, datum)
                // 2. Attach a copy of each child of this `repeat` node
                //     as a child of `puppeteer`
                // 3. write logic in `puppeteer` that delegates rendering to its contained nodes
                // 4. evaluate if we need to support any flattening fanciness around here
                

                let children_borrowed = self.children.borrow();

            }).collect()
        ))
    }

    fn get_align(&self) -> (f64, f64) {
        (0.0, 0.0)
    }
    fn should_flatten(&self) -> bool {
        true
    }
    fn get_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.children)
    }
    fn get_size(&self) -> Option<(Size<f64>, Size<f64>)> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
    fn get_id(&self) -> &str {
        &self.id.as_str()
    }
    fn get_origin(&self) -> (Size<f64>, Size<f64>) {
        (Size::Pixel(0.0), Size::Pixel(0.0))
    }
    fn get_transform(&self) -> &Affine {
        &self.transform
    }
    fn pre_render(&mut self, _rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {}
    fn render(&self, _rtc: &mut RenderTreeContext, _rc: &mut WebRenderContext) {}
    fn post_render(&self, _rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {}
}
