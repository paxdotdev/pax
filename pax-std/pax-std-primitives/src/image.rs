use pax_runtime::{api::Property, api::RenderContext};
use pax_std::primitives::Image;
use std::{cell::RefCell, collections::HashMap};

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
    native_message_props: RefCell<HashMap<u32, Property<()>>>,
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

    fn update(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        _context: &Rc<RefCell<RuntimeContext>>,
    ) {
        //trigger computation of property that computes + sends native message update
        self.native_message_props
            .borrow()
            .get(&expanded_node.id_chain[0])
            .unwrap()
            .get();
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RefCell<RuntimeContext>>,
    ) {
        let id_chain = expanded_node.id_chain.clone();

        // send update message when relevant properties change
        let weak_self_ref = Rc::downgrade(&expanded_node);
        let context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(ImagePatch {
            id_chain: id_chain.clone(),
            ..Default::default()
        }));

        let deps: Vec<_> = expanded_node
            .properties_scope
            .borrow()
            .values()
            .cloned()
            .chain([
                expanded_node.layout_properties.transform.untyped(),
                expanded_node.layout_properties.bounds.untyped(),
            ])
            .collect();
        self.native_message_props.borrow_mut().insert(
            id_chain[0],
            Property::computed(
                move || {
                    let Some(expanded_node) = weak_self_ref.upgrade() else {
                        unreachable!()
                    };
                    let id_chain = expanded_node.id_chain.clone();
                    let mut old_state = last_patch.borrow_mut();

                    let mut patch = ImagePatch {
                        id_chain: id_chain.clone(),
                        ..Default::default()
                    };
                    let path_val = expanded_node
                        .with_properties_unwrapped(|props: &mut Image| props.path.get().clone());

                    let update =
                        patch_if_needed(&mut old_state.path, &mut patch.path, path_val.string);

                    if update {
                        context
                            .borrow_mut()
                            .enqueue_native_message(pax_message::NativeMessage::ImageLoad(patch));
                    }
                    ()
                },
                &deps,
            ),
        );
    }

    fn handle_unmount(
        &self,
        expanded_node: &Rc<ExpandedNode>,
        _context: &Rc<RefCell<RuntimeContext>>,
    ) {
        let id_chain = expanded_node.id_chain.clone();
        let id = id_chain[0];
        // Reset so that native_message stops sending updates while unmounted
        self.native_message_props.borrow_mut().remove(&id);
    }

    fn render(
        &self,
        expanded_node: &ExpandedNode,
        _rtc: &Rc<RefCell<RuntimeContext>>,
        rc: &mut dyn RenderContext,
    ) {
        let transform = expanded_node.layout_properties.transform.get();
        let bounding_dimens = expanded_node.layout_properties.bounds.get();
        let width = bounding_dimens.0;
        let height = bounding_dimens.1;

        let transformed_bounds = kurbo::Rect::new(0.0, 0.0, width, height);

        let path =
            expanded_node.with_properties_unwrapped(|props: &mut Image| props.path.get().clone());
        let layer_id = format!("{}", expanded_node.occlusion_id.borrow());
        rc.save(&layer_id);
        rc.transform(&layer_id, transform.into());
        rc.draw_image(&layer_id, &path.string, transformed_bounds);
        rc.restore(&layer_id);
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&pax_runtime::ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Image").finish_non_exhaustive()
    }
}
