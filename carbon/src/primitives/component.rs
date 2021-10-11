use std::cell::RefCell;
use std::rc::Rc;

use piet_web::WebRenderContext;

use crate::{PropertiesCoproduct, RenderNode, RenderNodePtrList, RenderTreeContext, Scope, Size2D, Transform};


/// A render node with its own runtime context.  Will push a frame
/// to the runtime stack including the specified `adoptees` and
/// `PropertiesCoproduct` object.  `Component` is used at the root of
/// applications, at the root of reusable components like `Spread`, and
/// in special applications like `Repeat` where it houses the `RepeatItem`
/// properties attached to each of Repeat's virtual nodes.
pub struct Component {
    pub template: RenderNodePtrList,
    pub adoptees: RenderNodePtrList,
    pub transform: Rc<RefCell<Transform>>,
    pub properties: Rc<RefCell<PropertiesCoproduct>>,
    pub timeline_frame_count: usize,
    //TODO: private
    pub timeline_playhead_position: usize,
    pub timeline_is_playing: bool,
}

//TODO:
//  - track internal playhead for this component

impl RenderNode for Component {
    fn get_rendering_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.template)
    }
    fn get_size(&self) -> Option<Size2D> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
    fn get_transform(&mut self) -> Rc<RefCell<Transform>> { Rc::clone(&self.transform) }
    fn compute_properties(&mut self, rtc: &mut RenderTreeContext) {
        rtc.runtime.borrow_mut().push_stack_frame(
            Rc::clone(&self.adoptees),
            Box::new(Scope {
                properties: Rc::clone(&self.properties)
            }),
            self.timeline_playhead_position,
        );
    }



    fn post_render(&mut self, rtc: &mut RenderTreeContext, _rc: &mut WebRenderContext) {
        rtc.runtime.borrow_mut().pop_stack_frame();

        if self.timeline_is_playing {
            self.timeline_playhead_position += 1;
            if self.timeline_playhead_position >= self.timeline_frame_count {
                self.timeline_playhead_position = 0;
            }
        }
    }
}
