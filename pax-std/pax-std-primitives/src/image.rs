use pax_runtime_api::RenderContext;
use pax_std::primitives::Image;
use std::{cell::RefCell, collections::HashMap};

use pax_core::{
    declarative_macros::handle_vtable_update, BaseInstance, ExpandedNode, InstanceFlags,
    InstanceNode, InstantiationArgs, RuntimeContext,
};
use pax_message::ImagePatch;
use std::rc::Rc;
/// An Image (decoded by chassis), drawn to the bounds specified
/// by `size`, transformed by `transform`
pub struct ImageInstance {
    base: BaseInstance,
    last_patches: RefCell<HashMap<Vec<u32>, pax_message::ImagePatch>>,
}

impl InstanceNode for ImageInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self>
    where
        Self: Sized,
    {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: false,
                    layer: pax_runtime_api::Layer::Canvas,
                    is_component: false,
                },
            ),
            last_patches: Default::default(),
        })
    }

    fn update(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, context: &mut RuntimeContext) {
        //Doesn't need to expand any children
        expanded_node.with_properties_unwrapped(|properties: &mut Image| {
            handle_vtable_update(
                context.expression_table(),
                &expanded_node.stack,
                &mut properties.path,
            );
        });
    }

    fn handle_native_patches(&self, expanded_node: &ExpandedNode, rtc: &mut RuntimeContext) {
        let val =
            expanded_node.with_properties_unwrapped(|props: &mut Image| props.path.get().clone());
        let mut new_message: ImagePatch = Default::default();
        new_message.id_chain = expanded_node.id_chain.clone();
        let mut last_patches = self.last_patches.borrow_mut();
        if !last_patches.contains_key(&new_message.id_chain) {
            let mut patch = ImagePatch::default();
            patch.id_chain = new_message.id_chain.clone();
            last_patches.insert(new_message.id_chain.clone(), patch);
        }
        let last_patch = last_patches.get_mut(&new_message.id_chain).unwrap();
        let mut has_any_updates = false;

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
            rtc.enqueue_native_message(pax_message::NativeMessage::ImageLoad(new_message));
        }
    }

    fn render(
        &self,
        expanded_node: &ExpandedNode,
        rtc: &mut RuntimeContext,
        rc: &mut dyn RenderContext,
    ) {
        let comp_props = &expanded_node.layout_properties.borrow();
        let comp_props = comp_props.as_ref().unwrap();
        let transform = comp_props.computed_tab.transform;
        let bounding_dimens = comp_props.computed_tab.bounds;
        let width = bounding_dimens.0;
        let height = bounding_dimens.1;

        let bounds = kurbo::Rect::new(0.0, 0.0, width, height);
        let top_left = transform * kurbo::Point::new(bounds.min_x(), bounds.min_y());
        let bottom_right = transform * kurbo::Point::new(bounds.max_x(), bounds.max_y());
        let transformed_bounds =
            kurbo::Rect::new(top_left.x, top_left.y, bottom_right.x, bottom_right.y);

        let val =
            expanded_node.with_properties_unwrapped(|props: &mut Image| props.path.get().clone());
        let layer_id = format!("{}", expanded_node.occlusion_id.borrow());
        rc.draw_image(&layer_id, &val.string, transformed_bounds);
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn resolve_debug(
        &self,
        _f: &mut std::fmt::Formatter,
        _expanded_node: Option<&pax_core::ExpandedNode>,
    ) -> std::fmt::Result {
        todo!()
    }
}
