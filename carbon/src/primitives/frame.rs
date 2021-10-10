
use std::rc::Rc;
use core::option::Option;
use core::option::Option::Some;
use kurbo::Affine;
use kurbo::BezPath;
use piet_web::WebRenderContext;
use crate::{RenderNodePtrList, Size, Property, RenderNode, PropertyTreeContext, RenderTreeContext, Transform, Size2D};
use piet::RenderContext;
use std::cell::RefCell;

pub struct Frame {
    pub id: String,
    pub children: RenderNodePtrList,
    pub size: Size2D,
    pub transform: Rc<RefCell<Transform>>,
}

impl RenderNode for Frame {
    fn eval_properties_in_place(&mut self, ptc: &PropertyTreeContext) {
        self.transform.borrow_mut().eval_in_place(ptc);
        self.size.borrow_mut().0.eval_in_place(ptc);
        self.size.borrow_mut().1.eval_in_place(ptc);
        //TODO: handle each of Frame's `Expressable` properties
    }
    fn get_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.children)
    }
    fn get_size(&self) -> Option<Size2D> {
        Some(Rc::clone(&self.size))
    }

    fn get_transform_mut(&mut self) -> Rc<RefCell<Transform>> { Rc::clone(&self.transform) }

    fn pre_render(&mut self, rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {

        // construct a BezPath of this frame's bounds * its transform,
        // then pass that BezPath into rc.clip() [which pushes a clipping context to a piet-internal stack]
        //TODO:  if clipping is TURNED OFF for this Frame, don't do any of this
        let transform = rtc.transform;
        let bounding_dimens = rtc.bounding_dimens;
        let width: f64 =  bounding_dimens.0;
        let height: f64 =  bounding_dimens.1;

        let mut bez_path = BezPath::new();
        bez_path.move_to((0.0, 0.0));
        bez_path.line_to((width , 0.0));
        bez_path.line_to((width , height ));
        bez_path.line_to((0.0, height));
        bez_path.line_to((0.0,0.0));
        bez_path.close_path();

        let transformed_bez_path = *transform * bez_path;
        rc.save(); //our "save point" before clipping — restored to in the post_render
        rc.clip(transformed_bez_path);
    }
    fn render(&self, _rtc: &mut RenderTreeContext, _rc: &mut WebRenderContext) {}
    fn post_render(&self, _rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {
        //pop the clipping context from the stack
        rc.restore();
    }
}
