use pax_core::{
    with_properties_unwrapped, BaseInstance, ExpandedNode, InstanceFlags, InstanceNode,
    InstantiationArgs,
};
use pax_std::primitives::Group;
use piet_common::RenderContext;
use std::cell::RefCell;
use std::rc::Rc;

use pax_runtime_api::Layer;

/// Gathers a set of children underneath a single render node:
/// useful for composing transforms and simplifying render trees.
pub struct GroupInstance<R: 'static + RenderContext> {
    base: BaseInstance<R>,
}

impl<R: 'static + RenderContext> InstanceNode<R> for GroupInstance<R> {
    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>>
    where
        Self: Sized,
    {
        Rc::new(RefCell::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
                    layer: Layer::DontCare,
                },
            ),
        }))
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&ExpandedNode<R>>,
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

    fn base(&self) -> &BaseInstance<R> {
        &self.base
    }
}
