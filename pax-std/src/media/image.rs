use kurbo::{Affine, Rect, Shape};
use pax_engine::*;
use pax_runtime::api::{borrow_mut, use_RefCell};
use pax_runtime::{api::Property, api::RenderContext, ExpandedNodeIdentifier};
use std::collections::HashSet;

use_RefCell!();
use pax_message::ImagePatch;
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use std::rc::Rc;

use crate::common::{begin_bounded_canvas_node, patch_if_needed};

/// A GPU/canvas-rendered image decoded by the active chassis.
///
/// `Image` draws into the node bounds and participates in the same canvas
/// rendering path as vectors. Use `NativeImage` when a platform-native image
/// element is preferable.
/// Node and inherited opacity multiply the source pixels' alpha at draw time.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::media::image::ImageInstance")]
pub struct Image {
    /// Image source: empty, URL, or raw RGBA data.
    pub source: Property<ImageSource>,
    /// How the image should fit into this node's bounds.
    pub fit: Property<ImageFit>,
}

/// Source data for an `Image`.
///
/// In Pax templates, a string in an `ImageSource` context is shorthand for
/// [`ImageSource::Url`]:
///
/// ```pax
/// <Image source="assets/spaceship.png" />
/// <Image source={avatar_url} />
/// ```
///
/// [`ImageSource::Url`] and [`ImageSource::Data`] remain available as explicit
/// constructor forms. Unlike the list shorthands used by compound Pax types,
/// the URL shorthand has no positional ("magic index") fields: the entire
/// string is the URL or chassis-relative asset path.
#[pax]
#[derive(PartialEq)]
#[engine_import_path("pax_engine")]
#[custom(CoercionRules)]
pub enum ImageSource {
    /// No image.
    #[default]
    Empty,
    /// Image loaded from a URL/path understood by the chassis.
    Url(String),
    /// Raw RGBA image data: width, height, and bytes where `len = width * height * 4`.
    Data(usize, usize, Vec<u8>),
}

impl CoercionRules for ImageSource {
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        match value {
            PaxValue::String(url) => Ok(Self::Url(url)),
            PaxValue::Enum(contents) => {
                let (enum_name, variant_name, values) = *contents;
                if enum_name != "ImageSource" {
                    return Err(format!(
                        "expected ImageSource enum value, got {enum_name}::{variant_name}"
                    ));
                }

                let expected_arity = match variant_name.as_str() {
                    "Empty" => 0,
                    "Url" => 1,
                    "Data" => 3,
                    _ => {
                        return Err(format!(
                            "failed to coerce ImageSource: unknown variant {variant_name:?}"
                        ))
                    }
                };
                if values.len() != expected_arity {
                    return Err(format!(
                        "ImageSource::{variant_name} expects {expected_arity} argument(s), got {}",
                        values.len()
                    ));
                }

                let mut values = values.into_iter();
                match variant_name.as_str() {
                    "Empty" => Ok(Self::Empty),
                    "Url" => Ok(Self::Url(String::try_coerce(values.next().unwrap())?)),
                    "Data" => Ok(Self::Data(
                        usize::try_coerce(values.next().unwrap())?,
                        usize::try_coerce(values.next().unwrap())?,
                        Vec::<u8>::try_coerce(values.next().unwrap())?,
                    )),
                    _ => unreachable!(),
                }
            }
            PaxValue::Option(value) => match *value {
                Some(value) => Self::try_coerce(value),
                None => Err("None can't be coerced into ImageSource".to_string()),
            },
            value => Err(format!("{value:?} can't be coerced into ImageSource")),
        }
    }
}

#[cfg(test)]
mod image_source_coercion_tests {
    use super::*;

    #[test]
    fn string_coerces_to_url() {
        assert_eq!(
            ImageSource::try_coerce(PaxValue::String("assets/photo.png".to_string())).unwrap(),
            ImageSource::Url("assets/photo.png".to_string())
        );
    }

    #[test]
    fn explicit_variants_remain_supported() {
        let url = PaxValue::Enum(Box::new((
            "ImageSource".to_string(),
            "Url".to_string(),
            vec![PaxValue::String(
                "https://example.com/photo.png".to_string(),
            )],
        )));
        assert_eq!(
            ImageSource::try_coerce(url).unwrap(),
            ImageSource::Url("https://example.com/photo.png".to_string())
        );

        let empty = PaxValue::Enum(Box::new((
            "ImageSource".to_string(),
            "Empty".to_string(),
            vec![],
        )));
        assert_eq!(ImageSource::try_coerce(empty).unwrap(), ImageSource::Empty);

        let data = PaxValue::Enum(Box::new((
            "ImageSource".to_string(),
            "Data".to_string(),
            vec![
                1usize.to_pax_value(),
                1usize.to_pax_value(),
                vec![255u8, 128, 64, 255].to_pax_value(),
            ],
        )));
        assert_eq!(
            ImageSource::try_coerce(data).unwrap(),
            ImageSource::Data(1, 1, vec![255, 128, 64, 255])
        );
    }
}

// Runtime instance backing `<Image>`.
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
        let last_patch = Rc::new(RefCell::new(ImagePatch {
            id,
            ..Default::default()
        }));

        let source =
            expanded_node.with_properties_unwrapped(|props: &mut Image| props.source.clone());

        let tab = expanded_node.transform_and_bounds.clone();
        let deps = [tab.untyped(), expanded_node.computed_opacity.untyped()];
        let cloned_context = context.clone();
        let occlusion = expanded_node.occlusion.clone();
        let expanded_node_id = expanded_node.id;

        let tab_changed = Property::computed(
            move || {
                cloned_context.mark_canvas_node_dirty(expanded_node_id);
                cloned_context.set_canvas_dirty(occlusion.get().render_layer_id);
            },
            &deps,
        );

        let cloned_context = context.clone();
        let deps = [source.untyped()];
        let needs_to_load_data = Rc::clone(&self.needs_to_load_data);
        let source_changed = Property::computed(
            move || {
                let Some(expanded_node) = weak_self_ref.upgrade() else {
                    return;
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
                            let update = patch_if_needed(&mut old_state.path, &mut patch.path, url);

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
                cloned_context.mark_canvas_node_dirty(expanded_node.id);
                cloned_context.set_canvas_dirty(expanded_node.occlusion.get().render_layer_id)
            },
            &deps,
        );

        let deps = [source_changed.untyped(), tab_changed.untyped()];
        expanded_node
            .changed_listener
            .replace_with(Property::computed(
                move || {
                    source_changed.get();
                    tab_changed.get();
                },
                &deps,
            ))
    }

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, _context: &Rc<RuntimeContext>) {
        let id = expanded_node.id.clone();
        expanded_node
            .changed_listener
            .replace_with(Property::default());
        borrow_mut!(self.needs_to_load_data).remove(&id);
    }

    fn resolve_coverage_path(&self, expanded_node: &ExpandedNode) -> Option<kurbo::BezPath> {
        let tab = expanded_node.transform_and_bounds.get();
        let rect = Rect::new(0.0, 0.0, tab.bounds.0, tab.bounds.1);
        Some(Affine::from(tab.transform) * rect.into_path(0.01))
    }

    fn render(
        &self,
        expanded_node: &ExpandedNode,
        rtc: &Rc<RuntimeContext>,
        rc: &mut dyn RenderContext,
    ) {
        let node_dirty = rtc.is_canvas_node_dirty(&expanded_node.id);
        let initial_load_complete = self.initial_load.borrow().contains(&expanded_node.id);

        if !node_dirty && initial_load_complete {
            return;
        }

        let Some(scope) = begin_bounded_canvas_node(rc, expanded_node, rtc) else {
            return;
        };

        let (container_width, container_height) = scope.bounds;
        let mut did_draw = false;

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
            rc.save(scope.layer_id);
            rc.transform(scope.layer_id, scope.surface_transform);
            rc.clip(scope.layer_id, clip_path.into_path(0.01));
            rc.draw_image_with_opacity(
                scope.layer_id,
                &path,
                transformed_bounds,
                expanded_node.computed_opacity.get(),
            );
            rc.restore(scope.layer_id);
            did_draw = true;
            self.initial_load
                .borrow_mut()
                .insert(expanded_node.id.clone());
        });
        if did_draw && rc.end_node(scope.layer_id, scope.node_id) {
            rtc.clear_canvas_node_dirty(&expanded_node.id);
        } else {
            let _ = rc.end_node(scope.layer_id, scope.node_id);
        }
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

/// Image fit/layout options.
#[pax]
#[derive(PartialEq)]
#[engine_import_path("pax_engine")]
pub enum ImageFit {
    /// Scale the image to fill its bounds, possibly clipping part of the image.
    Fill,
    /// Scale the image to fit within its bounds without clipping, possibly leaving empty space.
    #[default]
    Fit,
    /// Stretch the image to exactly match the container.
    Stretch,
}

#[cfg(test)]
mod tests;
