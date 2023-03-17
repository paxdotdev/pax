use kurbo::{BezPath};
use piet::{RenderContext};

use pax_std::primitives::{Rectangle};
use pax_std::types::ColorVariant;
use pax_core::{Color, TabCache, RenderNode, RenderNodePtrList, RenderTreeContext, ExpressionContext, InstanceRegistry, HandlerRegistry, InstantiationArgs, RenderNodePtr, unsafe_unwrap};
use pax_core::pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use pax_runtime_api::{PropertyInstance, PropertyLiteral, Size, Transform2D, Size2D, ArgsCoproduct, Property};

use std::str::FromStr;
use std::cell::RefCell;
use std::rc::Rc;


/// A basic 2D vector rectangle, drawn to fill the bounds specified
/// by `size`, transformed by `transform`
pub struct RectangleInstance<R: 'static + RenderContext> {
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry<R>>>>,
    pub instance_id: u64,
    pub properties: Rc<RefCell<Rectangle>>,
    pub size: Rc<RefCell<[Box<dyn PropertyInstance<Size>>; 2]>>,
    pub transform: Rc<RefCell<dyn PropertyInstance<Transform2D>>>,
}

impl<R: 'static + RenderContext>  RenderNode<R> for RectangleInstance<R> {

    fn get_instance_id(&self) -> u64 {
        self.instance_id
    }
    
    fn get_rendering_children(&self) -> RenderNodePtrList<R> {
        Rc::new(RefCell::new(vec![]))
    }

    fn instantiate(mut args: InstantiationArgs<R>) -> Rc<RefCell<Self>> where Self: Sized {
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
        let mut properties = &mut *self.properties.as_ref().borrow_mut();

        if let Some(stroke_width) = rtc.compute_vtable_value(properties.stroke.get().width._get_vtable_id()) {
            let new_value = if let TypesCoproduct::SizePixels(v) = stroke_width { v } else { unreachable!() };
            properties.stroke.get_mut().width.set(new_value);
        }

        if let Some(stroke_color) = rtc.compute_vtable_value(properties.stroke.get().color._get_vtable_id()) {
            let new_value = if let TypesCoproduct::__pax_stdCOCOtypesCOCOColor(v) = stroke_color { v } else { unreachable!() };
            properties.stroke.get_mut().color.set(new_value);
        }

        if let Some(fill) = rtc.compute_vtable_value(properties.fill._get_vtable_id()) {
            let new_value = if let TypesCoproduct::__pax_stdCOCOtypesCOCOColor(v) = fill { v } else { unreachable!() };
            properties.fill.set(new_value);
        }

        let mut size = &mut *self.size.as_ref().borrow_mut();

        if let Some(new_size) = rtc.compute_vtable_value(size[0]._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Size(v) = new_size { v } else { unreachable!() };
            size[0].set(new_value);
        }

        if let Some(new_size) = rtc.compute_vtable_value(size[1]._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Size(v) = new_size { v } else { unreachable!() };
            size[1].set(new_value);
        }

        let mut transform = &mut *self.transform.as_ref().borrow_mut();
        if let Some(new_transform) = rtc.compute_vtable_value(transform._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Transform2D(v) = new_transform { v } else { unreachable!() };
            transform.set(new_value);
        }

    }
    fn handle_render(&self, rtc: &mut RenderTreeContext<R>, rc: &mut R) {
        let transform = rtc.transform;
        let bounding_dimens = rtc.bounds;
        let width: f64 =  bounding_dimens.0;
        let height: f64 =  bounding_dimens.1;

        let properties = (*self.properties).borrow();

        let properties_color = properties.fill.get();
        let color = match properties_color.color_variant {
            ColorVariant::Hlca(slice) => {
                Color::hlca(slice[0], slice[1], slice[2], slice[3])
            },
            ColorVariant::Hlc(slice) => {
                Color::hlc(slice[0], slice[1], slice[2])
            },
            ColorVariant::Rgba(slice) => {
                Color::rgba(slice[0], slice[1], slice[2], slice[3])
            },
            ColorVariant::Rgb(slice) => {
                Color::rgb(slice[0], slice[1], slice[2])
            }
        };

        let mut bez_path = BezPath::new();
        bez_path.move_to((0.0, 0.0));
        bez_path.line_to((width , 0.0));
        bez_path.line_to((width , height ));
        bez_path.line_to((0.0, height));
        bez_path.line_to((0.0,0.0));
        bez_path.close_path();

        let transformed_bez_path = transform * bez_path;
        let duplicate_transformed_bez_path = transformed_bez_path.clone();

        let color = properties.fill.get().to_piet_color();
        rc.fill(transformed_bez_path, &color);
        rc.stroke(duplicate_transformed_bez_path, &properties.stroke.get().color.get().to_piet_color(), *&properties.stroke.get().width.get().into());

    }
}
