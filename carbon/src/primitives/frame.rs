
use std::rc::Rc;
use core::option::Option;
use core::option::Option::Some;
use kurbo::Affine;
use kurbo::BezPath;
use piet_web::WebRenderContext;
use crate::{RenderNodePtrList, Size, Property, RenderNode, PropertyTreeContext, RenderTreeContext, Transform};
use piet::RenderContext;

pub struct Frame {
    pub id: String,
    pub children: RenderNodePtrList,
    pub size: (
        Box<dyn Property<Size<f64>>>,
        Box<dyn Property<Size<f64>>>,
    ),
    pub transform: Transform,
}

impl RenderNode for Frame {
    fn eval_properties_in_place(&mut self, ptc: &PropertyTreeContext) {
        self.transform.eval_in_place(ptc);
        self.size.0.eval_in_place(ptc);
        self.size.1.eval_in_place(ptc);
        //TODO: handle each of Frame's `Expressable` properties
    }
    fn get_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.children)
    }
    fn get_size(&self) -> Option<(Size<f64>, Size<f64>)> {
        Some((*self.size.0.read(), *self.size.1.read()))
    }
    fn get_transform_computed(&self) -> &Affine {
        self.transform.get_cached_computed_value()
    }

    fn get_transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }

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
