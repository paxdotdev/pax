use std::cell::RefCell;
use std::rc::Rc;

use pax_core::{HandlerRegistry, HostPlatformContext, InstanceMap, InstantiationArgs, RenderNode, RenderNodePtr, RenderNodePtrList, RenderTreeContext};
use pax_core::pax_properties_coproduct::PropertiesCoproduct;

use pax_runtime_api::{Transform2D, Size2D, Property, ArgsCoproduct};

/// Gathers a set of children underneath a single render node:
/// useful for composing transforms and simplifying render trees.
pub struct GroupInstance {
    pub primitive_children: RenderNodePtrList,
    pub id: String,
    pub transform: Rc<RefCell<dyn Property<Transform2D>>>,
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry>>>,
}

impl GroupInstance {

}

impl RenderNode for GroupInstance {

    fn get_rendering_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.primitive_children)
    }

    fn instantiate(args: InstantiationArgs) -> Rc<RefCell<Self>> where Self: Sized {
        let new_id = pax_runtime_api::mint_unique_id();
        let ret = Rc::new(RefCell::new(Self {
            primitive_children: match args.primitive_children {
                None => {Rc::new(RefCell::new(vec![]))}
                Some(children) => children
            },
            id: "".to_string(),
            transform: args.transform,
            handler_registry: args.handler_registry,
        }));

        args.instance_map.borrow_mut().insert(new_id, Rc::clone(&ret) as Rc<RefCell<dyn RenderNode>>);
        ret
    }

    fn get_handler_registry(&self) -> Option<Rc<RefCell<HandlerRegistry>>> {
        match &self.handler_registry {
            Some(registry) => {
                Some(Rc::clone(&registry))
            },
            _ => {None}
        }
    }

    fn get_size(&self) -> Option<Size2D> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
    fn get_transform(&mut self) -> Rc<RefCell<dyn Property<Transform2D>>> { Rc::clone(&self.transform) }
}
