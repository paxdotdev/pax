use std::cell::RefCell;
use std::rc::Rc;

use crate::{PropertiesCoproduct, RenderNode, RenderNodePtrList, RenderTreeContext, Scope, Size2D, Transform, HostPlatformContext};
use crate::timeline::Timeline;

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
    pub timeline: Option<Rc<RefCell<Timeline>>>,

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
            self.timeline.clone(),
        );
    }

    fn post_render(&mut self, rtc: &mut RenderTreeContext, _hpc: &mut HostPlatformContext) {
        rtc.runtime.borrow_mut().pop_stack_frame();
        match &self.timeline {
            Some(timeline_rc) => {
                let mut timeline = timeline_rc.borrow_mut();
                if timeline.is_playing {
                    timeline.playhead_position += 1;
                    if timeline.playhead_position >= timeline.frame_count {
                        timeline.playhead_position = 0;
                    }
                }
            },
            None => (),
        }

    }
}
