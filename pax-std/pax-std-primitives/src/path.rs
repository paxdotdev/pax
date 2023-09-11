use kurbo::{BezPath, Rect, Ellipse as KurboEllipse, Shape};
use piet::{RenderContext};

use pax_std::primitives::{Path};
use pax_std::types::{ColorVariant, CurveSegmentData, LineSegmentData, PathSegment};
use pax_core::{Color, RenderNode, RenderNodePtrList, RenderTreeContext, ExpressionContext, InstanceRegistry, HandlerRegistry, InstantiationArgs, RenderNodePtr, unsafe_unwrap};
use pax_core::pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use pax_runtime_api::{PropertyInstance, PropertyLiteral, Size, Transform2D, Size2D};

use std::str::FromStr;
use std::cell::RefCell;
use std::rc::Rc;
use pax_std::types::PathSegment::LineSegment;


/// A basic 2D vector path for arbitrary Bézier / line-segment chains
pub struct PathInstance<R: 'static + RenderContext> {
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry<R>>>>,
    pub instance_id: u64,
    pub properties: Rc<RefCell<Path>>,
    pub transform: Rc<RefCell<dyn PropertyInstance<Transform2D>>>,
}

impl<R: 'static + RenderContext>  RenderNode<R> for PathInstance<R> {

    fn get_instance_id(&self) -> u64 {
        self.instance_id
    }

    fn get_rendering_children(&self) -> RenderNodePtrList<R> {
        Rc::new(RefCell::new(vec![]))
    }

    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>> where Self: Sized {
        let properties = unsafe_unwrap!(args.properties, PropertiesCoproduct, Path);

        let mut instance_registry = (*args.instance_registry).borrow_mut();
        let instance_id = instance_registry.mint_id();
        let ret = Rc::new(RefCell::new(PathInstance {
            instance_id,
            properties: Rc::new(RefCell::new(properties)),
            handler_registry: args.handler_registry,
            transform: args.transform,
        }));

        instance_registry.register(instance_id, Rc::clone(&ret) as RenderNodePtr<R>);
        ret
    }

    fn get_handler_registry(&self) -> Option<Rc<RefCell<HandlerRegistry<R>>>> {
        match &self.handler_registry {
            Some(registry) => {
                Some(Rc::clone(registry))
            },
            _ => {None}
        }

    }

    fn get_size(&self) -> Option<Size2D> { None }
    fn get_transform(&mut self) -> Rc<RefCell<dyn PropertyInstance<Transform2D>>> { Rc::clone(&self.transform) }

    fn compute_properties(&mut self, rtc: &mut RenderTreeContext<R>) {
        let properties = &mut *self.properties.as_ref().borrow_mut();

        if let Some(stroke_width) = rtc.compute_vtable_value(properties.stroke.get().width._get_vtable_id()) {
            let new_value = unsafe_unwrap!(stroke_width, TypesCoproduct, pax_runtime_api::SizePixels);
            properties.stroke.get_mut().width.set(new_value);
        }

        if let Some(stroke_color) = rtc.compute_vtable_value(properties.stroke.get().color._get_vtable_id()) {
            let new_value = unsafe_unwrap!(stroke_color, TypesCoproduct, pax_std::types::Color);
            properties.stroke.get_mut().color.set(new_value);
        }

        if let Some(fill) = rtc.compute_vtable_value(properties.fill._get_vtable_id()) {
            let new_value = unsafe_unwrap!(fill, TypesCoproduct, pax_std::types::Color);
            properties.fill.set(new_value);
        }

        if let Some(segments) = rtc.compute_vtable_value(properties.segments._get_vtable_id()) {
            let new_value = unsafe_unwrap!(segments, TypesCoproduct, Vec<PathSegment>);
            properties.segments.set(new_value);
        }

    }
    fn handle_render(&mut self, rtc: &mut RenderTreeContext<R>, rc: &mut R) {
        let transform = rtc.transform;

        let properties = (*self.properties).borrow();

        let mut bez_path = BezPath::new();

        for segment in properties.segments.get().iter() {
            match segment{
                PathSegment::Empty => {/* no-op */},
                PathSegment::LineSegment(data) => {
                    bez_path.move_to(data.start);
                    bez_path.line_to(data.end);
                } ,
                PathSegment::CurveSegment(data) => {
                    bez_path.move_to(data.start);
                    bez_path.quad_to( data.handle, data.end);
                },
            }
        }

        let transformed_bez_path = transform * bez_path;
        let duplicate_transformed_bez_path = transformed_bez_path.clone();

        let color = properties.fill.get().to_piet_color();
        rc.fill(transformed_bez_path, &color);
        rc.stroke(duplicate_transformed_bez_path, &properties.stroke.get().color.get().to_piet_color(), *&properties.stroke.get().width.get().into());

    }
}
