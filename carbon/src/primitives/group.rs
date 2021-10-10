use std::cell::RefCell;
use std::rc::Rc;

use piet_web::WebRenderContext;

use crate::{Affine, PropertyTreeContext, RenderNode, RenderNodePtr, RenderNodePtrList, RenderTreeContext, Size, Transform};
use crate::rendering::Size2D;

pub struct Group {
    pub children: Rc<RefCell<Vec<RenderNodePtr>>>,
    pub id: String,
    pub transform: Rc<RefCell<Transform>>,
}

impl RenderNode for Group {
    fn eval_properties_in_place(&mut self, _: &PropertyTreeContext) {
        //TODO: handle each of Group's `Expressable` properties
    }

    fn get_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.children)
    }
    fn get_size(&self) -> Option<Size2D> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }

    fn get_transform(&mut self) -> Rc<RefCell<Transform>> { Rc::clone(&self.transform) }

}
