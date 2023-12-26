use pax_core::{BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs};
use pax_std::primitives::Group;
use std::rc::Rc;

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
                    is_component: false,
                },
            ),
        })
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        match expanded_node {
            Some(expanded_node) => expanded_node
                .with_properties_unwrapped(|_g: &mut Group| f.debug_struct("Group").finish()),
            None => f.debug_struct("Group").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}
