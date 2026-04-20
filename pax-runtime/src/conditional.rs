use std::{iter, ops::Range, rc::Rc};
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
    branch_child_ranges: Vec<Range<usize>>,
}

impl ImplToFromPaxAny for ConditionalProperties {}

///Contains the expression of a conditional, evaluated as an expression.
#[derive(Default)]
pub struct ConditionalProperties {
    pub boolean_expression: Property<bool>,
    pub conditional_branches: Vec<Property<bool>>,
}

impl ToPaxValue for ConditionalProperties {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(
            vec![
                (
                    "boolean_expression".to_string(),
                    self.boolean_expression.to_pax_value(),
                ),
                (
                    "conditional_branches".to_string(),
                    self.conditional_branches.to_pax_value(),
                ),
            ]
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
                    is_slot: false,
                },
            ),
            branch_child_ranges: vec![],
        })
    }

    fn handle_mount(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        self.handle_setup(expanded_node, context, true);
    }

    fn handle_control_flow_node_expansion(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
    ) {
        self.handle_setup(expanded_node, context, false);
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

impl ConditionalInstance {
    pub fn instantiate_with_branch_child_ranges(
        args: InstantiationArgs,
        branch_child_ranges: Vec<Range<usize>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    invisible_to_slot: true,
                    invisible_to_raycasting: true,
                    layer: Layer::DontCare,
                    is_component: false,
                    is_slot: false,
                },
            ),
            branch_child_ranges,
        })
    }

    fn handle_setup(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
        is_mount: bool,
    ) {
        let weak_ref_self = Rc::downgrade(expanded_node);
        let cloned_self = Rc::clone(&self);
        let cloned_context = Rc::clone(context);

        let boolean_expression =
            expanded_node.with_properties_unwrapped(|properties: &mut ConditionalProperties| {
                properties.boolean_expression.clone()
            });
        let conditional_branches =
            expanded_node.with_properties_unwrapped(|properties: &mut ConditionalProperties| {
                properties.conditional_branches.clone()
            });
        let branch_conditions = if conditional_branches.is_empty() {
            vec![boolean_expression]
        } else {
            conditional_branches
        };

        let deps = branch_conditions
            .iter()
            .map(|condition| condition.untyped())
            .collect::<Vec<_>>();
        let branch_child_ranges = self.branch_child_ranges.clone();

        let old_active_branch = RefCell::new(None::<Option<usize>>);
        expanded_node
            .children
            .replace_with(Property::computed_with_name(
                move || {
                    let Some(cloned_expanded_node) = weak_ref_self.upgrade() else {
                        panic!("ran evaluator after expanded node dropped (conditional elem)")
                    };
                    let active_branch = branch_conditions
                        .iter()
                        .enumerate()
                        .find_map(|(index, condition)| condition.get().then_some(index));
                    if *borrow!(old_active_branch) == Some(active_branch) {
                        return cloned_expanded_node.current_attached_children();
                    }
                    *borrow_mut!(old_active_branch) = Some(active_branch);

                    let children = borrow!(cloned_self.base().get_instance_children());
                    let effective_branch_ranges = if branch_child_ranges.is_empty() {
                        vec![0..children.len()]
                    } else {
                        branch_child_ranges.clone()
                    };
                    let selected_range = active_branch
                        .and_then(|index| effective_branch_ranges.get(index).cloned())
                        .unwrap_or(0..0);
                    let start = selected_range.start.min(children.len());
                    let end = selected_range.end.min(children.len());
                    let env = Rc::clone(&cloned_expanded_node.stack);
                    let children_with_envs =
                        children[start..end].iter().cloned().zip(iter::repeat(env));
                    let ret = cloned_expanded_node.generate_children(
                        children_with_envs,
                        &cloned_context,
                        &cloned_expanded_node.parent_frame,
                        is_mount,
                    );
                    ret
                },
                &deps,
                &format!("conditional_children (node id: {})", expanded_node.id.0),
            ));
    }
}
