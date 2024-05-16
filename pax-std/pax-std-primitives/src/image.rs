use pax_runtime::{api::Property, api::RenderContext, ExpandedNodeIdentifier};
use pax_runtime_api::{borrow, borrow_mut, use_RefCell};
use pax_std::{primitives::Image, types::ImageFit};
use std::collections::HashMap;

use_RefCell!();
use pax_message::ImagePatch;
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use std::rc::Rc;

use crate::patch_if_needed;
/// An Image (decoded by chassis), drawn to the bounds specified
/// by `size`, transformed by `transform`
pub struct ImageInstance {
    base: BaseInstance,
    // Properties that listen to Image property changes, and computes
    // a patch in the case that they have changed + sends it as a native
    // message to the chassi. Since InstanceNode -> ExpandedNode has a one
    // to many relationship, needs to be a hashmap
    native_message_props: RefCell<HashMap<ExpandedNodeIdentifier, Property<()>>>,
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
                    layer: pax_runtime::api::Layer::Canvas,
                    is_component: false,
                },
            ),
            native_message_props: Default::default(),
        })
    }

    fn update(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, _context: &Rc<RuntimeContext>) {
        //trigger computation of property that computes + sends native message update
        borrow!(self.native_message_props)
            .get(&expanded_node.id)
            .unwrap()
            .get();
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        let id = expanded_node.id.to_u32();

        // send update message when relevant properties change
        let weak_self_ref = Rc::downgrade(&expanded_node);
        let context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(ImagePatch {
            id,
            ..Default::default()
        }));

        let deps: Vec<_> = borrow!(expanded_node.properties_scope)
            .values()
            .cloned()
            .chain([
                expanded_node.layout_properties.transform.untyped(),
                expanded_node.layout_properties.bounds.untyped(),
            ])
            .collect();
        borrow_mut!(self.native_message_props).insert(
            expanded_node.id,
            Property::computed(
                move || {
                    let Some(expanded_node) = weak_self_ref.upgrade() else {
                        unreachable!()
                    };
                    let mut old_state = borrow_mut!(last_patch);

                    let mut patch = ImagePatch {
                        id,
                        ..Default::default()
                    };
                    let path_val = expanded_node
                        .with_properties_unwrapped(|props: &mut Image| props.path.get().clone());

                    let update = patch_if_needed(&mut old_state.path, &mut patch.path, path_val);

                    if update {
                        context
                            .enqueue_native_message(pax_message::NativeMessage::ImageLoad(patch));
                    }
                    ()
                },
                &deps,
            ),
        );
    }

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, _context: &Rc<RuntimeContext>) {
        let id = expanded_node.id.clone();
        // Reset so that native_message stops sending updates while unmounted
        borrow_mut!(self.native_message_props).remove(&id);
    }

    fn render(
        &self,
        expanded_node: &ExpandedNode,
        _rtc: &Rc<RuntimeContext>,
        rc: &mut dyn RenderContext,
    ) {
        let transform = expanded_node.layout_properties.transform.get();
        let bounding_dimens = expanded_node.layout_properties.bounds.get();
        let container_width = bounding_dimens.0;
        let container_height = bounding_dimens.1;

        expanded_node.with_properties_unwrapped(|props: &mut Image| {
            let path = props.path.get();
            let Some((image_width, image_height)) = rc.get_image_size(&path) else {
                // image not loaded yet
                return;
            };
            let (image_width, image_height) = (image_width as f64, image_height as f64);
            let stretch_w = container_width / image_width;
            let stretch_h = container_height / image_height;
            let (width, height) = match props.fit.get() {
                ImageFit::FillVertical => (image_width * stretch_h, image_height * stretch_h),
                ImageFit::FillHorizontal => (image_width * stretch_w, image_height * stretch_w),
                ImageFit::Fill => {
                    let stretch = stretch_h.max(stretch_w);
                    (image_width * stretch, image_height * stretch)
                }
                ImageFit::Fit => {
                    let stretch = stretch_h.min(stretch_w);
                    (image_width * stretch, image_height * stretch)
                }
                ImageFit::Stretch => (container_width, container_height),
            };
            let x = (container_width - width) / 2.0;
            let y = (container_height - height) / 2.0;
            let transformed_bounds = kurbo::Rect::new(x, y, x + width, y + height);
            let layer_id = format!("{}", borrow!(expanded_node.occlusion_id));
            rc.save(&layer_id);
            rc.transform(&layer_id, transform.into());
            rc.draw_image(&layer_id, &path, transformed_bounds);
            rc.restore(&layer_id);
        });
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&pax_runtime::ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Image").finish_non_exhaustive()
    }
}
