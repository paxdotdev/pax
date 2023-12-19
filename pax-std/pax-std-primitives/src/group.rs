use pax_core::{
    with_properties_unwrapped, BaseInstance, ExpandedNode, InstanceFlags, InstanceNode,
    InstantiationArgs, PropertiesTreeContext,
};
use pax_std::primitives::Group;
use std::{cell::RefCell, rc::Rc};

use pax_runtime_api::Layer;

/// Gathers a set of children underneath a single render node:
/// useful for composing transforms and simplifying render trees.
pub struct GroupInstance {
    base: BaseInstance,
}

impl InstanceNode for GroupInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self>
    where
        Self: Sized,
    {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    layer: Layer::DontCare,
                },
            ),
        })
    }

    fn expand(self: Rc<Self>, ptc: &mut PropertiesTreeContext) -> Rc<RefCell<ExpandedNode>> {
        let this_expanded_node = self
            .base()
            .expand_from_instance(Rc::clone(&self) as Rc<dyn InstanceNode>, ptc);
        for child in self.base().get_children() {
            let mut new_ptc = ptc.clone();
            let child_expanded_node = Rc::clone(&child).expand(&mut new_ptc);
            child_expanded_node.borrow_mut().parent_expanded_node =
                Rc::downgrade(&this_expanded_node);
            this_expanded_node
                .borrow_mut()
                .append_child_expanded_node(child_expanded_node);
        }
        this_expanded_node
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        match expanded_node {
            Some(expanded_node) => {
                with_properties_unwrapped!(
                    &expanded_node.get_properties(),
                    Group,
                    |_g: &mut Group| { f.debug_struct("Group").finish() }
                )
            }
            None => f.debug_struct("Group").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}
