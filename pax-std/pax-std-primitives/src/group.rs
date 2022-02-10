use std::cell::RefCell;
use std::rc::Rc;

use pax_core::{RenderNode, RenderNodePtr, RenderNodePtrList};
use pax_core::pax_properties_coproduct::PropertiesCoproduct;

use pax_runtime_api::{Transform, Size2D, Property};

/// Gathers a set of children underneath a single render node:
/// useful for composing transforms and simplifying render trees.
pub struct GroupInstance {
    pub children: RenderNodePtrList,
    pub id: String,
    pub transform: Rc<RefCell<dyn Property<Transform>>>,
}

impl GroupInstance {
    pub fn instantiate(properties: PropertiesCoproduct, transform: Rc<RefCell<dyn Property<Transform>>>, children: RenderNodePtrList) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            children,
            id: "".to_string(),
            transform,
        }))
    }
}

impl RenderNode for GroupInstance {
    fn get_rendering_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.children)
    }
    fn get_size(&self) -> Option<Size2D> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
    fn get_transform(&mut self) -> Rc<RefCell<dyn Property<Transform>>> { Rc::clone(&self.transform) }
}
