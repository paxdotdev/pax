use pax_engine::pax;
use pax_runtime::{BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext};
use std::rc::Rc;

use pax_runtime::api::Layer;

/// Allows embedding another Pax #[main] component with
/// a separate manifest via a separate DefinitionToInstanceTraverser
/// Useful at least for embedding userland projects inside pax-designer; may be useful for other purposes
#[pax]
#[primitive("pax_std::core::inline_frame::InlineFrameInstance")]
pub struct InlineFrame {}

pub struct InlineFrameInstance {
    base: BaseInstance,
}

impl InstanceNode for InlineFrameInstance {
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

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        expanded_node: Option<&pax_runtime::ExpandedNode>,
    ) -> std::fmt::Result {
        match expanded_node {
            Some(expanded_node) => expanded_node
                .with_properties_unwrapped(|_g: &mut InlineFrame| f.debug_struct("InlineFrame").finish()),
            None => f.debug_struct("InlineFrame").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn handle_mount(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, ctx: &Rc<RuntimeContext>) {
        let inline_children = vec![ctx.get_userland_root_expanded_node().expect("Unable to load userland component via InlineFrame — has it been registered?")];
        expanded_node.children.set(inline_children);
    }

}
