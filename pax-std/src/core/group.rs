use pax_engine::pax;
use pax_runtime::api::{borrow, Layer};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};
use std::iter;
use std::rc::Rc;

/// Gathers a set of children underneath a single render node:
/// useful for composing transforms and simplifying render trees.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::core::group::GroupInstance")]
pub struct Group {}

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
                    is_slot: false,
                },
            ),
        })
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&pax_runtime::ExpandedNode>,
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

    fn handle_control_flow_node_expansion(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        // Detached sidecar trees are not mounted, but mask sources still need a
        // fully expanded subtree so their descendants can contribute coverage.
        let env = Rc::clone(&expanded_node.stack);
        let children = borrow!(self.base().get_instance_children());
        let children_with_envs = children.iter().cloned().zip(iter::repeat(env));
        let children = expanded_node.generate_children(
            children_with_envs,
            context,
            &expanded_node.parent_frame,
            true,
        );
        for child in children.iter() {
            child.recurse_control_flow_expansion(context);
        }
        expanded_node.children.set(children);
    }
}
