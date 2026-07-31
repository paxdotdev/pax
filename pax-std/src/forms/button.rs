use crate::{Font, TextAlignHorizontal, TextAlignVertical};
use pax_engine::api::Fill;
use pax_engine::*;
use pax_message::{AnyCreatePatch, ButtonPatch};
use pax_runtime::api::{borrow, borrow_mut, use_RefCell, Color, Numeric, Size, Stroke};
use pax_runtime::api::{Layer, Property};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use std::rc::Rc;
use_RefCell!();
use crate::common::{native_surface_opacity, patch_if_needed, patch_liquid_glass_if_needed};
use crate::TextStyle;

/// A button control, delegating to a platform-specific native button.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::forms::button::ButtonInstance")]
#[custom(Default)]
pub struct Button {
    /// Text label displayed inside the button.
    pub label: Property<String>,
    /// Button background color.
    pub color: Property<Color>,
    /// Button background color while hovered, when supported.
    pub hover_color: Property<Color>,
    /// Button corner radius, in pixels.
    pub corner_radius: Property<f64>,
    /// Button outline stroke.
    pub outline: Property<Stroke>,
    /// Text style applied to the label.
    pub style: Property<TextStyle>,
}

impl Default for Button {
    fn default() -> Self {
        Self {
            color: Property::new(Color::GRAY),
            hover_color: Property::new(Color::INDIGO),
            corner_radius: Property::new(8.0),
            label: Property::new(String::from("button")),
            style: Property::new(TextStyle {
                font: Property::new(Font::default()),
                font_size: Property::new(Size::Pixels(Numeric::F64(20.0))),
                fill: Property::new(Fill::Solid(Color::WHITE)),
                underline: Property::new(false),
                align_multiline: Property::new(TextAlignHorizontal::Center),
                align_vertical: Property::new(TextAlignVertical::Center),
                align_horizontal: Property::new(TextAlignHorizontal::Center),
            }),

            outline: Property::new(Stroke::default()),
        }
    }
}

// Runtime instance backing `<Button>`.
pub struct ButtonInstance {
    base: BaseInstance,
}

impl InstanceNode for ButtonInstance {
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
        // Send creation message
        let id = expanded_node.id.clone();
        context.enqueue_native_message(pax_message::NativeMessage::ButtonCreate(AnyCreatePatch {
            id: id.to_u32(),
            parent_frame: expanded_node.parent_frame.get().map(|v| v.to_u32()),
            render_layer_id: 0,
        }));

        // send update message when relevant properties change
        let weak_self_ref = Rc::downgrade(&expanded_node);
        let context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(ButtonPatch {
            id: id.to_u32(),
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
                expanded_node.liquid_glass_scope.untyped(),
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

                    let mut patch = ButtonPatch {
                        id: expanded_node.id.to_u32(),
                        ..Default::default()
                    };
                    expanded_node.with_properties_unwrapped(|properties: &mut Button| {
                        let computed_tab = expanded_node.transform_and_bounds.get();
                        let (width, height) = computed_tab.bounds;
                        let updates = [
                            patch_if_needed(
                                &mut old_state.outline_stroke_color,
                                &mut patch.outline_stroke_color,
                                (&properties.outline.get().color.get()).into(),
                            ),
                            patch_if_needed(
                                &mut old_state.outline_stroke_width,
                                &mut patch.outline_stroke_width,
                                properties
                                    .outline
                                    .get()
                                    .width
                                    .get()
                                    .expect_pixels()
                                    .to_float(),
                            ),
                            patch_if_needed(
                                &mut old_state.hover_color,
                                &mut patch.hover_color,
                                (&properties.hover_color.get()).into(),
                            ),
                            patch_if_needed(
                                &mut old_state.corner_radius,
                                &mut patch.corner_radius,
                                properties.corner_radius.get(),
                            ),
                            patch_if_needed(
                                &mut old_state.content,
                                &mut patch.content,
                                properties.label.get(),
                            ),
                            patch_if_needed(
                                &mut old_state.color,
                                &mut patch.color,
                                (&properties.color.get()).into(),
                            ),
                            patch_if_needed(
                                &mut old_state.style,
                                &mut patch.style,
                                (&properties.style.get()).into(),
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
                            patch_liquid_glass_if_needed(
                                &mut old_state.liquid_glass,
                                &mut patch.liquid_glass,
                                &expanded_node,
                            ),
                        ];
                        if updates.into_iter().any(|v| v == true) {
                            context.enqueue_native_message(
                                pax_message::NativeMessage::ButtonUpdate(patch),
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
        context.enqueue_native_message(pax_message::NativeMessage::ButtonDelete(id.to_u32()));
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn property_requires_occlusion_recompute(&self, _property_name: &str) -> bool {
        false
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Button").finish_non_exhaustive()
    }
}
