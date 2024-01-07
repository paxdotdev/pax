use kurbo::BezPath;
use piet::RenderContext;

use pax_core::{
    HandlerRegistry, InstanceNode, InstanceNodePtr, InstanceNodePtrList, InstantiationArgs,
    PropertiesComputable, RenderTreeContext,
};
use pax_runtime_api::{CommonProperties, Size};
use pax_std::primitives::Path;
use pax_std::types::PathSegment;

use std::cell::RefCell;
use std::rc::Rc;

/// A basic 2D vector path for arbitrary Bézier / line-segment chains
pub struct PathInstance<R: 'static + RenderContext> {
    base: BaseInstance,
}

impl<R: 'static + RenderContext> InstanceNode<R> for PathInstance<R> {
    fn get_instance_children(&self) -> InstanceNodePtrList<R> {
        Rc::new(RefCell::new(vec![]))
    }

    fn new(args: InstantiationArgs<R>) -> Rc<RefCell<Self>>
    where
        Self: Sized,
    {
        Rc::new(RefCell::new(Self {
            base: BaseInstance::new(args),
            last_patches: Default::default(),
        }))
    }

    fn get_size(&self) -> Option<(Size, Size)> {
        None
    }

    fn expand_node_and_compute_properties(&mut self, rtc: &mut RenderTreeContext<R>) {
        // let properties = &mut *self.properties.as_ref().borrow_mut();
        //
        // if let Some(stroke_width) =
        //     rtc.compute_vtable_value(properties.stroke.get().width._get_vtable_id())
        // {
        //     let new_value =
        //         unsafe_unwrap!(stroke_width, TypesCoproduct, pax_runtime_api::SizePixels);
        //     properties.stroke.get_mut().width.set(new_value);
        // }
        //
        // if let Some(stroke_color) =
        //     rtc.compute_vtable_value(properties.stroke.get().color._get_vtable_id())
        // {
        //     let new_value = unsafe_unwrap!(stroke_color, TypesCoproduct, pax_std::types::Color);
        //     properties.stroke.get_mut().color.set(new_value);
        // }
        //
        // if let Some(fill) = rtc.compute_vtable_value(properties.fill._get_vtable_id()) {
        //     let new_value = unsafe_unwrap!(fill, TypesCoproduct, pax_std::types::Color);
        //     properties.fill.set(new_value);
        // }
        //
        // if let Some(segments) = rtc.compute_vtable_value(properties.segments._get_vtable_id()) {
        //     let new_value = unsafe_unwrap!(segments, TypesCoproduct, Vec<PathSegment>);
        //     properties.segments.set(new_value);
        // }
        //
        todo!()
    }
    fn handle_render(&mut self, rtc: &mut RenderTreeContext<R>, rc: &mut R) {
        let transform = rtc.transform_scroller_reset;

        let properties = (*self.properties).borrow();

        let mut bez_path = BezPath::new();

        for segment in properties.segments.get().iter() {
            match segment {
                PathSegment::Empty => { /* no-op */ }
                PathSegment::LineSegment(data) => {
                    bez_path.move_to(data.start);
                    bez_path.line_to(data.end);
                }
                PathSegment::CurveSegment(data) => {
                    bez_path.move_to(data.start);
                    bez_path.quad_to(data.handle, data.end);
                }
            }
        }

        let transformed_bez_path = transform * bez_path;
        let duplicate_transformed_bez_path = transformed_bez_path.clone();

        let color = properties.fill.get().to_piet_color();
        rc.fill(transformed_bez_path, &color);
        rc.stroke(
            duplicate_transformed_bez_path,
            &properties.stroke.get().color.get().to_piet_color(),
            *&properties.stroke.get().width.get().into(),
        );
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}
