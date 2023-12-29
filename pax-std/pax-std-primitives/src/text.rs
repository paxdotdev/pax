use pax_core::declarative_macros::handle_vtable_update;
use pax_core::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use pax_message::{AnyCreatePatch, TextPatch};
use pax_runtime_api::{Layer, RenderContext};
use pax_std::primitives::Text;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct TextInstance {
    base: BaseInstance,
    //Used as a cache of last-sent values, for crude dirty-checking.
    //Hopefully, this will by obviated by the built-in expression dirty-checking mechanism.
    //Note: must build in awareness of id_chain, since each virtual instance if this single `Text` instance
    //      shares this last_patches cache
    last_patches: RefCell<HashMap<Vec<u32>, pax_message::TextPatch>>,
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
                    invisible_to_raycasting: true, //TODO make this optional?
                    layer: Layer::Native,
                    is_component: false,
                },
            ),
            last_patches: Default::default(),
        })
    }

    fn update_children(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &mut RuntimeContext,
    ) {
        //Doesn't need to expand any children
        expanded_node.with_properties_unwrapped(|properties: &mut Text| {
            handle_vtable_update(
                context.expression_table(),
                &expanded_node.stack,
                &mut properties.text,
            );
        });
    }

    fn handle_native_patches(&self, expanded_node: &ExpandedNode, context: &mut RuntimeContext) {
        let mut last_patches = self.last_patches.borrow_mut();
        let mut new_message: TextPatch = Default::default();
        new_message.id_chain = expanded_node.id_chain.clone();
        if !last_patches.contains_key(&new_message.id_chain) {
            let mut patch = TextPatch::default();
            patch.id_chain = new_message.id_chain.clone();
            last_patches.insert(new_message.id_chain.clone(), patch);
        }
        let last_patch = last_patches.get_mut(&new_message.id_chain).unwrap();
        let mut has_any_updates = false;

        expanded_node.with_properties_unwrapped(|properties: &mut Text| {
            let val = properties.text.get().string.clone();
            let is_new_value = match &last_patch.content {
                Some(cached_value) => !val.eq(cached_value),
                None => true,
            };
            if is_new_value {
                new_message.content = Some(val.clone());
                last_patch.content = Some(val.clone());
                has_any_updates = true;
            }

            let val = properties.style.get();
            let _is_new_val = match &last_patch.style {
                Some(cached_value) => !val.eq(cached_value),
                None => true,
            };

            if is_new_value {
                new_message.style = Some(val.into());
                last_patch.style = Some(val.into());
                has_any_updates = true;
            }

            let val = properties.style_link.get();
            let _is_new_val = match &last_patch.style_link {
                Some(cached_value) => !val.eq(cached_value),
                None => true,
            };

            if is_new_value {
                new_message.style_link = Some(val.into());
                last_patch.style_link = Some(val.into());
                has_any_updates = true;
            }

            let computed_props = expanded_node.computed_expanded_properties.borrow();
            let tab = &computed_props.as_ref().unwrap().computed_tab;

            let val = tab.bounds.0;
            let is_new_value = match &last_patch.size_x {
                Some(cached_value) => !val.eq(cached_value),
                None => true,
            };
            if is_new_value {
                new_message.size_x = Some(val);
                last_patch.size_x = Some(val);
                has_any_updates = true;
            }

            let val = tab.bounds.1;
            let is_new_value = match &last_patch.size_y {
                Some(cached_value) => !val.eq(cached_value),
                None => true,
            };
            if is_new_value {
                new_message.size_y = Some(val);
                last_patch.size_y = Some(val);
                has_any_updates = true;
            }

            let is_new_transform = match &last_patch.transform {
                Some(cached_transform) => tab
                    .transform
                    .as_coeffs()
                    .iter()
                    .enumerate()
                    .any(|(i, elem)| *elem != cached_transform[i]),
                None => true,
            };
            if is_new_transform {
                new_message.transform = Some(tab.transform.as_coeffs().to_vec());
                last_patch.transform = Some(tab.transform.as_coeffs().to_vec());
                has_any_updates = true;
            }

            if has_any_updates {
                context.enqueue_native_message(pax_message::NativeMessage::TextUpdate(new_message));
            }
        });
    }

    fn render(
        &self,
        _expanded_node: &ExpandedNode,
        _context: &mut RuntimeContext,
        _rc: &mut Box<dyn RenderContext>,
    ) {
        //no-op -- only native rendering for Text (unless/until we support rasterizing text, which Piet should be able to handle!)
    }

    fn handle_mount(&self, expanded_node: &ExpandedNode, context: &mut RuntimeContext) {
        // though macOS and iOS don't need this ancestry chain for clipping, Web does
        // let clipping_ids = ptc.get_current_clipping_ids();

        // let scroller_ids = ptc.get_current_scroller_ids();

        let id_chain = expanded_node.id_chain.clone();
        context.enqueue_native_message(pax_message::NativeMessage::TextCreate(AnyCreatePatch {
            id_chain,
            clipping_ids: vec![],
            scroller_ids: vec![],
            z_index: 0,
        }));
    }

    fn handle_unmount(&self, expanded_node: &ExpandedNode, context: &mut RuntimeContext) {
        let id_chain = expanded_node.id_chain.clone();
        context.enqueue_native_message(pax_message::NativeMessage::TextDelete(id_chain));
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        match expanded_node {
            Some(expanded_node) => expanded_node.with_properties_unwrapped(|r: &mut Text| {
                f.debug_struct("Text").field("text", r.text.get()).finish()
            }),
            None => f.debug_struct("Text").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}
/*fn expand_node_and_compute_properties(
    &mut self,
    ptc: &mut PropertiesTreeContext<R>,
) -> Rc<RefCell<ExpandedNode<R>>> {
    //
    // let properties = &mut *self.properties.as_ref().borrow_mut();
    //
    // if let Some(text) = rtc.compute_vtable_value(properties.text._get_vtable_id()) {
    //     let new_value = unsafe_unwrap!(text, TypesCoproduct, StringBox);
    //     properties.text.set(new_value);
    // }
    //
    // if let Some(style_font) =
    //     rtc.compute_vtable_value(properties.style.get().font._get_vtable_id())
    // {
    //     let new_value = unsafe_unwrap!(style_font, TypesCoproduct, Font);
    //     properties.style.get_mut().font.set(new_value);
    // }
    //
    // if let Some(style_font_size) =
    //     rtc.compute_vtable_value(properties.style.get().font_size._get_vtable_id())
    // {
    //     let new_value = unsafe_unwrap!(style_font_size, TypesCoproduct, SizePixels);
    //     properties.style.get_mut().font_size.set(new_value);
    // }
    //
    // if let Some(style_fill) =
    //     rtc.compute_vtable_value(properties.style.get().fill._get_vtable_id())
    // {
    //     let new_value = unsafe_unwrap!(style_fill, TypesCoproduct, Color);
    //     properties.style.get_mut().fill.set(new_value);
    // }
    //
    // if let Some(style_underline) =
    //     rtc.compute_vtable_value(properties.style.get().underline._get_vtable_id())
    // {
    //     let new_value = unsafe_unwrap!(style_underline, TypesCoproduct, bool);
    //     properties.style.get_mut().underline.set(new_value);
    // }
    //
    // if let Some(style_align_multiline) =
    //     rtc.compute_vtable_value(properties.style.get().align_multiline._get_vtable_id())
    // {
    //     let new_value =
    //         unsafe_unwrap!(style_align_multiline, TypesCoproduct, TextAlignHorizontal);
    //     properties.style.get_mut().align_multiline.set(new_value);
    // }
    //
    // if let Some(style_align_vertical) =
    //     rtc.compute_vtable_value(properties.style.get().align_vertical._get_vtable_id())
    // {
    //     let new_value = unsafe_unwrap!(style_align_vertical, TypesCoproduct, TextAlignVertical);
    //     properties.style.get_mut().align_vertical.set(new_value);
    // }
    //
    // if let Some(style_align_horizontal) =
    //     rtc.compute_vtable_value(properties.style.get().align_horizontal._get_vtable_id())
    // {
    //     let new_value =
    //         unsafe_unwrap!(style_align_horizontal, TypesCoproduct, TextAlignHorizontal);
    //     properties.style.get_mut().align_horizontal.set(new_value);
    // }
    //
    // if let Some(style_link) = rtc.compute_vtable_value(properties.style_link._get_vtable_id()) {
    //     let new_value = unsafe_unwrap!(style_link, TypesCoproduct, TextStyle);
    //     properties.style_link.set(new_value);
    // }
    //
    // let style_link = properties.style_link.get_mut();
    // if let Some(style_font) = rtc.compute_vtable_value(style_link.font._get_vtable_id()) {
    //     let new_value = unsafe_unwrap!(style_font, TypesCoproduct, Font);
    //     style_link.font.set(new_value);
    // }
    //
    // if let Some(style_font_size) =
    //     rtc.compute_vtable_value(style_link.font_size._get_vtable_id())
    // {
    //     let new_value = unsafe_unwrap!(style_font_size, TypesCoproduct, SizePixels);
    //     style_link.font_size.set(new_value);
    // }
    //
    // if let Some(style_fill) = rtc.compute_vtable_value(style_link.fill._get_vtable_id()) {
    //     let new_value = unsafe_unwrap!(style_fill, TypesCoproduct, Color);
    //     style_link.fill.set(new_value);
    // }
    //
    // if let Some(style_underline) =
    //     rtc.compute_vtable_value(style_link.underline._get_vtable_id())
    // {
    //     let new_value = unsafe_unwrap!(style_underline, TypesCoproduct, bool);
    //     style_link.underline.set(new_value);
    // }
    //
    // if let Some(style_align_multiline) =
    //     rtc.compute_vtable_value(style_link.align_multiline._get_vtable_id())
    // {
    //     let new_value =
    //         unsafe_unwrap!(style_align_multiline, TypesCoproduct, TextAlignHorizontal);
    //     style_link.align_multiline.set(new_value);
    // }
    //
    // if let Some(style_align_vertical) =
    //     rtc.compute_vtable_value(style_link.align_vertical._get_vtable_id())
    // {
    //     let new_value = unsafe_unwrap!(style_align_vertical, TypesCoproduct, TextAlignVertical);
    //     style_link.align_vertical.set(new_value);
    // }
    //
    // if let Some(style_align_horizontal) =
    //     rtc.compute_vtable_value(style_link.align_horizontal._get_vtable_id())
    // {
    //     let new_value =
    //         unsafe_unwrap!(style_align_horizontal, TypesCoproduct, TextAlignHorizontal);
    //     style_link.align_horizontal.set(new_value);
    // }

    // self.common_properties.compute_properties(rtc);
    todo!()
}*/
