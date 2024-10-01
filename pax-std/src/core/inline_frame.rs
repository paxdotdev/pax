use pax_engine::{api::pax_value::ToFromPaxAny, pax};
use pax_message::{borrow, borrow_mut};
use pax_runtime::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
    RuntimePropertiesStackFrame,
};
use std::{collections::HashMap, rc::Rc};

pax_message::use_RefCell!();

use pax_runtime::api::Layer;

/// Allows embedding another Pax #[main] component with
/// a separate manifest via a separate DefinitionToInstanceTraverser
/// Useful at least for embedding userland projects inside pax-designer; may be useful for other purposes
#[pax]
#[engine_import_path("pax_engine")]
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
            Some(expanded_node) => {
                expanded_node.with_properties_unwrapped(|_g: &mut InlineFrame| {
                    f.debug_struct("InlineFrame").finish()
                })
            }
            None => f.debug_struct("InlineFrame").finish_non_exhaustive(),
        }
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn handle_mount(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, ctx: &Rc<RuntimeContext>) {
        let instance_node = Rc::clone(&*borrow!(ctx.userland_frame_instance_node));
        let children_with_envs = vec![(instance_node, ctx.globals().stack_frame())];
        let new_children =
            expanded_node.generate_children(children_with_envs, ctx, &expanded_node.parent_frame);
        *borrow_mut!(ctx.userland_root_expanded_node) = Some(Rc::clone(&new_children[0].clone()));
        expanded_node.children.set(new_children);
    }
}
