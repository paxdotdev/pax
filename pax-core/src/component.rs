use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::rc::Rc;

use pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use crate::{RenderNode, RenderNodePtrList, RenderTreeContext, Scope, HostPlatformContext, HandlerRegistry, InstantiationArgs, RenderNodePtr};

use pax_runtime_api::{Timeline, Transform2D, Size2D, PropertyInstance, ArgsCoproduct};

/// A render node with its own runtime context.  Will push a frame
/// to the runtime stack including the specified `adoptees` and
/// `PropertiesCoproduct` object.  `Component` is used at the root of
/// applications, at the root of reusable components like `Spread`, and
/// in special applications like `Repeat` where it houses the `RepeatItem`
/// properties attached to each of Repeat's virtual nodes.
pub struct ComponentInstance {
    pub template: RenderNodePtrList,
    pub children: RenderNodePtrList,
    pub should_skip_adoption: bool,
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry>>>,
    pub transform: Rc<RefCell<dyn PropertyInstance<Transform2D>>>,
    pub properties: Rc<RefCell<PropertiesCoproduct>>,
    pub timeline: Option<Rc<RefCell<Timeline>>>,
    pub compute_properties_fn: Box<dyn FnMut(Rc<RefCell<PropertiesCoproduct>>,&mut RenderTreeContext)>,
}






fn flatten_adoptees(adoptees: RenderNodePtrList) -> RenderNodePtrList {
    let mut running_adoptees : Vec<RenderNodePtr> = vec![];

    (*adoptees).borrow_mut().iter().for_each(|node|{
        let node_borrowed = (**node).borrow();
        if node_borrowed.should_flatten() {
            let children = (*node_borrowed).get_rendering_children();
            (*children).borrow().iter().for_each(|child|{running_adoptees.push(Rc::clone(child))});
        } else {
            running_adoptees.push(Rc::clone(node));
        }
    });

    Rc::new(RefCell::new(running_adoptees))
}


//TODO:
//  - track internal playhead for this component


impl RenderNode for ComponentInstance {
    fn get_rendering_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.template)
    }

    fn get_handler_registry(&self) -> Option<Rc<RefCell<HandlerRegistry>>> {
        match &self.handler_registry {
            Some(registry) => {
                Some(Rc::clone(&registry))
            },
            _ => {None}
        }
    }

    fn instantiate(args: InstantiationArgs) -> Rc<RefCell<Self>> {
        let new_id = pax_runtime_api::mint_unique_id();

        let template = match args.component_template {
            Some(t) => t,
            None => Rc::new(RefCell::new(vec![])),
        };

        let ret = Rc::new(RefCell::new(ComponentInstance {
            template,
            children: match args.children {
                Some(children) => children,
                None => Rc::new(RefCell::new(vec![])),
            },
            transform: args.transform,
            properties: Rc::new(RefCell::new(args.properties)),
            compute_properties_fn: args.compute_properties_fn.expect("must pass a compute_properties_fn to a Component instance"),
            timeline: None,
            should_skip_adoption: args.should_skip_adoption,
            handler_registry: args.handler_registry,
        }));

        (*args.instance_map).borrow_mut().insert(new_id, Rc::clone(&ret) as Rc<RefCell<dyn RenderNode>>);
        ret
    }

    fn get_size(&self) -> Option<Size2D> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
    fn get_transform(&mut self) -> Rc<RefCell<dyn PropertyInstance<Transform2D>>> { Rc::clone(&self.transform) }
    fn compute_properties(&mut self, rtc: &mut RenderTreeContext) {
        let mut transform = &mut *self.transform.as_ref().borrow_mut();
        if let Some(new_transform) = rtc.get_computed_value(transform._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Transform2D(v) = new_transform { v } else { unreachable!() };
            transform.set(new_value);
        }
        (*self.compute_properties_fn)(Rc::clone(&self.properties), rtc);

        //TODO: adoptees need their properties calculated too!

        (*rtc.runtime).borrow_mut().push_stack_frame(
            Rc::clone(&self.children),
            Box::new(Scope {
                properties: Rc::clone(&self.properties)
            }),
            self.timeline.clone(),
            self.should_skip_adoption
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
