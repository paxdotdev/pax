use std::{iter, rc::Rc};
use_RefCell!();

use pax_runtime_api::pax_value::ImplToFromPaxAny;
use pax_runtime_api::{borrow, borrow_mut, use_RefCell, PaxValue, Property, ToPaxValue};

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

impl ImplToFromPaxAny for ConditionalProperties {}

///Contains the expression of a conditional, evaluated as an expression.
#[derive(Default)]
pub struct ConditionalProperties {
    pub boolean_expression: Property<bool>,
}

impl ToPaxValue for ConditionalProperties {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(
            vec![(
                "boolean_expression".to_string(),
                self.boolean_expression.to_pax_value(),
            )]
            .into_iter()
            .collect(),
        )
    }
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
        context: &Rc<RuntimeContext>,
    ) {
        let weak_ref_self = Rc::downgrade(expanded_node);
        let cloned_self = Rc::clone(&self);
        let cloned_context = Rc::clone(context);

        let cond_expr =
            expanded_node.with_properties_unwrapped(|properties: &mut ConditionalProperties| {
                properties.boolean_expression.clone()
            });

        let dep = cond_expr.untyped();

        let old_val = RefCell::new(false);
        expanded_node
            .children
            .replace_with(Property::computed_with_name(
                move || {
                    let Some(cloned_expanded_node) = weak_ref_self.upgrade() else {
                        panic!("ran evaluator after expanded node dropped (conditional elem)")
                    };
                    let val = cond_expr.get();
                    if val == *borrow!(old_val) {
                        return cloned_expanded_node.children.get();
                    }
                    *borrow_mut!(old_val) = val;
                    if val {
                        let env = Rc::clone(&cloned_expanded_node.stack);
                        let children = borrow!(cloned_self.base().get_instance_children());
                        let children_with_envs = children.iter().cloned().zip(iter::repeat(env));
                        let res = cloned_expanded_node.generate_children(
                            children_with_envs,
                            &cloned_context,
                            &cloned_expanded_node.parent_frame,
                        );
                        res
                    } else {
                        cloned_expanded_node.generate_children(
                            vec![],
                            &cloned_context,
                            &cloned_expanded_node.parent_frame,
                        )
                    }
                },
                &[dep],
                &format!("conditional_children (node id: {})", expanded_node.id.0),
            ));
    }

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
