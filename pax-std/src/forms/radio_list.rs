use crate::{Font, TextAlignHorizontal, TextAlignVertical, TextStyle};
use pax_message::{AnyCreatePatch, NativeInterrupt, RadioListPatch};
use pax_runtime::api as pax_runtime_api;
use pax_runtime::api::{use_RefCell, Layer, Property};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use_RefCell!();
use pax_runtime_api::*;

use pax_engine::pax;
use std::rc::Rc;

use crate::common::{native_surface_opacity, patch_if_needed, patch_liquid_glass_if_needed};

/// A radio list control, delegating to a platform-specific native control.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::forms::radio_list::RadioListInstance")]
#[custom(Default)]
pub struct RadioList {
    /// Radio button background color when unchecked.
    pub background: Property<Color>,
    /// Radio button background color when checked.
    pub background_checked: Property<Color>,
    /// Radio button outline stroke.
    pub outline: Property<Stroke>,
    /// List of selectable option labels.
    pub options: Property<Vec<String>>,
    /// Index of the currently selected option.
    pub selected_id: Property<u32>,
    /// Text style for option labels.
    pub style: Property<TextStyle>,
}

impl Default for RadioList {
    fn default() -> Self {
        Self {
            background: Property::new(Color::rgb(243.into(), 244.into(), 246.into())),
            background_checked: Property::new(Color::rgb(27.into(), 100.into(), 242.into())),
            outline: Property::new(Stroke {
                color: Property::new(Color::rgb(209.into(), 213.into(), 219.into())),
                width: Property::new(Size::Pixels(1.into())),
                cap: Property::new(StrokeCap::default()),
            }),
            options: Property::new(vec!["option 1".to_string(), "option 2".to_string()]),
            selected_id: Property::new(0),
            style: Property::new(TextStyle {
                font: Property::new(Font::default()),
                font_size: Property::new(Size::Pixels(Numeric::F64(14.0))),
                fill: Property::new(Fill::Solid(Color::BLACK)),
                underline: Property::new(false),
                align_horizontal: Property::new(TextAlignHorizontal::Left),
                align_multiline: Property::new(TextAlignHorizontal::Left),
                align_vertical: Property::new(TextAlignVertical::Center),
            }),
        }
    }
}
// Runtime instance backing `<RadioList>`.
pub struct RadioListInstance {
    base: BaseInstance,
}

impl InstanceNode for RadioListInstance {
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
        context.enqueue_native_message(pax_message::NativeMessage::RadioListCreate(
            AnyCreatePatch {
                id: id.to_u32(),
                parent_frame: expanded_node.parent_frame.get().map(|v| v.to_u32()),
                render_layer_id: 0,
            },
        ));

        // send update message when relevant properties change
        let weak_self_ref = Rc::downgrade(&expanded_node);
        let context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(RadioListPatch {
            id: id.to_u32(),
            ..Default::default()
        }));

        let deps: Vec<_> = borrow_mut!(expanded_node.properties_scope)
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
                    let id = expanded_node.id.clone();
                    let mut old_state = borrow_mut!(last_patch);

                    let mut patch = RadioListPatch {
                        id: id.to_u32(),
                        ..Default::default()
                    };
                    expanded_node.with_properties_unwrapped(|properties: &mut RadioList| {
                        let computed_tab = expanded_node.transform_and_bounds.get();
                        let (width, height) = computed_tab.bounds;
                        let updates = [
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
                                &mut old_state.style,
                                &mut patch.style,
                                (&properties.style.get()).into(),
                            ),
                            patch_if_needed(
                                &mut old_state.background,
                                &mut patch.background,
                                (&properties.background.get()).into(),
                            ),
                            patch_if_needed(
                                &mut old_state.background_checked,
                                &mut patch.background_checked,
                                (&properties.background_checked.get()).into(),
                            ),
                            patch_if_needed(
                                &mut old_state.outline_color,
                                &mut patch.outline_color,
                                (&properties.outline.get().color.get()).into(),
                            ),
                            patch_if_needed(
                                &mut old_state.outline_width,
                                &mut patch.outline_width,
                                properties
                                    .outline
                                    .get()
                                    .width
                                    .get()
                                    .expect_pixels()
                                    .to_float(),
                            ),
                            patch_if_needed(
                                &mut old_state.selected_id,
                                &mut patch.selected_id,
                                properties.selected_id.get(),
                            ),
                            patch_if_needed(
                                &mut old_state.options,
                                &mut patch.options,
                                properties.options.get(),
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
                                pax_message::NativeMessage::RadioListUpdate(patch),
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
        context.enqueue_native_message(pax_message::NativeMessage::RadioListDelete(id.to_u32()));
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("RadioList").finish_non_exhaustive()
    }

    fn property_requires_occlusion_recompute(&self, _property_name: &str) -> bool {
        false
    }

    fn handle_native_interrupt(
        &self,
        expanded_node: &Rc<ExpandedNode>,
        interrupt: &NativeInterrupt,
    ) {
        if let NativeInterrupt::FormRadioListChange(args) = interrupt {
            expanded_node.with_properties_unwrapped(|props: &mut RadioList| {
                props.selected_id.set(args.selected_id)
            });
        }
    }
}
