use core::option::Option;
use core::option::Option::Some;
use std::cell::RefCell;
use std::rc::Rc;

use kurbo::BezPath;
use piet::RenderContext;

use pax_core::{RenderNode, RenderNodePtrList, RenderTreeContext, HostPlatformContext, InstantiationArgs};
use pax_properties_coproduct::TypesCoproduct;
use pax_runtime_api::{Transform2D, Size, PropertyInstance, Size2D};

/// A primitive that gathers children underneath a single render node with a shared base transform,
/// like [`Group`], except [`Frame`] has the option of clipping rendering outside
/// of its bounds.
///
/// If clipping or the option of clipping is not required,
/// a [`Group`] will generally be a more performant and otherwise-equivalent ]
/// option since clipping is expensive.
pub struct FrameInstance {
    pub children: RenderNodePtrList,
    pub size: Size2D,
    pub transform: Rc<RefCell<dyn PropertyInstance<Transform2D>>>,
}

impl RenderNode for FrameInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<RefCell<Self>> where Self: Sized {
        //TODO: add to instance_map!
        Rc::new(RefCell::new(
            Self {
                children: args.children.expect("Frame expects primitive_children, even if empty Vec"),
                size: Rc::new(RefCell::new(args.size.expect("Frame requires size"))),
                transform: args.transform,
            }
        ))
    }

    fn get_rendering_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.children)
    }

    fn get_size(&self) -> Option<Size2D> {
        Some(Rc::clone(&self.size))
    }

    fn get_transform(&mut self) -> Rc<RefCell<dyn PropertyInstance<Transform2D>>> { Rc::clone(&self.transform) }

    fn compute_properties(&mut self, rtc: &mut RenderTreeContext) {
        let mut size = &mut *self.size.as_ref().borrow_mut();

        if let Some(new_size) = rtc.get_vtable_computed_value(size[0]._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Size(v) = new_size { v } else { unreachable!() };
            size[0].set(new_value);
        }

        if let Some(new_size) = rtc.get_vtable_computed_value(size[1]._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Size(v) = new_size { v } else { unreachable!() };
            size[1].set(new_value);
        }

        let mut transform = &mut *self.transform.as_ref().borrow_mut();
        if let Some(new_transform) = rtc.get_vtable_computed_value(transform._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Transform2D(v) = new_transform { v } else { unreachable!() };
            transform.set(new_value);
        }
    }

    fn pre_render(&mut self, rtc: &mut RenderTreeContext, hpc: &mut HostPlatformContext) {
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
        hpc.drawing_context.save().unwrap(); //our "save point" before clipping — restored to in the post_render
        hpc.drawing_context.clip(transformed_bez_path);
    }
    fn post_render(&mut self, _rtc: &mut RenderTreeContext, hpc: &mut HostPlatformContext) {
        //pop the clipping context from the stack
        hpc.drawing_context.restore().unwrap();
    }
}
