use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use piet_common::RenderContext;

use pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use crate::{RenderNode, RenderNodePtrList, RenderTreeContext, HandlerRegistry, InstantiationArgs, RenderNodePtr, Runtime, LifecycleNode, TabCache};

use pax_runtime_api::{Timeline, Transform2D, Size2D, PropertyInstance};

/// A render node with its own runtime context.  Will push a frame
/// to the runtime stack including the specified `adoptees` and
/// `PropertiesCoproduct` object.  `Component` is used at the root of
/// applications, at the root of reusable components like `Stacker`, and
/// in special applications like `Repeat` where it houses the `RepeatItem`
/// properties attached to each of Repeat's virtual nodes.
pub struct ComponentInstance<R: 'static + RenderContext> {
    pub(crate) instance_id: u64,
    pub template: RenderNodePtrList<R>,
    pub children: RenderNodePtrList<R>,
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry<R>>>>,
    pub transform: Rc<RefCell<dyn PropertyInstance<Transform2D>>>,
    pub properties: Rc<RefCell<PropertiesCoproduct>>,
    pub timeline: Option<Rc<RefCell<Timeline>>>,
    pub compute_properties_fn: Box<dyn FnMut(Rc<RefCell<PropertiesCoproduct>>,&mut RenderTreeContext<R>)>,
}

impl<R: 'static + RenderContext> RenderNode<R> for ComponentInstance<R> {

    fn get_instance_id(&self) -> u64 {
        self.instance_id
    }
    fn get_rendering_children(&self) -> RenderNodePtrList<R> {
        Rc::clone(&self.template)
    }

    fn get_handler_registry(&self) -> Option<Rc<RefCell<HandlerRegistry<R>>>> {
        match &self.handler_registry {
            Some(registry) => {
                Some(Rc::clone(&registry))
            },
            _ => {None}
        }
    }

    fn handle_did_render(&mut self, rtc: &mut RenderTreeContext<R>, _rcs: &mut Vec<R>) {
        (*rtc.runtime).borrow_mut().pop_stack_frame();
    }

    fn instantiate(mut args: InstantiationArgs<R>) -> Rc<RefCell<Self>> {
        let mut instance_registry = (*args.instance_registry).borrow_mut();
        let instance_id = instance_registry.mint_id();

        let template = match args.component_template {
            Some(t) => t,
            None => Rc::new(RefCell::new(vec![])),
        };

        let ret = Rc::new(RefCell::new(ComponentInstance {
            instance_id,
            template,
            children: match args.children {
                Some(children) => children,
                None => Rc::new(RefCell::new(vec![])),
            },
            transform: args.transform,
            properties: Rc::new(RefCell::new(args.properties)),
            compute_properties_fn: args.compute_properties_fn.expect("must pass a compute_properties_fn to a Component instance"),
            timeline: None,
            handler_registry: args.handler_registry,
        }));

        instance_registry.register(instance_id, Rc::clone(&ret) as RenderNodePtr<R>);
        ret
    }

    fn get_size(&self) -> Option<Size2D> { None }
    fn compute_size_within_bounds(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
    fn get_transform(&mut self) -> Rc<RefCell<dyn PropertyInstance<Transform2D>>> { Rc::clone(&self.transform) }
    fn compute_properties(&mut self, rtc: &mut RenderTreeContext<R>) {
        let mut transform = &mut *self.transform.as_ref().borrow_mut();
        if let Some(new_transform) = rtc.compute_vtable_value(transform._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Transform2D(v) = new_transform { v } else { unreachable!() };
            transform.set(new_value);
        }
        (*self.compute_properties_fn)(Rc::clone(&self.properties), rtc);

        //expand adoptees before adding to stack frame.
        //NOTE: this requires *evaluating properties* for `should_flatten` nodes like Repeat and Conditional, whose
        //      properties must be evaluated before we can know how to handle them as adoptees
        let unflattened_adoptees = Rc::clone(&self.children);

        let flattened_adoptees = Rc::new(RefCell::new(
            (*unflattened_adoptees).borrow().iter().map(|adoptee| {
                Runtime::process__should_flatten__adoptees_recursive(adoptee, rtc)
            }).flatten().collect()
        ));

        (*rtc.runtime).borrow_mut().push_stack_frame(
            flattened_adoptees,
            Rc::clone(&self.properties),
            self.timeline.clone(),
        );
    }


}

