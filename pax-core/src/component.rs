use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::rc::Rc;

use pax_properties_coproduct::{PropertiesCoproduct};
use crate::{RenderNode, RenderNodePtrList, RenderTreeContext, Scope, HostPlatformContext, HandlerRegistry};

use pax_runtime_api::{Timeline, Transform, Size2D, Property, ArgsCoproduct};

/// A render node with its own runtime context.  Will push a frame
/// to the runtime stack including the specified `adoptees` and
/// `PropertiesCoproduct` object.  `Component` is used at the root of
/// applications, at the root of reusable components like `Spread`, and
/// in special applications like `Repeat` where it houses the `RepeatItem`
/// properties attached to each of Repeat's virtual nodes.
pub struct ComponentInstance {
    pub template: RenderNodePtrList,
    pub adoptees: RenderNodePtrList,
    pub transform: Rc<RefCell<dyn Property<Transform>>>,
    pub properties: Rc<RefCell<PropertiesCoproduct>>,
    pub timeline: Option<Rc<RefCell<Timeline>>>,
    pub compute_properties_fn: Box<dyn FnMut(Rc<RefCell<PropertiesCoproduct>>,&mut RenderTreeContext)>,
    pub dispatch_event_fn: Box<dyn FnMut(Rc<RefCell<PropertiesCoproduct>>, ArgsCoproduct)>,
}





//TODO:
//  - track internal playhead for this component

impl RenderNode for ComponentInstance {
    fn get_rendering_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.template)
    }

    fn dispatch_event(&mut self, args: ArgsCoproduct) {
        (*self.dispatch_event_fn)(Rc::clone(&self.properties), args);
    }

    fn get_size(&self) -> Option<Size2D> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
    fn get_transform(&mut self) -> Rc<RefCell<dyn Property<Transform>>> { Rc::clone(&self.transform) }
    fn compute_properties(&mut self, rtc: &mut RenderTreeContext) {
        (*self.compute_properties_fn)(Rc::clone(&self.properties), rtc);
        (*rtc.runtime).borrow_mut().push_stack_frame(
            Rc::clone(&self.adoptees),
            Box::new(Scope {
                properties: Rc::clone(&self.properties)
            }),
            self.timeline.clone(),
        );
    }

    fn post_render(&mut self, rtc: &mut RenderTreeContext, _hpc: &mut HostPlatformContext) {
        (*rtc.runtime).borrow_mut().pop_stack_frame();
        match &self.timeline {
            Some(timeline_rc) => {
                let mut timeline = (**timeline_rc).borrow_mut();
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
