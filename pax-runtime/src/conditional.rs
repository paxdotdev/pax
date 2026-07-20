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
        let cached_children = RefCell::new(Vec::new());
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
                        return if cloned_expanded_node.attached.get() > 0 {
                            cloned_expanded_node.current_attached_children()
                        } else {
                            borrow!(cached_children).clone()
                        };
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
                    let mut selected_children = Vec::new();
                    for template in children[start..end].iter() {
                        let rescued = (is_mount && cloned_expanded_node.attached.get() > 0)
                            .then(|| {
                                cloned_expanded_node.rescue_exiting_child_matching(|child| {
                                    let child_template = borrow!(child.instance_node).clone();
                                    Rc::ptr_eq(&child_template, template)
                                })
                            })
                            .flatten();
                        let child = rescued.unwrap_or_else(|| {
                            cloned_expanded_node
                                .create_children_detached(
                                    iter::once((Rc::clone(template), Rc::clone(&env))),
                                    &cloned_context,
                                    &Rc::downgrade(&cloned_expanded_node),
                                )
                                .pop()
                                .expect("conditional child creation returned no child")
                        });
                        if !is_mount && child.attached.get() == 0 {
                            child.recurse_control_flow_expansion(&cloned_context);
                        }
                        selected_children.push(child);
                    }
                    let ret = if is_mount {
                        cloned_expanded_node.attach_children(
                            selected_children,
                            &cloned_context,
                            &cloned_expanded_node.parent_frame,
                        )
                    } else {
                        selected_children
                    };
                    *borrow_mut!(cached_children) = ret.clone();
                    ret
                },
                &deps,
                &format!("conditional_children (node id: {})", expanded_node.id.0),
            ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::math::Transform2;
    use crate::api::CommonProperties;
    use crate::{
        BaseInstance, ComponentInstance, Globals, InstanceFlags, RouteLocation,
        RuntimePropertiesStackFrame, TransformAndBounds,
    };
    use pax_manifest::cartridge_generation::{
        ComponentTransitionConfig, TRANSITION_PHASE_ENTER, TRANSITION_PHASE_EXIT,
    };
    use pax_runtime_api::pax_value::{PaxAny, ToFromPaxAny};
    use pax_runtime_api::{Platform, Property, TargetInfo, OS};
    use std::cell::RefCell;

    fn test_globals() -> Globals {
        Globals {
            elapsed_frames: Property::new(0),
            elapsed_millis: Property::new(0),
            viewport: Property::new(TransformAndBounds {
                transform: Transform2::identity(),
                bounds: (100.0, 100.0),
            }),
            gyro: Property::new(Default::default()),
            accel: Property::new(Default::default()),
            route_location: Property::new(RouteLocation::root()),
            browser_allows_scroller_vector_layers: Property::new(true),
            browser_allows_nested_scroller_vector_layers: Property::new(true),
            platform: Platform::Unknown,
            os: OS::Unknown,
            target: TargetInfo::new(Platform::Unknown, OS::Unknown),
            get_elapsed_millis: Rc::new(|| 0),
        }
    }

    fn default_properties_factory() -> Box<
        dyn Fn(
            Rc<RuntimePropertiesStackFrame>,
            Option<Rc<ExpandedNode>>,
        ) -> Option<Rc<RefCell<PaxAny>>>,
    > {
        Box::new(|_, expanded_node| {
            expanded_node
                .is_none()
                .then(|| Rc::new(RefCell::new(PaxAny::Builtin(Default::default()))))
        })
    }

    fn default_common_properties_factory() -> Box<
        dyn Fn(
            Rc<RuntimePropertiesStackFrame>,
            Option<Rc<ExpandedNode>>,
        ) -> Option<Rc<RefCell<CommonProperties>>>,
    > {
        Box::new(|_, expanded_node| {
            expanded_node
                .is_none()
                .then(|| Rc::new(RefCell::new(CommonProperties::default())))
        })
    }

    fn component_args(template: Option<Vec<Rc<dyn InstanceNode>>>) -> InstantiationArgs {
        InstantiationArgs {
            prototypical_common_properties: crate::CommonPropertiesInit::Factory(
                default_common_properties_factory(),
            ),
            prototypical_properties: crate::PropertiesInit::Factory(default_properties_factory()),
            handler_registry: None,
            children: None,
            component_template: template.map(RefCell::new),
            component_settings: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: crate::PropertiesScopeInit::None,
        }
    }

    fn transition_leaf() -> Rc<dyn InstanceNode> {
        let mut args = component_args(Some(Vec::new()));
        args.transition_config = ComponentTransitionConfig {
            has_enter: true,
            enter_frame_count: 10,
            has_exit: true,
            exit_frame_count: 10,
            timeout_ms: 5_000,
            ..Default::default()
        };
        ComponentInstance::instantiate(args)
    }

    fn mounted_conditional(
        condition: Property<bool>,
    ) -> (Rc<ExpandedNode>, Rc<ExpandedNode>, Rc<RuntimeContext>) {
        let conditional: Rc<dyn InstanceNode> =
            ConditionalInstance::instantiate(conditional_args(condition, vec![transition_leaf()]));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![conditional])));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);
        root.recurse_update(&context);
        let conditional_node = root.children.get().remove(0);
        (root, conditional_node, context)
    }

    fn conditional_args(
        condition: Property<bool>,
        children: Vec<Rc<dyn InstanceNode>>,
    ) -> InstantiationArgs {
        InstantiationArgs {
            prototypical_common_properties: crate::CommonPropertiesInit::Factory(
                default_common_properties_factory(),
            ),
            prototypical_properties: crate::PropertiesInit::Factory(Box::new(
                move |_, expanded_node| {
                    expanded_node.is_none().then(|| {
                        Rc::new(RefCell::new(
                            ConditionalProperties {
                                boolean_expression: condition.clone(),
                                conditional_branches: Vec::new(),
                            }
                            .to_pax_any(),
                        ))
                    })
                },
            )),
            handler_registry: None,
            children: Some(RefCell::new(children)),
            component_template: None,
            component_settings: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: crate::PropertiesScopeInit::None,
        }
    }

    struct TestLeaf {
        base: BaseInstance,
    }

    impl InstanceNode for TestLeaf {
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
            _expanded_node: Option<&ExpandedNode>,
        ) -> std::fmt::Result {
            f.debug_struct("TestLeaf").finish()
        }

        fn base(&self) -> &BaseInstance {
            &self.base
        }
    }

    #[test]
    fn detached_conditional_keeps_cached_children_on_same_branch_recompute() {
        let condition = Property::new(true);
        let leaf: Rc<dyn InstanceNode> = TestLeaf::instantiate(component_args(Some(Vec::new())));
        let conditional =
            ConditionalInstance::instantiate(conditional_args(condition.clone(), vec![leaf]));
        let root_component = ComponentInstance::instantiate(component_args(None));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        let detached = root
            .create_children_detached(
                vec![(
                    conditional.clone() as Rc<dyn InstanceNode>,
                    Rc::clone(&root.stack),
                )],
                &context,
                &Rc::downgrade(&root),
            )
            .remove(0);

        conditional.clone().handle_setup(&detached, &context, false);

        let initial = detached.children.get();
        assert_eq!(initial.len(), 1);
        let initial_id = initial[0].id;

        condition.set(true);
        let recomputed = detached.children.get();
        assert_eq!(recomputed.len(), 1);
        assert_eq!(recomputed[0].id, initial_id);
    }

    #[test]
    fn conditional_prunes_completed_exit_and_can_reenter_during_exit() {
        let condition = Property::new(true);
        let (root, conditional_node, context) = mounted_conditional(condition.clone());
        let initial_id = borrow!(conditional_node.active_children)[0].id;

        condition.set(false);
        root.recurse_update(&context);
        assert!(borrow!(conditional_node.active_children).is_empty());
        assert_eq!(borrow!(conditional_node.exiting_children).len(), 1);
        assert_eq!(
            borrow!(conditional_node.exiting_children)[0]
                .transition_phase
                .get(),
            TRANSITION_PHASE_EXIT
        );

        context.globals().elapsed_frames.set(5);
        condition.set(true);
        root.recurse_update(&context);

        let active = borrow!(conditional_node.active_children).clone();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, initial_id);
        assert_eq!(active[0].transition_phase.get(), TRANSITION_PHASE_ENTER);
        assert!(borrow!(conditional_node.exiting_children).is_empty());

        context.globals().elapsed_frames.set(10);
        context.drain_node_effects();
        assert!(borrow!(conditional_node.exiting_children).is_empty());
        assert_eq!(conditional_node.children.get().len(), 1);
    }

    #[test]
    fn conditional_survives_repeated_rapid_toggles_during_transitions() {
        let condition = Property::new(true);
        let (root, conditional_node, context) = mounted_conditional(condition.clone());
        let stable_id = borrow!(conditional_node.active_children)[0].id;

        for frame in 1..=6 {
            context.globals().elapsed_frames.set(frame);
            condition.set(frame % 2 == 0);
            root.recurse_update(&context);

            let expected_active = usize::from(frame % 2 == 0);
            assert_eq!(
                borrow!(conditional_node.active_children).len(),
                expected_active
            );
            assert_eq!(borrow!(conditional_node.mounted_children).len(), 1);
            assert_eq!(borrow!(conditional_node.mounted_children)[0].id, stable_id);
        }

        context.globals().elapsed_frames.set(20);
        context.drain_node_effects();
        assert_eq!(borrow!(conditional_node.active_children).len(), 1);
        assert!(borrow!(conditional_node.exiting_children).is_empty());
    }

    #[test]
    fn conditional_refreshes_computed_children_after_exit_cleanup() {
        let condition = Property::new(true);
        let (_root, conditional_node, context) = mounted_conditional(condition.clone());

        condition.set(false);
        let _ = conditional_node.children.get();
        context.globals().elapsed_frames.set(10);
        context.drain_node_effects();

        assert!(borrow!(conditional_node.active_children).is_empty());
        assert!(borrow!(conditional_node.exiting_children).is_empty());
        assert!(conditional_node.children.get().is_empty());
    }
}
