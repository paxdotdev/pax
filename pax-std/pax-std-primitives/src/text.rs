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
        _rc: &mut dyn RenderContext,
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
