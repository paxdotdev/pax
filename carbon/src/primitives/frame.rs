use core::option::Option;
use core::option::Option::Some;
use std::cell::RefCell;
use std::rc::Rc;

use kurbo::Affine;
use kurbo::BezPath;
use piet::RenderContext;
use piet_web::WebRenderContext;

use crate::{Property, RenderNode, RenderNodePtrList, RenderTreeContext, Size, Transform};
use crate::rendering::Size2D;

pub struct Frame {
    pub children: RenderNodePtrList,
    pub size: Size2D,
    pub transform: Rc<RefCell<Transform>>,
}

impl RenderNode for Frame {
    fn get_rendering_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.children)
    }
    fn get_size(&self) -> Option<Size2D> {
        Some(Rc::clone(&self.size))
    }

    fn get_transform(&mut self) -> Rc<RefCell<Transform>> { Rc::clone(&self.transform) }

    fn compute_properties(&mut self, rtc: &mut RenderTreeContext) {
        self.size.borrow_mut().0.compute_in_place(rtc);
        self.size.borrow_mut().1.compute_in_place(rtc);
        self.transform.borrow_mut().compute_in_place(rtc);
    }

    fn pre_render(&mut self, rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {
        // construct a BezPath of this frame's bounds * its transform,
        // then pass that BezPath into rc.clip() [which pushes a clipping context to a piet-internal stack]
        //TODO:  if clipping is TURNED OFF for this Frame, don't do any of this

        let transform = rtc.transform;
        let bounding_dimens = rtc.bounds;

        let width: f64 =  bounding_dimens.0;
        let height: f64 =  bounding_dimens.1;

        let mut bez_path = BezPath::new();
        bez_path.move_to((0.0, 0.0));
        bez_path.line_to((width , 0.0));
        bez_path.line_to((width , height ));
        bez_path.line_to((0.0, height));
        bez_path.line_to((0.0,0.0));
        bez_path.close_path();

        let transformed_bez_path = transform * bez_path;
        rc.save(); //our "save point" before clipping — restored to in the post_render
        rc.clip(transformed_bez_path);
    }
    fn post_render(&self, rtc: &mut RenderTreeContext, rc: &mut WebRenderContext) {
        //pop the clipping context from the stack
        rc.restore();
    }
}
