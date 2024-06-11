use pax_message::{AnyCreatePatch, TextPatch};
use pax_runtime::api::{Layer, Property, RenderContext};
use pax_runtime::{
    BaseInstance, ExpandedNode, ExpandedNodeIdentifier, InstanceFlags, InstanceNode,
    InstantiationArgs, RuntimeContext,
};
use pax_runtime_api::{borrow, borrow_mut, use_RefCell};
use pax_std::primitives::Text;
use_RefCell!();
use std::collections::HashMap;
use std::rc::Rc;
#[cfg(feature = "designtime")]
use {
    kurbo::{RoundedRect, Shape},
    pax_runtime::DEBUG_TEXT_GREEN_BACKGROUND,
    piet::Color,
};

use crate::patch_if_needed;

pub struct TextInstance {
    base: BaseInstance,
    // Properties that listen to Text property changes, and computes
    // a patch in the case that they have changed + sends it as a native
    // message to the chassi. Since InstanceNode -> ExpandedNode has a one
    // to many relationship, needs to be a hashmap
    native_message_props: RefCell<HashMap<ExpandedNodeIdentifier, Property<()>>>,
}

impl InstanceNode for TextInstance {
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

    fn render(
        &self,
        _expanded_node: &ExpandedNode,
        _context: &Rc<RuntimeContext>,
        _rc: &mut dyn RenderContext,
    ) {
        //no-op -- only native rendering for Text (unless/until we support rasterizing text, which Piet should be able to handle!)

        #[cfg(feature = "designtime")]
        if DEBUG_TEXT_GREEN_BACKGROUND {
            let computed_props = borrow!(expanded_node.layout_properties);
            let tab = &computed_props.as_ref().unwrap().computed_tab;
            let layer_id = format!("{}", borrow!(expanded_node.occlusion_id));
            let width: f64 = tab.bounds.0;
            let height: f64 = tab.bounds.1;
            let rect = RoundedRect::new(0.0, 0.0, width, height, 0.0);
            let bez_path = rect.to_path(0.1);
            let transformed_bez_path = Into::<kurbo::Affine>::into(tab.transform) * bez_path;
            rc.fill(
                &layer_id,
                transformed_bez_path,
                &piet::PaintBrush::Color(Color::rgba8(0, 255, 0, 100)),
            );
        }
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        // Send creation message
        let id = expanded_node.id.to_u32();
        context.enqueue_native_message(pax_message::NativeMessage::TextCreate(AnyCreatePatch {
            id,
            parent_frame: expanded_node.parent_frame.get().map(|v| v.to_u32()),
            occlusion_layer_id: 0,
        }));

        // send update message when relevant properties change
        let weak_self_ref = Rc::downgrade(&expanded_node);
        let context = Rc::clone(context);
        let last_patch = Rc::new(RefCell::new(TextPatch {
            id,
            ..Default::default()
        }));

        let deps: Vec<_> = borrow!(expanded_node.properties_scope)
            .values()
            .cloned()
            .chain([expanded_node.transform_and_bounds.untyped()])
            .collect();

        borrow_mut!(self.native_message_props).insert(
            expanded_node.id,
            Property::computed(
                move || {
                    let Some(expanded_node) = weak_self_ref.upgrade() else {
                        unreachable!()
                    };
                    let mut old_state = borrow_mut!(last_patch);

                    let mut patch = TextPatch {
                        id,
                        ..Default::default()
                    };
                    expanded_node.with_properties_unwrapped(|properties: &mut Text| {
                        let computed_tab = expanded_node.transform_and_bounds.get();
                        let (width, height) = computed_tab.bounds;
                        let cp = expanded_node.get_common_properties();
                        let cp = borrow!(cp);
                        // send width/height only if common props exist, otherwise we are in "listening mode"
                        // trying to infer width and height from the engine. To signal this we
                        // send width/height = -1.0, telling chassis that "you tell me!".
                        let (width, height) = (
                            cp.width.get().is_some().then_some(width).unwrap_or(-1.0),
                            cp.height.get().is_some().then_some(height).unwrap_or(-1.0),
                        );

                        let updates = [
                            // Content
                            patch_if_needed(
                                &mut old_state.content,
                                &mut patch.content,
                                properties.text.get().clone(),
                            ),
                            // Styles
                            patch_if_needed(
                                &mut old_state.style,
                                &mut patch.style,
                                (&properties.style.get().clone()).into(),
                            ),
                            patch_if_needed(
                                &mut old_state.style_link,
                                &mut patch.style_link,
                                (&properties.style_link.get().clone()).into(),
                            ),
                            patch_if_needed(
                                &mut old_state.editable,
                                &mut patch.editable,
                                properties.editable.get(),
                            ),
                            // Transform and bounds
                            patch_if_needed(&mut old_state.size_x, &mut patch.size_x, width),
                            patch_if_needed(&mut old_state.size_y, &mut patch.size_y, height),
                            patch_if_needed(
                                &mut old_state.transform,
                                &mut patch.transform,
                                computed_tab.transform.coeffs().to_vec(),
                            ),
                        ];
                        if updates.into_iter().any(|v| v == true) {
                            context.enqueue_native_message(pax_message::NativeMessage::TextUpdate(
                                patch,
                            ));
                        }
                    });
                    ()
                },
                &deps,
            ),
        );
    }

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &Rc<RuntimeContext>) {
        let id = expanded_node.id.to_u32();
        context.enqueue_native_message(pax_message::NativeMessage::TextDelete(id));
        // Reset so that native_message sending updates while unmounted
        borrow_mut!(self.native_message_props).remove(&expanded_node.id);
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        match expanded_node {
            Some(expanded_node) => expanded_node.with_properties_unwrapped(|r: &mut Text| {
                f.debug_struct("Text").field("text", &r.text.get()).finish()
            }),
            None => f.debug_struct("Text").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn handle_text_change(&self, expanded_node: &Rc<ExpandedNode>, text: String) {
        expanded_node.with_properties_unwrapped(|properties: &mut Text| properties.text.set(text));
    }
}
