use std::cell::RefCell;
use std::{iter, rc::Rc};

use pax_runtime_api::Property;

use crate::api::Layer;
use crate::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};

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
    pub boolean_expression: Property<bool>,
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

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RefCell<RuntimeContext>>,
    ) {
        let weak_ref_self = Rc::downgrade(expanded_node);
        let cloned_self = Rc::clone(&self);
        let cloned_context = Rc::clone(context);

        let cond_expr =
            expanded_node.with_properties_unwrapped(|properties: &mut ConditionalProperties| {
                properties.boolean_expression.clone()
            });

        let dep = cond_expr.untyped();

        expanded_node
            .children
            .replace_with(Property::computed_with_name(
                move || {
                    let Some(cloned_expanded_node) = weak_ref_self.upgrade() else {
                        panic!("ran evaluator after expanded node dropped (conditional elem)")
                    };
                    if cond_expr.get() {
                        let env = Rc::clone(&cloned_expanded_node.stack);
                        let children = cloned_self.base().get_instance_children().borrow();
                        let children_with_envs = children.iter().cloned().zip(iter::repeat(env));
                        cloned_expanded_node.generate_children(children_with_envs, &cloned_context)
                    } else {
                        cloned_expanded_node.generate_children(iter::empty(), &cloned_context)
                    }
                },
                &[dep],
                &format!("conditional_children (node id: {})", expanded_node.id.0),
            ));
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
