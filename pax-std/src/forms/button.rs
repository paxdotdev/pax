use crate::*;
use pax_engine::*;
use pax_message::{AnyCreatePatch, ButtonPatch};
use pax_runtime::api as pax_runtime_api;
use pax_runtime::api::{borrow, borrow_mut, use_RefCell, Color, Numeric, Size, Stroke};
use pax_runtime::api::{Layer, Property};
use pax_runtime::{
    BaseInstance, ExpandedNode, ExpandedNodeIdentifier, InstanceFlags, InstanceNode,
    InstantiationArgs, RuntimeContext,
};
use std::collections::HashMap;
use std::rc::Rc;
use_RefCell!();
use crate::common::patch_if_needed;
use crate::TextStyle;

#[pax]
#[primitive("pax_std::forms::button::ButtonInstance")]
#[custom(Default)]
pub struct Button {
    pub label: Property<String>,
    pub color: Property<Color>,
    pub hover_color: Property<Color>,
    pub border_radius: Property<f64>,
    pub outline: Property<Stroke>,
    pub style: Property<TextStyle>,
}

impl Default for Button {
    fn default() -> Self {
        Self {
            color: Property::new(Color::rgb(27.into(), 100.into(), 242.into())),
            hover_color: Property::new(Color::rgb(26.into(), 86.into(), 219.into())),
            border_radius: Property::new(8.0),
            label: Property::new(String::from("button")),
            style: Property::new(TextStyle {
                font: Property::new(Font::default()),
                font_size: Property::new(Size::Pixels(Numeric::F64(20.0))),
                fill: Property::new(Color::WHITE),
                underline: Property::new(false),
                align_multiline: Property::new(TextAlignHorizontal::Center),
                align_vertical: Property::new(TextAlignVertical::Center),
                align_horizontal: Property::new(TextAlignHorizontal::Center),
            }),

            outline: Property::new(Stroke::default()),
        }
    }
}

pub struct ButtonInstance {
    base: BaseInstance,
    native_message_props: RefCell<HashMap<ExpandedNodeIdentifier, Property<()>>>,
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
        context.enqueue_native_message(pax_message::NativeMessage::ButtonCreate(AnyCreatePatch {
            id: id.to_u32(),
            parent_frame: expanded_node.parent_frame.get().map(|v| v.to_u32()),
            occlusion_layer_id: 0,
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
            .chain([expanded_node.transform_and_bounds.untyped()])
            .collect();
        borrow_mut!(self.native_message_props).insert(
            id,
            Property::computed(
                move || {
                    let Some(expanded_node) = weak_self_ref.upgrade() else {
                        unreachable!()
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
                                &mut old_state.border_radius,
                                &mut patch.border_radius,
                                properties.border_radius.get(),
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
                                &mut old_state.transform,
                                &mut patch.transform,
                                computed_tab.transform.coeffs().to_vec(),
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
            ),
        );
    }

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        let id = expanded_node.id.clone();
        context.enqueue_native_message(pax_message::NativeMessage::ButtonDelete(id.to_u32()));
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
        f.debug_struct("Button").finish_non_exhaustive()
    }
}
