use kurbo::{BezPath, Rect, Ellipse as KurboEllipse, Shape};
use piet::{ImageFormat, InterpolationMode, RenderContext, Image as PietImage, Error, TextStorage};
use std::collections::HashMap;
use pax_std::primitives::{Image};
use pax_std::types::ColorVariant;
use pax_core::{Color, RenderNode, RenderNodePtrList, RenderTreeContext, ExpressionContext, InstanceRegistry, HandlerRegistry, InstantiationArgs, RenderNodePtr, safe_unwrap, generate_property_access};
use pax_core::pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use pax_runtime_api::{PropertyInstance, PropertyLiteral, Size, Transform2D, Size2D, log};

use std::str::FromStr;
use std::cell::RefCell;
use std::rc::Rc;
use pax_message::ImagePatch;
/// An Image (decoded by chassis), drawn to the bounds specified
/// by `size`, transformed by `transform`
pub struct ImageInstance<R: 'static + RenderContext> {
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry<R>>>>,
    pub instance_id: u64,
    pub size: Rc<RefCell<[Box<dyn PropertyInstance<Size>>; 2]>>,
    pub transform: Rc<RefCell<dyn PropertyInstance<Transform2D>>>,
    last_patches: HashMap<Vec<u64>, pax_message::ImagePatch>,
    pub image: Option<<R as RenderContext>::Image>,

    properties_raw: PropertiesCoproduct,
}
generate_property_access!(ImageInstance, Image);

impl<R: 'static + RenderContext>  RenderNode<R> for ImageInstance<R> {

    fn get_instance_id(&self) -> u64 {
        self.instance_id
    }

    fn get_rendering_children(&self) -> RenderNodePtrList<R> {
        Rc::new(RefCell::new(vec![]))
    }

    fn instantiate(mut args: InstantiationArgs<R>) -> Rc<RefCell<Self>> where Self: Sized {

        let mut instance_registry = (*args.instance_registry).borrow_mut();
        let instance_id = instance_registry.mint_id();
        let ret = Rc::new(RefCell::new(ImageInstance {
            instance_id,
            transform: args.transform,
            properties_raw: args.properties,
            size: args.size.expect("Image requires a size"),
            handler_registry: args.handler_registry,
            last_patches: Default::default(),
            image: None,
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
        let mut properties = self.get_properties_mut();

        if let Some(path) = rtc.compute_vtable_value(properties.path._get_vtable_id()) {
            let new_value = if let TypesCoproduct::String(v) = path { v } else { unreachable!() };
            properties.path.set(new_value);
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


    fn compute_native_patches(&mut self, rtc: &mut RenderTreeContext<R>, computed_size: (f64, f64), transform_coeffs: Vec<f64>, depth : usize) {
        let mut new_message: ImagePatch = Default::default();
        new_message.id_chain = rtc.get_id_chain(self.instance_id);
        if !self.last_patches.contains_key(&new_message.id_chain) {
            let mut patch = ImagePatch::default();
            patch.id_chain = new_message.id_chain.clone();
            self.last_patches.insert(new_message.id_chain.clone(), patch);
        }
        let last_patch = self.last_patches.get_mut(&new_message.id_chain).unwrap();
        let mut has_any_updates = false;

        let mut properties = self.get_properties_mut();
        let val = properties.path.get();
        let is_new_value = match &last_patch.path {
            Some(cached_value) => !val.eq(cached_value),
            None => true,
        };
        if is_new_value {
            new_message.path = Some(val.clone());
            last_patch.path = Some(val.clone());
            has_any_updates = true;
        }

        if has_any_updates {
            (*rtc.engine.runtime).borrow_mut().enqueue_native_message(pax_message::NativeMessage::ImageLoad(new_message));
        }
    }

    fn handle_render(&self, rtc: &mut RenderTreeContext<R>, rc: &mut R) {
        let transform = rtc.transform;
        let bounding_dimens = rtc.bounds;
        let width =  bounding_dimens.0;
        let height =  bounding_dimens.1;

        let properties = self.get_properties();
        let id_chain = rtc.get_id_chain(self.instance_id);
        if rtc.engine.image_map.contains_key(&id_chain) {
            let bounds = kurbo::Rect::new(0.0,0.0, width, height);
            let top_left = transform * kurbo::Point::new(bounds.min_x(), bounds.min_y());
            let bottom_right = transform * kurbo::Point::new(bounds.max_x(), bounds.max_y());
            let transformed_bounds = kurbo::Rect::new(top_left.x, top_left.y, bottom_right.x, bottom_right.y);
            let (bytes, width, height) = rtc.engine.image_map.get(&id_chain).unwrap();
            let image = rc.make_image(*width, *height, &*bytes, ImageFormat::RgbaSeparate).unwrap();
            rc.draw_image(&image, transformed_bounds, InterpolationMode::Bilinear);
        }
    }
}