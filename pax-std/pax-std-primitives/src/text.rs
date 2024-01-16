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

use crate::patch_if_needed;

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

    fn update(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, context: &mut RuntimeContext) {
        expanded_node.with_properties_unwrapped(|properties: &mut Text| {
            let tbl = context.expression_table();
            let stk = &expanded_node.stack;
            handle_vtable_update(tbl, stk, &mut properties.text);

            // Style
            handle_vtable_update(tbl, stk, &mut properties.style);
            let stl = properties.style.get_mut();
            handle_vtable_update(tbl, stk, &mut stl.fill);
            handle_vtable_update(tbl, stk, &mut stl.font);
            handle_vtable_update(tbl, stk, &mut stl.font_size);
            handle_vtable_update(tbl, stk, &mut stl.underline);
            handle_vtable_update(tbl, stk, &mut stl.align_vertical);
            handle_vtable_update(tbl, stk, &mut stl.align_horizontal);
            handle_vtable_update(tbl, stk, &mut stl.align_multiline);
        });
    }

    fn handle_native_patches(&self, expanded_node: &ExpandedNode, context: &mut RuntimeContext) {
        let id_chain = expanded_node.id_chain.clone();
        let mut patch = TextPatch {
            id_chain: id_chain.clone(),
            ..Default::default()
        };
        let mut last_patches = self.last_patches.borrow_mut();
        let old_state = last_patches
            .entry(id_chain.clone())
            .or_insert(patch.clone());

        expanded_node.with_properties_unwrapped(|properties: &mut Text| {
            let layout_properties = expanded_node.layout_properties.borrow();
            let computed_tab = &layout_properties.as_ref().unwrap().computed_tab;
            let update_needed =

            // Content
                patch_if_needed(
                &mut old_state.content,
                &mut patch.content,
                properties.text.get().string.clone(),
            )

            // Styles
              || patch_if_needed(
                &mut old_state.style,
                &mut patch.style,
                properties.style.get().into(),
            ) || patch_if_needed(
                &mut old_state.style_link,
                &mut patch.style_link,
                properties.style_link.get().into(),
            )

            // Transform and bounds 
              || patch_if_needed(
                &mut old_state.size_x,
                &mut patch.size_x,
                computed_tab.bounds.0,
            ) || patch_if_needed(
                &mut old_state.size_y,
                &mut patch.size_y,
                computed_tab.bounds.1,
            ) || patch_if_needed(
                &mut old_state.transform,
                &mut patch.transform,
                computed_tab.transform.as_coeffs().to_vec(),
            );

            if update_needed {
                context.enqueue_native_message(pax_message::NativeMessage::TextUpdate(patch));
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

    fn handle_mount(&self, expanded_node: &Rc<ExpandedNode>, context: &mut RuntimeContext) {
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

    fn handle_unmount(&self, expanded_node: &Rc<ExpandedNode>, context: &mut RuntimeContext) {
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
