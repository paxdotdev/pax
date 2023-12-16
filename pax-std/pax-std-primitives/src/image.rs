use pax_std::primitives::Image;
use piet::{ImageFormat, InterpolationMode, RenderContext};
use std::collections::HashMap;

use pax_core::{
    HandlerRegistry, InstanceNode, InstanceNodePtr, InstanceNodePtrList, InstantiationArgs,
    PropertiesComputable, RenderTreeContext,
};
use pax_message::ImagePatch;
use pax_runtime_api::CommonProperties;
use std::cell::RefCell;
use std::rc::Rc;
/// An Image (decoded by chassis), drawn to the bounds specified
/// by `size`, transformed by `transform`
pub struct ImageInstance<R: 'static + RenderContext> {
    base: BaseInstance,
    last_patches: HashMap<Vec<u32>, pax_message::ImagePatch>,
    pub image: Option<<R as RenderContext>::Image>,
}

impl<R: 'static + RenderContext> InstanceNode<R> for ImageInstance<R> {
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
            image: None,
        }))
    }

    fn expand_node_and_compute_properties(&mut self, rtc: &mut PropertiesTreeContext<R>) {
        // let properties = &mut *self.properties.as_ref().borrow_mut();

        // if let Some(path) = rtc.compute_vtable_value(properties.path._get_vtable_id()) {
        //     let new_value = if let TypesCoproduct::String(v) = path {
        //         v
        //     } else {
        //         unreachable!()
        //     };
        //     properties
        //         .path
        //         .set(pax_runtime_api::StringBox { string: new_value });
        // }
        //
        // self.common_properties.compute_properties(rtc);
        todo!()
    }

    fn handle_native_patches(
        &mut self,
        rtc: &mut RenderTreeContext<R>,
        _computed_size: (f64, f64),
        _transform_coeffs: Vec<f64>,
        _z_index: u32,
        _subtree_depth: u32,
    ) {
        let mut new_message: ImagePatch = Default::default();
        new_message.id_chain = rtc.get_id_chain(self.instance_id);
        if !self.last_patches.contains_key(&new_message.id_chain) {
            let mut patch = ImagePatch::default();
            patch.id_chain = new_message.id_chain.clone();
            self.last_patches
                .insert(new_message.id_chain.clone(), patch);
        }
        let last_patch = self.last_patches.get_mut(&new_message.id_chain).unwrap();
        let mut has_any_updates = false;

        let properties = &mut *self.properties.as_ref().borrow_mut();
        let val = properties.path.get();
        let is_new_value = match &last_patch.path {
            Some(cached_value) => !val.string.eq(cached_value),
            None => true,
        };
        if is_new_value {
            new_message.path = Some(val.string.clone());
            last_patch.path = Some(val.string.clone());
            has_any_updates = true;
        }

        if has_any_updates {
            (*rtc.engine.runtime)
                .borrow_mut()
                .enqueue_native_message(pax_message::NativeMessage::ImageLoad(new_message));
        }
    }

    fn handle_render(&mut self, rtc: &mut RenderTreeContext<R>, rc: &mut R) {
        let transform = rtc.transform_scroller_reset;
        let bounding_dimens = rtc.bounds;
        let width = bounding_dimens.0;
        let height = bounding_dimens.1;

        let bounds = kurbo::Rect::new(0.0, 0.0, width, height);
        let top_left = transform * kurbo::Point::new(bounds.min_x(), bounds.min_y());
        let bottom_right = transform * kurbo::Point::new(bounds.max_x(), bounds.max_y());
        let transformed_bounds =
            kurbo::Rect::new(top_left.x, top_left.y, bottom_right.x, bottom_right.y);

        let _properties = (*self.properties).borrow();
        let id_chain = rtc.get_id_chain(self.instance_id);
        if rtc.engine.image_map.contains_key(&id_chain) && self.image.is_none() {
            let (bytes, width, height) = rtc.engine.image_map.get(&id_chain).unwrap();
            let image = rc
                .make_image(*width, *height, &*bytes, ImageFormat::RgbaSeparate)
                .unwrap();
            self.image = Some(image);
        }
        if let Some(image) = &self.image {
            rc.draw_image(&image, transformed_bounds, InterpolationMode::Bilinear);
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}
