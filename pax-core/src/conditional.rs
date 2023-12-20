use std::rc::Rc;

use crate::{
    declarative_macros::handle_vtable_update, BaseInstance, ExpandedNode, InstanceFlags,
    InstanceNode, InstantiationArgs, PropertiesTreeContext, UpdateContext,
};
use pax_message::NativeMessage;
use pax_runtime_api::Layer;

/// A special "control-flow" primitive, Conditional (`if`) allows for a
/// subtree of a component template to be rendered conditionally,
/// based on the value of the property `boolean_expression`.
/// The Pax compiler handles ConditionalInstance specially
/// with the `if` syntax in templates.
pub struct ConditionalInstance {
    base: BaseInstance,
}

///Contains the expression of a conditional, evaluated as an expression.
#[derive(Default)]
pub struct ConditionalProperties {
    pub boolean_expression: Box<dyn pax_runtime_api::PropertyInstance<bool>>,
}

impl InstanceNode for ConditionalInstance {
    fn instantiate(args: InstantiationArgs) -> Rc<Self>
    where
        Self: Sized,
    {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: true,
                    invisible_to_raycasting: true,
                    layer: Layer::DontCare,
                },
            ),
        })
    }

    fn expand(self: Rc<Self>, ptc: &mut PropertiesTreeContext) -> Rc<ExpandedNode> {
        let this_expanded_node = self
            .base()
            .expand_from_instance(Rc::clone(&self) as Rc<dyn InstanceNode>, ptc);

        let id_chain = ptc.get_id_chain(self.base().get_instance_id());

        for child in self.base().get_children() {
            let mut new_ptc = ptc.clone();
            let expanded_child = Rc::clone(child).expand(&mut new_ptc);
            this_expanded_node.append_child(expanded_child)
        }
        this_expanded_node
    }

    fn update(
        &self,
        expanded_node: &Rc<ExpandedNode>,
        context: &UpdateContext,
        messages: &mut Vec<NativeMessage>,
    ) {
        expanded_node.with_properties_unwrapped(|properties: &mut ConditionalProperties| {
            handle_vtable_update(
                context.expression_table,
                expanded_node,
                &mut properties.boolean_expression,
            );
            //return evaluated value
            *properties.boolean_expression.get()
        });
    }

    #[cfg(debug_assertions)]
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Conditional").finish()
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}
