use std::{iter, rc::Rc};

use crate::{
    declarative_macros::handle_vtable_update, BaseInstance, ExpandedNode, InstanceFlags,
    InstanceNode, InstantiationArgs, RuntimeContext,
};
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
    last_boolean_expression: bool,
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
                    is_component: false,
                },
            ),
        })
    }

    fn update(self: Rc<Self>, expanded_node: &Rc<ExpandedNode>, context: &mut RuntimeContext) {
        let (should_update, active) =
            expanded_node.with_properties_unwrapped(|properties: &mut ConditionalProperties| {
                handle_vtable_update(
                    context.expression_table(),
                    &expanded_node.stack,
                    &mut properties.boolean_expression,
                );
                let val = *properties.boolean_expression.get();
                let update_children = properties.last_boolean_expression != val;
                properties.last_boolean_expression = val;
                (update_children, *properties.boolean_expression.get())
            });

        if should_update {
            if active {
                let env = Rc::clone(&expanded_node.stack);
                let children_with_envs = self
                    .base()
                    .get_template_children()
                    .iter()
                    .cloned()
                    .zip(iter::repeat(env));
                expanded_node.set_children(children_with_envs, context);
            } else {
                expanded_node.set_children(iter::empty(), context);
            }
        }
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

    fn get_size(
        &self,
        expanded_node: &ExpandedNode,
    ) -> (pax_runtime_api::Size, pax_runtime_api::Size) {
        let common_properties = expanded_node.get_common_properties();
        let common_properties_borrowed = common_properties.borrow();
        (
            common_properties_borrowed.width.get().clone(),
            common_properties_borrowed.height.get().clone(),
        )
    }

    fn get_clipping_size(
        &self,
        _expanded_node: &ExpandedNode,
    ) -> Option<(pax_runtime_api::Size, pax_runtime_api::Size)> {
        None
    }

    fn handle_form_event(&self, event: crate::form_event::FormEvent) {
        panic!("form event sent to non-compatible component: {:?}", event)
    }
}
