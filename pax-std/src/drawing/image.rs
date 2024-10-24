use api::Window;
use kurbo::Shape;
use pax_engine::*;
use pax_runtime::api::{borrow_mut, use_RefCell};
use pax_runtime::{api::Property, api::RenderContext, ExpandedNodeIdentifier};
use std::cell::Ref;
use std::collections::HashSet;

use_RefCell!();
use pax_message::ImagePatch;
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
    TransformAndBounds,
};
use std::rc::Rc;

use crate::common::patch_if_needed;

/// An Image (decoded by chassis), drawn to the bounds specified
/// by `size`, transformed by `transform`
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::drawing::image::ImageInstance")]
pub struct Image {
    pub source: Property<ImageSource>,
    pub fit: Property<ImageFit>,
}

#[pax]
#[derive(PartialEq)]
#[engine_import_path("pax_engine")]
pub enum ImageSource {
    #[default]
    Empty,
    Url(String),
    /// width, height, vec.len() = width * height * 4 (one u8 for each rgba value)
    Data(usize, usize, Vec<u8>),
}

pub struct ImageInstance {
    base: BaseInstance,
    needs_to_load_data: Rc<RefCell<HashSet<ExpandedNodeIdentifier>>>,
    initial_load: RefCell<HashSet<ExpandedNodeIdentifier>>,
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
                    is_slot: false,
                },
            ),
            needs_to_load_data: Default::default(),
            initial_load: Default::default(),
        })
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        let id = expanded_node.id.to_u32();

        // send update message when relevant properties change
        let weak_self_ref = Rc::downgrade(&expanded_node);
        let cloned_context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(ImagePatch {
            id,
            ..Default::default()
        }));

        let (source, fit) = expanded_node.with_properties_unwrapped(|props: &mut Image| {
            (props.source.clone(), props.fit.clone())
        });

        let deps = [source.untyped()];
        let needs_to_load_data = Rc::clone(&self.needs_to_load_data);
        expanded_node
            .changed_listener
            .replace_with(Property::computed(
                move || {
                    let Some(expanded_node) = weak_self_ref.upgrade() else {
                        unreachable!()
                    };
                    let mut old_state = borrow_mut!(last_patch);

                    let mut patch = ImagePatch {
                        id,
                        ..Default::default()
                    };
                    expanded_node.with_properties_unwrapped(|props: &mut Image| {
                        let source = props.source.get();
                        match source {
                            ImageSource::Empty => (),
                            ImageSource::Url(url) => {
                                let update =
                                    patch_if_needed(&mut old_state.path, &mut patch.path, url);

                                if update {
                                    cloned_context.enqueue_native_message(
                                        pax_message::NativeMessage::ImageLoad(patch),
                                    );
                                }
                            }
                            ImageSource::Data(_width, _height, _data) => {
                                // insert to notify during render it needs to reload
                                borrow_mut!(needs_to_load_data).insert(expanded_node.id.clone());
                            }
                        }
                    });
                    cloned_context
                        .set_canvas_dirty(expanded_node.occlusion.get().occlusion_layer_id)
                },
                &deps,
            ));

        let tab = expanded_node.transform_and_bounds.clone();
        let deps = &[tab.untyped(), source.untyped(), fit.untyped()];
        let cloned_expanded_node = expanded_node.clone();
        let cloned_context = context.clone();
    }

    fn update(self: Rc<Self>, _expanded_node: &Rc<ExpandedNode>, _context: &Rc<RuntimeContext>) {}

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, _context: &Rc<RuntimeContext>) {
        let id = expanded_node.id.clone();
        expanded_node
            .changed_listener
            .replace_with(Property::default());
        borrow_mut!(self.needs_to_load_data).remove(&id);
    }

    fn render(
        &self,
        expanded_node: &ExpandedNode,
        rtc: &Rc<RuntimeContext>,
        rc: &mut dyn RenderContext,
    ) {
        let layer_id = expanded_node.occlusion.get().occlusion_layer_id;

        let t_and_b = expanded_node.transform_and_bounds.get();
        let (container_width, container_height) = t_and_b.bounds;

        expanded_node.with_properties_unwrapped(|props: &mut Image| {
            let image_size_and_load_path = props.source.read(|source| {
                match source {
                    ImageSource::Empty => return None,
                    ImageSource::Url(url) => {
                        let Some((image_width, image_height)) = rc.get_image_size(&url) else {
                            // image not loaded yet
                            return None;
                        };
                        Some((image_width, image_height, url.to_string()))
                    }
                    &ImageSource::Data(width, height, ref data) => {
                        let mut last_images = borrow_mut!(self.needs_to_load_data);
                        let unique_ident = format!("raw-image-ref-{}", expanded_node.id.to_u32());
                        if last_images.contains(&expanded_node.id) {
                            let mut data_of_correct_len = vec![0; width * height * 4];
                            data_of_correct_len[0..data.len()].copy_from_slice(&data);
                            rc.load_image(&unique_ident, &data_of_correct_len, width, height);
                            last_images.remove(&expanded_node.id);
                        }
                        Some((width, height, unique_ident))
                    }
                }
            });

            let Some((image_width, image_height, path)) = image_size_and_load_path else {
                return;
            };

            if !rtc.is_canvas_dirty(&layer_id)
                && self.initial_load.borrow().contains(&expanded_node.id)
            {
                return;
            }

            let (image_width, image_height) = (image_width as f64, image_height as f64);
            let stretch_w = container_width / image_width;
            let stretch_h = container_height / image_height;
            let (width, height) = match props.fit.get() {
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
            let clip_path = kurbo::Rect::new(0.0, 0.0, container_width, container_height);
            rc.save(layer_id);
            rc.transform(layer_id, t_and_b.transform.into());
            rc.clip(layer_id, clip_path.into_path(0.01));
            rc.draw_image(layer_id, &path, transformed_bounds);
            rc.restore(layer_id);
            self.initial_load
                .borrow_mut()
                .insert(expanded_node.id.clone());
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

/// Image fit/layout options
#[pax]
#[derive(PartialEq)]
#[engine_import_path("pax_engine")]
pub enum ImageFit {
    /// Scale the image to perfectly fit within it's bounds, choosing vertical or horizontal
    /// based on which of them makes it fill the container, possibly clipping parts of the image
    Fill,
    /// Scale the image to perfectly fit within it's bounds, without clipping the image, possibly leaving some
    /// of the available container area embty.
    #[default]
    Fit,
    /// Stretch the image to fit the container
    Stretch,
}
