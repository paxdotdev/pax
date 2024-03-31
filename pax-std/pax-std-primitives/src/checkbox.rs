use std::cell::RefCell;

use pax_message::{AnyCreatePatch, CheckboxPatch};
use pax_runtime::api::Layer;
use pax_runtime::declarative_macros::handle_vtable_update;
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use pax_std::primitives::Checkbox;
use std::collections::HashMap;
use std::rc::Rc;

use crate::patch_if_needed;

pub struct CheckboxInstance {
    base: BaseInstance,
    //Used as a cache of last-sent values, for crude dirty-checking.
    //Hopefully, this will by obviated by the built-in expression dirty-checking mechanism.
    //Note: must build in awareness of id_chain, since each virtual instance if this single `Checkbox` instance
    //      shares this last_patches cache
    last_patches: RefCell<HashMap<Vec<u32>, pax_message::CheckboxPatch>>,
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
        expanded_node.with_properties_unwrapped(|properties: &mut Checkbox| {
            handle_vtable_update(
                &context.borrow().expression_table(),
                &expanded_node.stack,
                &mut properties.checked,
                context.borrow().globals(),
            );
        });
    }

    fn handle_native_patches(
        &self,
        expanded_node: &ExpandedNode,
        context: &Rc<RefCell<RuntimeContext>>,
    ) {
        let id_chain = expanded_node.id_chain.clone();
        let mut patch = CheckboxPatch {
            id_chain: id_chain.clone(),
            ..Default::default()
        };
        let mut last_patches = self.last_patches.borrow_mut();
        let old_state = last_patches
            .entry(id_chain.clone())
            .or_insert(patch.clone());

        expanded_node.with_properties_unwrapped(|properties: &mut Checkbox| {
            let layout_properties = expanded_node.layout_properties.borrow();
            let computed_tab = &layout_properties.as_ref().unwrap().computed_tab;
            let updates = [
                patch_if_needed(
                    &mut old_state.checked,
                    &mut patch.checked,
                    properties.checked.get(),
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
            ];
            if updates.into_iter().any(|v| v == true) {
                context
                    .borrow_mut()
                    .enqueue_native_message(pax_message::NativeMessage::CheckboxUpdate(patch));
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
            .enqueue_native_message(pax_message::NativeMessage::CheckboxCreate(AnyCreatePatch {
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
            .enqueue_native_message(pax_message::NativeMessage::CheckboxDelete(id_chain));
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
        f.debug_struct("Checkbox").finish_non_exhaustive()
    }
}
