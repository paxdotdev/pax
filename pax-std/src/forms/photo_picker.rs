use pax_engine::pax;
use pax_message::{AnyCreatePatch, PhotoPickerPatch};
use pax_runtime::api::{borrow, borrow_mut, use_RefCell, Layer, Property};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use std::rc::Rc;
use_RefCell!();

use crate::common::{native_surface_opacity, patch_if_needed};

/// Source requested when opening a `PhotoPicker`.
#[pax]
#[derive(PartialEq)]
#[engine_import_path("pax_engine")]
pub enum PhotoPickerSource {
    /// Let the user choose existing images from the platform photo/file picker.
    #[default]
    Library,
    /// Request camera capture where the current platform supports it.
    Camera,
}

impl PhotoPickerSource {
    fn as_message(&self) -> String {
        match self {
            PhotoPickerSource::Library => "library",
            PhotoPickerSource::Camera => "camera",
        }
        .to_string()
    }
}

/// A transparent native hit target that opens the platform photo picker.
///
/// Apps should render their own visible button or thumbnail affordance and place
/// `PhotoPicker` over the interactive area. Increment `trigger` to request a
/// programmatic open; on the web, direct user activation of the picker element is
/// the most reliable way to satisfy browser file-picker requirements.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::forms::photo_picker::PhotoPickerInstance")]
#[custom(Default)]
pub struct PhotoPicker {
    /// Incrementing request value. Returned as `request_id` in `photo_picker_change`.
    pub trigger: Property<u64>,
    /// Requested platform source.
    pub source: Property<PhotoPickerSource>,
    /// Whether multiple images may be selected.
    pub allow_multiple: Property<bool>,
    /// Accepted MIME/file filter. Web uses this as the input `accept` value.
    pub accept: Property<String>,
    /// Whether the chassis should copy bytes into the event when practical.
    pub include_bytes: Property<bool>,
    /// Maximum copied/read bytes per selected photo. Larger photos report a size-limit status.
    pub max_bytes_per_photo: Property<u64>,
}

impl Default for PhotoPicker {
    fn default() -> Self {
        Self {
            trigger: Property::new(0),
            source: Property::new(PhotoPickerSource::Library),
            allow_multiple: Property::new(true),
            accept: Property::new("image/*".to_string()),
            include_bytes: Property::new(true),
            max_bytes_per_photo: Property::new(25 * 1024 * 1024),
        }
    }
}

// Runtime instance backing `<PhotoPicker>`.
pub struct PhotoPickerInstance {
    base: BaseInstance,
}

impl InstanceNode for PhotoPickerInstance {
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
        context.enqueue_native_message(pax_message::NativeMessage::PhotoPickerCreate(
            AnyCreatePatch {
                id,
                parent_frame: expanded_node.parent_frame.get().map(|v| v.to_u32()),
                occlusion_layer_id: 0,
            },
        ));

        let weak_self_ref = Rc::downgrade(expanded_node);
        let context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(PhotoPickerPatch {
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

                    let mut patch = PhotoPickerPatch {
                        id,
                        ..Default::default()
                    };
                    expanded_node.with_properties_unwrapped(|properties: &mut PhotoPicker| {
                        let computed_tab = expanded_node.transform_and_bounds.get();
                        let (width, height) = computed_tab.bounds;
                        let updates = [
                            patch_if_needed(
                                &mut old_state.trigger,
                                &mut patch.trigger,
                                properties.trigger.get(),
                            ),
                            patch_if_needed(
                                &mut old_state.source,
                                &mut patch.source,
                                properties.source.get().as_message(),
                            ),
                            patch_if_needed(
                                &mut old_state.allow_multiple,
                                &mut patch.allow_multiple,
                                properties.allow_multiple.get(),
                            ),
                            patch_if_needed(
                                &mut old_state.accept,
                                &mut patch.accept,
                                properties.accept.get(),
                            ),
                            patch_if_needed(
                                &mut old_state.include_bytes,
                                &mut patch.include_bytes,
                                properties.include_bytes.get(),
                            ),
                            patch_if_needed(
                                &mut old_state.max_bytes_per_photo,
                                &mut patch.max_bytes_per_photo,
                                properties.max_bytes_per_photo.get(),
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
                        if updates.into_iter().any(|v| v) {
                            context.enqueue_native_message(
                                pax_message::NativeMessage::PhotoPickerUpdate(patch),
                            );
                        }
                    });
                },
                &deps,
            ));
    }

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        let id = expanded_node.id.to_u32();
        expanded_node
            .changed_listener
            .replace_with(Property::default());
        context.enqueue_native_message(pax_message::NativeMessage::PhotoPickerDelete(id));
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("PhotoPicker").finish_non_exhaustive()
    }
}
