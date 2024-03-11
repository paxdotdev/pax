use pax_runtime::{BaseInstance, InstanceFlags, InstanceNode, InstantiationArgs, NodeGroup};
use std::rc::Rc;

use pax_runtime::api::Layer;

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
                    group: NodeGroup::Container,
                    layer: Layer::DontCare,
                    is_component: false,
                },
            ),
        })
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&pax_runtime::ExpandedNode>,
    ) -> std::fmt::Result {
        match expanded_node {
            Some(expanded_node) => {
                expanded_node.with_properties_unwrapped(|_g: &mut pax_std::primitives::Group| {
                    f.debug_struct("Group").finish()
                })
            }
            None => f.debug_struct("Group").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}
