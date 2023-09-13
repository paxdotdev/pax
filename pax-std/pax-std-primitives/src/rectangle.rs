use kurbo::{RoundedRect, Shape};
use piet::{LinearGradient, RadialGradient, RenderContext};

use pax_std::primitives::{Rectangle};
use pax_std::types::{Fill, RectangleCornerRadii};
use pax_core::{RenderNode, RenderNodePtrList, RenderTreeContext, HandlerRegistry, InstantiationArgs, RenderNodePtr, unsafe_unwrap};
use pax_core::pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use pax_runtime_api::{PropertyInstance, Size, Transform2D, Size2D};


use std::cell::RefCell;
use std::rc::Rc;


/// A basic 2D vector rectangle, drawn to fill the bounds specified
/// by `size`, transformed by `transform`
pub struct RectangleInstance<R: 'static + RenderContext> {
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry<R>>>>,
    pub instance_id: u32,
    pub properties: Rc<RefCell<Rectangle>>,
    pub size: Rc<RefCell<[Box<dyn PropertyInstance<Size>>; 2]>>,
    pub transform: Rc<RefCell<dyn PropertyInstance<Transform2D>>>,
}

impl<R: 'static + RenderContext>  RenderNode<R> for RectangleInstance<R> {

    fn get_instance_id(&self) -> u32 {
        self.instance_id
    }
    
    fn get_rendering_children(&self) -> RenderNodePtrList<R> {
        Rc::new(RefCell::new(vec![]))
    }

    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>> where Self: Sized {
        let properties = unsafe_unwrap!(args.properties, PropertiesCoproduct, Rectangle);
        let mut instance_registry = (*args.instance_registry).borrow_mut();
        let instance_id = instance_registry.mint_id();
        let ret = Rc::new(RefCell::new(RectangleInstance {
            instance_id,
            transform: args.transform,
            properties: Rc::new(RefCell::new(properties)),
            size: args.size.expect("Rectangle requires a size"),
            handler_registry: args.handler_registry,

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
    fn get_size(&self) -> Option<Size2D> { Some(Rc::clone(&self.size)) }
    fn get_transform(&mut self) -> Rc<RefCell<dyn PropertyInstance<Transform2D>>> { Rc::clone(&self.transform) }
    fn compute_properties(&mut self, rtc: &mut RenderTreeContext<R>) {
        let properties = &mut *self.properties.as_ref().borrow_mut();

        if let Some(stroke_width) = rtc.compute_vtable_value(properties.stroke.get().width._get_vtable_id()) {
            let new_value = if let TypesCoproduct::SizePixels(v) = stroke_width { v } else { unreachable!() };
            properties.stroke.get_mut().width.set(new_value);
        }

        if let Some(stroke_color) = rtc.compute_vtable_value(properties.stroke.get().color._get_vtable_id()) {
            let new_value = unsafe_unwrap!(stroke_color, TypesCoproduct, pax_std::types::Color);
            properties.stroke.get_mut().color.set(new_value);
        }

        if let Some(fill) = rtc.compute_vtable_value(properties.fill._get_vtable_id()) {
            let new_value = unsafe_unwrap!(fill, TypesCoproduct, Fill);
            properties.fill.set(new_value);
        }

        let size = &mut *self.size.as_ref().borrow_mut();

        if let Some(new_size) = rtc.compute_vtable_value(size[0]._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Size(v) = new_size { v } else { unreachable!() };
            size[0].set(new_value);
        }

        if let Some(top_right) = rtc.compute_vtable_value(properties.corner_radii.get().top_right._get_vtable_id()) {
            let new_value = unsafe_unwrap!(top_right, TypesCoproduct, f64);
            properties.corner_radii.get_mut().top_right.set(new_value);
        }

        if let Some(top_left) = rtc.compute_vtable_value(properties.corner_radii.get().top_left._get_vtable_id()) {
            let new_value = unsafe_unwrap!(top_left, TypesCoproduct, f64);
            properties.corner_radii.get_mut().top_left.set(new_value);
        }

        if let Some(bottom_right) = rtc.compute_vtable_value(properties.corner_radii.get().bottom_right._get_vtable_id()) {
            let new_value = unsafe_unwrap!(bottom_right, TypesCoproduct, f64);
            properties.corner_radii.get_mut().bottom_right.set(new_value);
        }

        if let Some(bottom_left) = rtc.compute_vtable_value(properties.corner_radii.get().bottom_left._get_vtable_id()) {
            let new_value = unsafe_unwrap!(bottom_left, TypesCoproduct, f64);
            properties.corner_radii.get_mut().bottom_left.set(new_value);
        }

        if let Some(corner_radii) = rtc.compute_vtable_value(properties.corner_radii._get_vtable_id()) {
            let new_value = unsafe_unwrap!(corner_radii, TypesCoproduct, RectangleCornerRadii);
            properties.corner_radii.set(new_value);
        }

        if let Some(new_size) = rtc.compute_vtable_value(size[1]._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Size(v) = new_size { v } else { unreachable!() };
            size[1].set(new_value);
        }

        let transform = &mut *self.transform.as_ref().borrow_mut();
        if let Some(new_transform) = rtc.compute_vtable_value(transform._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Transform2D(v) = new_transform { v } else { unreachable!() };
            transform.set(new_value);
        }
    }

    fn handle_render(&mut self, rtc: &mut RenderTreeContext<R>, rc: &mut R) {
        let transform = rtc.transform_scroller_reset;
        let bounding_dimens = rtc.bounds;
        let width: f64 =  bounding_dimens.0;
        let height: f64 =  bounding_dimens.1;

        let properties = (*self.properties).borrow();



        let rect = RoundedRect::new(0.0, 0.0, width, height, properties.corner_radii.get());

        let bez_path = rect.to_path(0.1);

        let transformed_bez_path = transform * bez_path;
        let duplicate_transformed_bez_path = transformed_bez_path.clone();

        match properties.fill.get() {
            Fill::Solid(color) => {
                rc.fill(transformed_bez_path, &color.to_piet_color());
            }
            Fill::LinearGradient(linear) => {
                let linear_gradient = LinearGradient::new(Fill::to_unit_point(linear.start,(width, height)),
                                    Fill::to_unit_point(linear.end, (width, height)),
                                        Fill::to_piet_gradient_stops(linear.stops.clone()));
                rc.fill(transformed_bez_path, &linear_gradient)
            }
            Fill::RadialGradient(radial) => {
                let origin = Fill::to_unit_point(radial.start, (width, height));
                let center = Fill::to_unit_point(radial.end, (width, height));
                let gradient_stops = Fill::to_piet_gradient_stops(radial.stops.clone());
                let radial_gradient = RadialGradient::new(radial.radius, gradient_stops).with_center(center).with_origin(origin);
                rc.fill(transformed_bez_path, &radial_gradient);
            }
        }

        //hack to address "phantom stroke" bug on Web
        let width : f64 = *&properties.stroke.get().width.get().into();
        if width > f64::EPSILON {
            rc.stroke(duplicate_transformed_bez_path, &properties.stroke.get().color.get().to_piet_color(), width);
        }

    }
}
