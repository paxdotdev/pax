use pax_message::{AnyCreatePatch, NativeInterrupt, RadioSetPatch};
use pax_runtime::api::{use_RefCell, Layer, Property};
use pax_runtime::{
    BaseInstance, ExpandedNode, ExpandedNodeIdentifier, InstanceFlags, InstanceNode,
    InstantiationArgs, RuntimeContext,
};
use pax_runtime::api as pax_runtime_api;
use_RefCell!();
use pax_runtime_api::*;
use crate::*;

use std::collections::HashMap;
use std::rc::Rc;
use pax_engine::pax;

use crate::common::patch_if_needed;

/// A platform-native radio set list
#[pax]
#[primitive("pax_std::forms::radio_set::RadioSetInstance")]
#[custom(Default)]
pub struct RadioSet {
    pub background: Property<Color>,
    pub background_checked: Property<Color>,
    pub outline: Property<Stroke>,
    pub options: Property<Vec<String>>,
    pub selected_id: Property<u32>,
    pub style: Property<TextStyle>,
}

impl Default for RadioSet {
    fn default() -> Self {
        Self {
            background: Property::new(Color::rgb(243.into(), 244.into(), 246.into())),
            background_checked: Property::new(Color::rgb(27.into(), 100.into(), 242.into())),
            outline: Property::new(Stroke {
                color: Property::new(Color::rgb(209.into(), 213.into(), 219.into())),
                width: Property::new(Size::Pixels(1.into())),
            }),
            options: Property::new(vec!["option 1".to_string(), "option 2".to_string()]),
            selected_id: Property::new(0),
            style: Property::new(TextStyle {
                font: Property::new(Font::default()),
                font_size: Property::new(Size::Pixels(Numeric::F64(14.0))),
                fill: Property::new(Color::BLACK),
                underline: Property::new(false),
                align_horizontal: Property::new(TextAlignHorizontal::Left),
                align_multiline: Property::new(TextAlignHorizontal::Left),
                align_vertical: Property::new(TextAlignVertical::Center),
            }),
        }
    }
}
pub struct RadioSetInstance {
    base: BaseInstance,
    native_message_props: RefCell<HashMap<ExpandedNodeIdentifier, Property<()>>>,
}

impl InstanceNode for RadioSetInstance {
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
        // Send creation message
        let id = expanded_node.id.clone();
        context.enqueue_native_message(pax_message::NativeMessage::RadioSetCreate(
            AnyCreatePatch {
                id: id.to_u32(),
                parent_frame: expanded_node.parent_frame.get().map(|v| v.to_u32()),
                occlusion_layer_id: 0,
            },
        ));

        // send update message when relevant properties change
        let weak_self_ref = Rc::downgrade(&expanded_node);
        let context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(RadioSetPatch {
            id: id.to_u32(),
            ..Default::default()
        }));

        let deps: Vec<_> = borrow_mut!(expanded_node.properties_scope)
            .values()
            .cloned()
            .chain([expanded_node.transform_and_bounds.untyped()])
            .collect();
        borrow_mut!(self.native_message_props).insert(
            id,
            Property::computed(
                move || {
                    let Some(expanded_node) = weak_self_ref.upgrade() else {
                        unreachable!()
                    };
                    let id = expanded_node.id.clone();
                    let mut old_state = borrow_mut!(last_patch);

                    let mut patch = RadioSetPatch {
                        id: id.to_u32(),
                        ..Default::default()
                    };
                    expanded_node.with_properties_unwrapped(|properties: &mut RadioSet| {
                        let computed_tab = expanded_node.transform_and_bounds.get();
                        let (width, height) = computed_tab.bounds;
                        let updates = [
                            patch_if_needed(&mut old_state.size_x, &mut patch.size_x, width),
                            patch_if_needed(&mut old_state.size_y, &mut patch.size_y, height),
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
                        ];
                        if updates.into_iter().any(|v| v == true) {
                            context.enqueue_native_message(
                                pax_message::NativeMessage::RadioSetUpdate(patch),
                            );
                        }
                    });
                    ()
                },
                &deps,
            ),
        );
    }

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        let id = expanded_node.id.clone();
        context.enqueue_native_message(pax_message::NativeMessage::RadioSetDelete(id.to_u32()));
        // Reset so that native_message sending updates while unmounted
        borrow_mut!(self.native_message_props).remove(&id);
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("RadioSet").finish_non_exhaustive()
    }

    fn handle_native_interrupt(
        &self,
        expanded_node: &Rc<ExpandedNode>,
        interrupt: &NativeInterrupt,
    ) {
        if let NativeInterrupt::FormRadioSetChange(args) = interrupt {
            expanded_node.with_properties_unwrapped(|props: &mut RadioSet| {
                props.selected_id.set(args.selected_id)
            });
        }
    }
}
