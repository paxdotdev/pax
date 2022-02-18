use std::cell::RefCell;
use std::rc::Rc;

use pax_core::{HandlerRegistry, HostPlatformContext, InstanceMap, RenderNode, RenderNodePtr, RenderNodePtrList, RenderTreeContext};
use pax_core::pax_properties_coproduct::PropertiesCoproduct;

use pax_runtime_api::{Transform, Size2D, Property, ArgsCoproduct};

/// Gathers a set of children underneath a single render node:
/// useful for composing transforms and simplifying render trees.
pub struct GroupInstance {
    pub children: RenderNodePtrList,
    pub id: String,
    pub transform: Rc<RefCell<dyn Property<Transform>>>,
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry>>>,
}

impl GroupInstance {
    pub fn instantiate(handler_registry: Option<Rc<RefCell<HandlerRegistry>>>, instance_map: Rc<RefCell<InstanceMap>>, properties: PropertiesCoproduct, transform: Rc<RefCell<dyn Property<Transform>>>, children: RenderNodePtrList) -> Rc<RefCell<Self>> {
        let new_id = pax_runtime_api::generate_unique_id();
        let ret = Rc::new(RefCell::new(Self {
            children,
            id: "".to_string(),
            transform,
            handler_registry,
        }));

        instance_map.borrow_mut().insert(new_id, Rc::clone(&ret) as Rc<RefCell<dyn RenderNode>>);
        ret
    }
}

impl RenderNode for GroupInstance {

    fn get_rendering_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.children)
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
    fn get_transform(&mut self) -> Rc<RefCell<dyn Property<Transform>>> { Rc::clone(&self.transform) }
}
