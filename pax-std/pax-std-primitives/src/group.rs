use std::cell::RefCell;
use std::rc::Rc;

use pax_core::{RenderNode, RenderNodePtr, RenderNodePtrList, Transform};
use pax_core::rendering::Size2D;

/// Gathers a set of children underneath a single render node:
/// useful for composing transforms and simplifying render trees.
/// #[pax primitive]
pub struct Group {
    pub children: Rc<RefCell<Vec<RenderNodePtr>>>,
    pub id: String,
    pub transform: Rc<RefCell<Transform>>,
}


impl RenderNode for Group {
    fn get_rendering_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.children)
    }
    fn get_size(&self) -> Option<Size2D> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
    fn get_transform(&mut self) -> Rc<RefCell<Transform>> { Rc::clone(&self.transform) }
}
