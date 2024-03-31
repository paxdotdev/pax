use std::cell::RefCell;

use pax_message::{AnyCreatePatch, TextboxPatch};
use pax_runtime::api::Layer;
use pax_runtime::declarative_macros::handle_vtable_update;
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use pax_std::primitives::Textbox;
use std::collections::HashMap;
use std::rc::Rc;

use crate::patch_if_needed;

pub struct TextboxInstance {
    base: BaseInstance,
    //Used as a cache of last-sent values, for crude dirty-checking.
    //Hopefully, this will by obviated by the built-in expression dirty-checking mechanism.
    last_patches: RefCell<HashMap<Vec<u32>, pax_message::TextboxPatch>>,
}

impl InstanceNode for TextboxInstance {
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
            last_patches: Default::default(),
        })
    }

    fn update(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RefCell<RuntimeContext>>,
    ) {
        expanded_node.with_properties_unwrapped(|properties: &mut Textbox| {
            let tbl = &context.borrow().expression_table();
            let stk = &expanded_node.stack;
            handle_vtable_update(
                tbl,
                stk,
                &mut properties.focus_on_mount,
                context.borrow().globals(),
            );
            handle_vtable_update(tbl, stk, &mut properties.text, context.borrow().globals());
            handle_vtable_update(tbl, stk, &mut properties.stroke, context.borrow().globals());
            handle_vtable_update(
                tbl,
                stk,
                &mut properties.stroke.get().color,
                context.borrow().globals(),
            );
            handle_vtable_update(
                tbl,
                stk,
                &mut properties.stroke.get().width,
                context.borrow().globals(),
            );
            handle_vtable_update(
                tbl,
                stk,
                &mut properties.border_radius,
                context.borrow().globals(),
            );
            handle_vtable_update(
                tbl,
                stk,
                &mut properties.background,
                context.borrow().globals(),
            );
            // Style
            handle_vtable_update(tbl, stk, &mut properties.style, context.borrow().globals());
            let stl = properties.style.get();
            handle_vtable_update(tbl, stk, &stl.fill, context.borrow().globals());
            handle_vtable_update(tbl, stk, &stl.font, context.borrow().globals());
            handle_vtable_update(tbl, stk, &stl.font_size, context.borrow().globals());
            handle_vtable_update(tbl, stk, &stl.underline, context.borrow().globals());
            handle_vtable_update(tbl, stk, &stl.align_vertical, context.borrow().globals());
            handle_vtable_update(tbl, stk, &stl.align_horizontal, context.borrow().globals());
            handle_vtable_update(tbl, stk, &stl.align_multiline, context.borrow().globals());
        });
    }

    fn handle_native_patches(
        &self,
        expanded_node: &ExpandedNode,
        context: &Rc<RefCell<RuntimeContext>>,
    ) {
        let id_chain = expanded_node.id_chain.clone();
        let mut patch = TextboxPatch {
            id_chain: id_chain.clone(),
            ..Default::default()
        };
        let mut last_patches = self.last_patches.borrow_mut();
        let old_state = last_patches
            .entry(id_chain.clone())
            .or_insert(patch.clone());

        expanded_node.with_properties_unwrapped(|properties: &mut Textbox| {
            let layout_properties = expanded_node.layout_properties.borrow();
            let computed_tab = &layout_properties.as_ref().unwrap().computed_tab;
            let updates = [
                patch_if_needed(
                    &mut old_state.text,
                    &mut patch.text,
                    properties.text.get().string.clone(),
                ),
                patch_if_needed(
                    &mut old_state.size_x,
                    &mut patch.size_x,
                    computed_tab.bounds.0,
                ),
                patch_if_needed(
                    &mut old_state.size_y,
                    &mut patch.size_y,
                    computed_tab.bounds.1,
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
                    &mut old_state.stroke_color,
                    &mut patch.stroke_color,
                    (&properties.stroke.get().color.get()).into(),
                ),
                patch_if_needed(
                    &mut old_state.stroke_width,
                    &mut patch.stroke_width,
                    properties
                        .stroke
                        .get()
                        .width
                        .get()
                        .get_pixels(computed_tab.bounds.0),
                ),
                patch_if_needed(
                    &mut old_state.background,
                    &mut patch.background,
                    (&properties.background.get()).into(),
                ),
                patch_if_needed(
                    &mut old_state.border_radius,
                    &mut patch.border_radius,
                    properties.border_radius.get().to_float(),
                ),
                patch_if_needed(
                    &mut old_state.focus_on_mount,
                    &mut patch.focus_on_mount,
                    properties.focus_on_mount.get(),
                ),
            ];
            if updates.into_iter().any(|v| v == true) {
                context
                    .borrow_mut()
                    .enqueue_native_message(pax_message::NativeMessage::TextboxUpdate(patch));
            }
        });
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RefCell<RuntimeContext>>,
    ) {
        context
            .borrow_mut()
            .enqueue_native_message(pax_message::NativeMessage::TextboxCreate(AnyCreatePatch {
                id_chain: expanded_node.id_chain.clone(),
                clipping_ids: vec![],
                scroller_ids: vec![],
                z_index: 0,
            }));
    }

    fn handle_unmount(
        &self,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RefCell<RuntimeContext>>,
    ) {
        let id_chain = expanded_node.id_chain.clone();
        context
            .borrow_mut()
            .enqueue_native_message(pax_message::NativeMessage::TextboxDelete(id_chain));
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Textbox").finish_non_exhaustive()
    }
}
