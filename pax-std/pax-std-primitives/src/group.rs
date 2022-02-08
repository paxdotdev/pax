use std::cell::RefCell;
use std::rc::Rc;

use pax_core::{RenderNode, RenderNodePtr, RenderNodePtrList};
use pax_core::pax_properties_coproduct::PropertiesCoproduct;
use pax_core::rendering::Size2D;

use pax_runtime_api::{Transform};

/// Gathers a set of children underneath a single render node:
/// useful for composing transforms and simplifying render trees.
pub struct GroupInstance {
    pub children: RenderNodePtrList,
    pub id: String,
    pub transform: Rc<RefCell<Transform>>,
}

impl GroupInstance {
    pub fn instantiate(properties: PropertiesCoproduct, transform: Transform, children: RenderNodePtrList) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            children,
            id: "".to_string(),
            transform: Rc::new(RefCell::new(transform))
        }))
    }
}

impl RenderNode for GroupInstance {
    fn get_rendering_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.children)
    }
    fn get_size(&self) -> Option<Size2D> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
    fn get_transform(&mut self) -> Rc<RefCell<Transform>> { Rc::clone(&self.transform) }
}
