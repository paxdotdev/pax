use std::cell::RefCell;
use std::rc::Rc;
use piet_common::RenderContext;
use pax_core::{HandlerRegistry, InstanceRegistry, InstantiationArgs, RenderNode, RenderNodePtr, RenderNodePtrList, RenderTreeContext, TransformAndBounds};
use pax_core::pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};

use pax_runtime_api::{Transform2D, Size2D, PropertyInstance, Layer};

/// Gathers a set of children underneath a single render node:
/// useful for composing transforms and simplifying render trees.
pub struct GroupInstance<R: 'static + RenderContext> {
    pub instance_id: u32,
    pub primitive_children: RenderNodePtrList<R>,
    pub transform: Rc<RefCell<dyn PropertyInstance<Transform2D>>>,
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry<R>>>>,

}

impl<R: 'static + RenderContext> RenderNode<R> for GroupInstance<R> {

    fn get_instance_id(&self) -> u32 {
        self.instance_id
    }

    fn get_rendering_children(&self) -> RenderNodePtrList<R> {
        Rc::clone(&self.primitive_children)
    }

    fn instantiate(mut args: InstantiationArgs<R>) -> Rc<RefCell<Self>> where Self: Sized {
        let mut instance_registry = args.instance_registry.borrow_mut();
        let instance_id = instance_registry.mint_id();
        let ret = Rc::new(RefCell::new(Self {
            instance_id,
            primitive_children: match args.children {
                None => {Rc::new(RefCell::new(vec![]))}
                Some(children) => children
            },
            transform: args.transform,
            handler_registry: args.handler_registry,

        }));

        instance_registry.register(instance_id, Rc::clone(&ret) as RenderNodePtr<R>);
        ret
    }

    fn get_handler_registry(&self) -> Option<Rc<RefCell<HandlerRegistry<R>>>> {
        match &self.handler_registry {
            Some(registry) => {
                Some(Rc::clone(&registry))
            },
            _ => {None}
        }
    }

    /// Can never hit a Group directly -- can only hit elements inside of it.
    /// Events can still be propagated to a group.
    fn ray_cast_test(&self, ray: &(f64, f64), tab: &TransformAndBounds) -> bool {
        false
    }

    fn get_layer_type(&mut self) -> Layer {
        Layer::DontCare
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
    }
}
