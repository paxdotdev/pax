use pax_message::{AnyCreatePatch, CheckboxPatch};
use pax_runtime::api::{Layer, Property};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use_RefCell!();
use pax_engine::pax;
use pax_runtime::api::*;

use std::rc::Rc;

use crate::common::patch_if_needed;

/// A platform-native checkbox element
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::forms::checkbox::CheckboxInstance")]
#[custom(Default)]
pub struct Checkbox {
    pub background: Property<Color>,
    pub background_checked: Property<Color>,
    pub outline: Property<Stroke>,
    pub border_radius: Property<f64>,

    pub checked: Property<bool>,
}

impl Default for Checkbox {
    fn default() -> Self {
        Self {
            background: Property::new(Color::rgb(243.into(), 244.into(), 246.into())),
            background_checked: Property::new(Color::rgb(27.into(), 100.into(), 242.into())),
            outline: Property::new(Stroke {
                color: Property::new(Color::rgb(209.into(), 213.into(), 219.into())),
                width: Property::new(Size::Pixels(1.into())),
            }),
            border_radius: Property::new(5.0),
            checked: Property::new(false),
        }
    }
}

pub struct CheckboxInstance {
    base: BaseInstance,
}

impl InstanceNode for CheckboxInstance {
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
        context.enqueue_native_message(pax_message::NativeMessage::CheckboxCreate(
            AnyCreatePatch {
                id,
                parent_frame: expanded_node.parent_frame.get().map(|v| v.to_u32()),
                occlusion_layer_id: 0,
            },
        ));
        let weak_self_ref = Rc::downgrade(&expanded_node);
        let context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(CheckboxPatch {
            id,
            ..Default::default()
        }));

        let deps: Vec<_> = borrow!(expanded_node.properties_scope)
            .values()
            .cloned()
            .map(|v| v.get_untyped_property().clone())
            .chain([expanded_node.transform_and_bounds.untyped()])
            .collect();
        expanded_node
            .changed_listener
            .replace_with(Property::computed(
                move || {
                    let Some(expanded_node) = weak_self_ref.upgrade() else {
                        unreachable!()
                    };
                    let mut old_state = borrow_mut!(last_patch);

                    let mut patch = CheckboxPatch {
                        id,
                        ..Default::default()
                    };
                    expanded_node.with_properties_unwrapped(|properties: &mut Checkbox| {
                        let computed_tab = expanded_node.transform_and_bounds.get();
                        let (width, height) = computed_tab.bounds;
                        let updates = [
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
                                &mut old_state.border_radius,
                                &mut patch.border_radius,
                                properties.border_radius.get(),
                            ),
                            patch_if_needed(
                                &mut old_state.checked,
                                &mut patch.checked,
                                properties.checked.get(),
                            ),
                            patch_if_needed(&mut old_state.size_x, &mut patch.size_x, width),
                            patch_if_needed(&mut old_state.size_y, &mut patch.size_y, height),
                            patch_if_needed(
                                &mut old_state.transform,
                                &mut patch.transform,
                                computed_tab.transform.coeffs().to_vec(),
                            ),
                        ];
                        if updates.into_iter().any(|v| v == true) {
                            context.enqueue_native_message(
                                pax_message::NativeMessage::CheckboxUpdate(patch),
                            );
                        }
                    });
                    ()
                },
                &deps,
            ));
    }

    fn handle_native_interrupt(
        &self,
        expanded_node: &Rc<ExpandedNode>,
        interrupt: &pax_message::NativeInterrupt,
    ) {
        if let pax_message::NativeInterrupt::FormCheckboxToggle(args) = interrupt {
            expanded_node.with_properties_unwrapped(|checkbox: &mut Checkbox| {
                checkbox.checked.set(args.state);
            });
        } else {
            log::warn!("checkbox element was handed interrupt it doesn't use");
        }
    }

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        let id = expanded_node.id.clone();
        expanded_node
            .changed_listener
            .replace_with(Property::default());
        context.enqueue_native_message(pax_message::NativeMessage::CheckboxDelete(id.to_u32()));
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Checkbox").finish_non_exhaustive()
    }
}
