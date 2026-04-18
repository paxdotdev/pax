use pax_message::AnyCreatePatch;
use pax_runtime::api::{Layer, Property};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use_RefCell!();
use pax_engine::pax;
use pax_runtime::api::*;

use std::rc::Rc;

use crate::common::{native_surface_opacity, patch_if_needed};
use crate::ImageFit;

/// A platform-native image.  This will be managed by Pax's compositor as a platform-specific
/// host for an image, for example a DOM `<img>` element on the web, or a UIImageView on iOS.
/// `NativeImage` is an alternative to using `pax_std::media::Image`, with various trade-offs,
/// where the latter is managed as a GPU texture and drawn on a canvas surface.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::media::native_image::NativeImageInstance")]
pub struct NativeImage {
    /// Image URL/path understood by the chassis.
    pub url: Property<String>,
    /// How the image should fit into this node's bounds.
    pub fit: Property<ImageFit>,
}

// Runtime instance backing `<NativeImage>`.
pub struct NativeImageInstance {
    base: BaseInstance,
}

impl InstanceNode for NativeImageInstance {
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
                    layer: Layer::Native,
                    is_component: false,
                    is_slot: false,
                },
            ),
        })
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        let id = expanded_node.id.to_u32();
        context.enqueue_native_message(pax_message::NativeMessage::NativeImageCreate(
            AnyCreatePatch {
                id,
                parent_frame: expanded_node.parent_frame.get().map(|v| v.to_u32()),
                occlusion_layer_id: 0,
            },
        ));
        let weak_self_ref = Rc::downgrade(&expanded_node);
        let context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(NativeImagePatch {
            id,
            ..Default::default()
        }));

        let deps: Vec<_> = borrow!(expanded_node.properties_scope)
            .values()
            .cloned()
            .map(|v| v.get_untyped_property().clone())
            .chain([
                expanded_node.transform_and_bounds.untyped(),
                expanded_node.computed_opacity.untyped(),
                expanded_node.occlusion.untyped(),
            ])
            .collect();
        expanded_node
            .changed_listener
            .replace_with(Property::computed(
                move || {
                    let Some(expanded_node) = weak_self_ref.upgrade() else {
                        return;
                    };
                    let mut old_state = borrow_mut!(last_patch);

                    let mut patch = NativeImagePatch {
                        id,
                        ..Default::default()
                    };
                    expanded_node.with_properties_unwrapped(|properties: &mut NativeImage| {
                        let computed_tab = expanded_node.transform_and_bounds.get();
                        let (width, height) = computed_tab.bounds;
                        let updates = [
                            patch_if_needed(
                                &mut old_state.url,
                                &mut patch.url,
                                properties.url.get(),
                            ),
                            patch_if_needed(
                                &mut old_state.fit,
                                &mut patch.fit,
                                match properties.fit.get() {
                                    ImageFit::Fill => "cover",
                                    ImageFit::Fit => "contain",
                                    ImageFit::Stretch => "fill",
                                }
                                .to_string(),
                            ),
                            patch_if_needed(&mut old_state.size_x, &mut patch.size_x, width),
                            patch_if_needed(&mut old_state.size_y, &mut patch.size_y, height),
                            patch_if_needed(
                                &mut old_state.parent_frame,
                                &mut patch.parent_frame,
                                expanded_node.parent_frame.get().map(|v| v.to_u32()),
                            ),
                            patch_if_needed(
                                &mut old_state.z_index,
                                &mut patch.z_index,
                                expanded_node.occlusion.get().z_index,
                            ),
                            patch_if_needed(
                                &mut old_state.transform,
                                &mut patch.transform,
                                computed_tab.transform.coeffs().to_vec(),
                            ),
                            patch_if_needed(
                                &mut old_state.opacity,
                                &mut patch.opacity,
                                native_surface_opacity(&expanded_node, &context),
                            ),
                        ];
                        if updates.into_iter().any(|v| v == true) {
                            context.enqueue_native_message(
                                pax_message::NativeMessage::NativeImageUpdate(patch),
                            );
                        }
                    });
                    ()
                },
                &deps,
            ));
    }

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        let id = expanded_node.id.clone();
        expanded_node
            .changed_listener
            .replace_with(Property::default());
        context.enqueue_native_message(pax_message::NativeMessage::NativeImageDelete(id.to_u32()));
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("NativeImage").finish_non_exhaustive()
    }
}
