use std::collections::{HashMap, HashSet};
use std::iter;
use std::rc::Rc;
use_RefCell!();

use pax_language::Computable;
use pax_manifest::ExpressionInfo;
use pax_runtime_api::CoercionRules;
use pax_runtime_api::{
    borrow, borrow_mut, use_RefCell, ImplToFromPaxAny, PaxValue, Property, ToPaxValue, Variable,
};

use crate::api::Layer;
use crate::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
    RuntimePropertiesStackFrame,
};

/// A special "control-flow" primitive associated with the `for` statement.
/// Repeat allows for nodes to be rendered dynamically per data specified in `source_expression`.
/// That is: for a `source_expression` of length `n`, `Repeat` will render its
/// template `n` times, each with an embedded component context (`RepeatItem`)
/// with an index `i` and a pointer to that relevant datum `source_expression[i]`
pub struct RepeatInstance {
    pub base: BaseInstance,
}

impl ImplToFromPaxAny for RepeatProperties {}
///Contains modal _vec_ and _range_ variants, describing whether the Repeat source
///is encoded as a `Vec<T>` (where T is a `PaxValue` properties type) or as a `Range<isize>`
#[derive(Default)]
pub struct RepeatProperties {
    pub source_expression: Property<PaxValue>,
    pub iterator_i_symbol: Property<Option<String>>,
    pub iterator_elem_symbol: Property<Option<String>>,
    pub repeat_key_expression: Option<ExpressionInfo>,
}

impl ToPaxValue for RepeatProperties {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(
            vec![
                (
                    "source_expression".to_string(),
                    self.source_expression.to_pax_value(),
                ),
                (
                    "iterator_i_symbol".to_string(),
                    self.iterator_i_symbol.to_pax_value(),
                ),
                (
                    "iterator_elem_symbol".to_string(),
                    self.iterator_elem_symbol.to_pax_value(),
                ),
            ]
            .into_iter()
            .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::math::Transform2;
    use crate::api::CommonProperties;
    use crate::{ComponentInstance, ExpandedNode, Globals, RouteLocation, TransformAndBounds};
    use pax_language::interpreter::property_resolution::IdentifierResolver;
    use pax_language::parse_pax_expression;
    use pax_manifest::cartridge_generation::{ComponentTransitionConfig, TRANSITION_PHASE_EXIT};
    use pax_runtime_api::pax_value::{PaxAny, ToFromPaxAny};
    use pax_runtime_api::{borrow, CoercionRules, Numeric, Platform, Size, OS};
    use std::cell::RefCell;

    fn test_globals() -> Globals {
        Globals {
            frames_elapsed: Property::new(0),
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
                .then(|| Rc::new(RefCell::new(PaxAny::Builtin(PaxValue::default()))))
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

    fn leaf_args(transition_config: ComponentTransitionConfig) -> InstantiationArgs {
        InstantiationArgs {
            prototypical_common_properties: crate::CommonPropertiesInit::Factory(
                default_common_properties_factory(),
            ),
            prototypical_properties: crate::PropertiesInit::Factory(default_properties_factory()),
            handler_registry: None,
            children: None,
            component_template: Some(RefCell::new(Vec::new())),
            component_settings: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config,
            properties_scope: crate::PropertiesScopeInit::None,
        }
    }

    fn repeat_args(
        source: Property<PaxValue>,
        children: Vec<Rc<dyn InstanceNode>>,
    ) -> InstantiationArgs {
        repeat_args_with_key(source, children, true)
    }

    fn unkeyed_repeat_args(
        source: Property<PaxValue>,
        children: Vec<Rc<dyn InstanceNode>>,
    ) -> InstantiationArgs {
        repeat_args_with_key(source, children, false)
    }

    fn repeat_args_with_key(
        source: Property<PaxValue>,
        children: Vec<Rc<dyn InstanceNode>>,
        use_key: bool,
    ) -> InstantiationArgs {
        let key_expression = ExpressionInfo::new(parse_pax_expression("item.id").unwrap());
        let source_for_factory = source.clone();
        InstantiationArgs {
            prototypical_common_properties: crate::CommonPropertiesInit::Factory(
                default_common_properties_factory(),
            ),
            prototypical_properties: crate::PropertiesInit::Factory(Box::new(
                move |_, expanded_node| {
                    expanded_node.is_none().then(|| {
                        Rc::new(RefCell::new(
                            RepeatProperties {
                                source_expression: source_for_factory.clone(),
                                iterator_i_symbol: Property::new(Some("i".to_string())),
                                iterator_elem_symbol: Property::new(Some("item".to_string())),
                                repeat_key_expression: use_key.then(|| key_expression.clone()),
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

    fn positioned_leaf_args() -> InstantiationArgs {
        InstantiationArgs {
            prototypical_common_properties: crate::CommonPropertiesInit::Factory(Box::new(
                |env, expanded_node| {
                    if expanded_node.is_some() {
                        return None;
                    }
                    let i_untyped = env.resolve_symbol_as_erased_property("i").unwrap();
                    let i = Property::<usize>::new_from_untyped(i_untyped.clone());
                    let deps = [i_untyped];

                    let mut cp = CommonProperties::default();
                    cp.x = Property::computed(
                        move || Some(Size::Pixels((i.get() as f64 * 10.0).into())),
                        &deps,
                    );
                    cp.width = Property::new(Some(Size::Pixels(10.into())));
                    cp.height = Property::new(Some(Size::Pixels(10.into())));
                    Some(Rc::new(RefCell::new(cp)))
                },
            )),
            prototypical_properties: crate::PropertiesInit::Factory(default_properties_factory()),
            handler_registry: None,
            children: None,
            component_template: Some(RefCell::new(Vec::new())),
            component_settings: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: crate::PropertiesScopeInit::None,
        }
    }

    fn transitioned_positioned_leaf_args() -> InstantiationArgs {
        let mut args = positioned_leaf_args();
        args.transition_config = ComponentTransitionConfig {
            has_enter: true,
            enter_frame_count: 10,
            has_exit: true,
            exit_frame_count: 10,
            timeout_ms: 5_000,
            ..Default::default()
        };
        args
    }

    fn expression_positioned_leaf_args(expr: &str) -> InstantiationArgs {
        let expression = ExpressionInfo::new(parse_pax_expression(expr).unwrap());
        InstantiationArgs {
            prototypical_common_properties: crate::CommonPropertiesInit::Factory(Box::new(
                move |env, expanded_node| {
                    if expanded_node.is_some() {
                        return None;
                    }
                    let mut deps = Vec::new();
                    for dependency in &expression.dependencies {
                        let property = env
                            .resolve_symbol_as_erased_property(dependency)
                            .unwrap_or_else(|| panic!("missing dependency: {dependency}"));
                        deps.push(property);
                    }

                    let expression_for_eval = expression.clone();
                    let env_for_eval = Rc::clone(&env);
                    let mut cp = CommonProperties::default();
                    cp.x = Property::computed(
                        move || {
                            let env_for_eval: Rc<dyn IdentifierResolver> = env_for_eval.clone();
                            expression_for_eval
                                .expression
                                .compute(env_for_eval)
                                .ok()
                                .and_then(|value| Size::try_coerce(value).ok())
                        },
                        &deps,
                    );
                    cp.width = Property::new(Some(Size::Pixels(10.into())));
                    cp.height = Property::new(Some(Size::Pixels(10.into())));
                    Some(Rc::new(RefCell::new(cp)))
                },
            )),
            prototypical_properties: crate::PropertiesInit::Factory(default_properties_factory()),
            handler_registry: None,
            children: None,
            component_template: Some(RefCell::new(Vec::new())),
            component_settings: None,
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: Default::default(),
            properties_scope: crate::PropertiesScopeInit::None,
        }
    }

    fn transitioned_expression_positioned_leaf_args(expr: &str) -> InstantiationArgs {
        let mut args = expression_positioned_leaf_args(expr);
        args.transition_config = ComponentTransitionConfig {
            has_enter: true,
            enter_frame_count: 10,
            has_exit: true,
            exit_frame_count: 10,
            timeout_ms: 5_000,
            ..Default::default()
        };
        args
    }

    fn item(id: &str) -> PaxValue {
        item_with_x(id, 0.0)
    }

    fn item_with_x(id: &str, x: f64) -> PaxValue {
        PaxValue::Object(
            vec![
                ("id".to_string(), PaxValue::String(id.to_string())),
                ("x".to_string(), x.to_pax_value()),
            ]
            .into_iter()
            .collect(),
        )
    }

    fn source(ids: &[&str]) -> PaxValue {
        PaxValue::Vec(ids.iter().map(|id| item(id)).collect())
    }

    fn source_with_x(entries: &[(&str, f64)]) -> PaxValue {
        PaxValue::Vec(entries.iter().map(|(id, x)| item_with_x(id, *x)).collect())
    }

    fn ids(nodes: &[Rc<ExpandedNode>]) -> Vec<u32> {
        nodes.iter().map(|node| node.id.0).collect()
    }

    #[test]
    fn keyed_repeat_reuses_reordered_children_and_exits_removed_keys() {
        let source_property = Property::new(source(&["a", "b", "c"]));
        let leaf_transition_config = ComponentTransitionConfig {
            has_exit: true,
            exit_frame_count: 30,
            timeout_ms: 5_000,
            ..Default::default()
        };
        let leaf: Rc<dyn InstanceNode> =
            ComponentInstance::instantiate(leaf_args(leaf_transition_config));
        let repeat: Rc<dyn InstanceNode> =
            RepeatInstance::instantiate(repeat_args(source_property.clone(), vec![leaf]));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![Rc::clone(&repeat)])));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let repeat_node = root.children.get().remove(0);
        let initial = repeat_node.children.get();
        assert_eq!(initial.len(), 3);
        let initial_ids = ids(&initial);

        source_property.set(source(&["c", "a", "b"]));
        root.recurse_update(&context);
        let reordered_active = borrow!(repeat_node.active_children).clone();
        assert_eq!(
            ids(&reordered_active),
            vec![initial_ids[2], initial_ids[0], initial_ids[1]]
        );
        assert!(borrow!(repeat_node.exiting_children).is_empty());

        source_property.set(source(&["c", "d", "a"]));
        root.recurse_update(&context);
        let active = borrow!(repeat_node.active_children).clone();
        let exiting = borrow!(repeat_node.exiting_children).clone();

        assert_eq!(active.len(), 3);
        assert_eq!(exiting.len(), 1);
        assert_eq!(active[0].id.0, initial_ids[2]);
        assert_ne!(active[1].id.0, initial_ids[0]);
        assert_ne!(active[1].id.0, initial_ids[1]);
        assert_ne!(active[1].id.0, initial_ids[2]);
        assert_eq!(active[2].id.0, initial_ids[0]);
        assert_eq!(exiting[0].id.0, initial_ids[1]);
        assert_eq!(exiting[0].transition_phase.get(), TRANSITION_PHASE_EXIT);
    }

    #[test]
    fn keyed_repeat_assigns_distinct_scope_bindings_per_child() {
        let source_property = Property::new(source(&["a", "b", "c"]));
        let leaf: Rc<dyn InstanceNode> =
            ComponentInstance::instantiate(leaf_args(Default::default()));
        let repeat: Rc<dyn InstanceNode> =
            RepeatInstance::instantiate(repeat_args(source_property, vec![leaf]));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![Rc::clone(&repeat)])));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let repeat_node = root.children.get().remove(0);
        let children = repeat_node.children.get();
        assert_eq!(children.len(), 3);

        let indices: Vec<_> = children
            .iter()
            .map(|child| {
                child
                    .stack
                    .resolve_symbol("i")
                    .and_then(|variable| Numeric::try_coerce(variable.get_as_pax_value()).ok())
                    .map(|value| value.to_int())
            })
            .collect();
        assert_eq!(indices, vec![Some(0), Some(1), Some(2)]);
    }

    #[test]
    fn keyed_repeat_assigns_distinct_element_bindings_per_child() {
        let source_property = Property::new(source_with_x(&[("a", 0.0), ("b", 10.0), ("c", 20.0)]));
        let leaf: Rc<dyn InstanceNode> =
            ComponentInstance::instantiate(leaf_args(Default::default()));
        let repeat: Rc<dyn InstanceNode> =
            RepeatInstance::instantiate(repeat_args(source_property, vec![leaf]));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![Rc::clone(&repeat)])));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let repeat_node = root.children.get().remove(0);
        let children = repeat_node.children.get();
        assert_eq!(children.len(), 3);

        let xs: Vec<_> = children
            .iter()
            .map(|child| {
                child
                    .stack
                    .resolve_symbol("item")
                    .map(|variable| variable.get_as_pax_value())
                    .and_then(|value| match value {
                        PaxValue::Object(fields) => fields
                            .into_iter()
                            .find_map(|(name, value)| (name == "x").then_some(value)),
                        _ => None,
                    })
                    .and_then(|value| Numeric::try_coerce(value).ok())
                    .map(|value| value.to_float())
            })
            .collect();

        assert_eq!(xs, vec![Some(0.0), Some(10.0), Some(20.0)]);
    }

    #[test]
    fn keyed_repeat_resolves_distinct_common_properties_per_child() {
        let source_property = Property::new(source(&["a", "b", "c"]));
        let leaf: Rc<dyn InstanceNode> = ComponentInstance::instantiate(positioned_leaf_args());
        let repeat: Rc<dyn InstanceNode> =
            RepeatInstance::instantiate(repeat_args(source_property, vec![leaf]));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![Rc::clone(&repeat)])));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let repeat_node = root.children.get().remove(0);
        let children = repeat_node.children.get();
        assert_eq!(children.len(), 3);

        let xs: Vec<_> = children
            .iter()
            .map(|child| {
                borrow!(child.get_common_properties())
                    .x
                    .get()
                    .map(|size| size.expect_pixels().to_float())
            })
            .collect();

        assert_eq!(xs, vec![Some(0.0), Some(10.0), Some(20.0)]);
    }

    #[test]
    fn keyed_repeat_resolves_distinct_expression_common_properties_from_element_binding() {
        let source_property = Property::new(source_with_x(&[("a", 0.0), ("b", 10.0), ("c", 20.0)]));
        let leaf: Rc<dyn InstanceNode> =
            ComponentInstance::instantiate(expression_positioned_leaf_args("(item.x)px"));
        let repeat: Rc<dyn InstanceNode> =
            RepeatInstance::instantiate(repeat_args(source_property, vec![leaf]));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![Rc::clone(&repeat)])));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let repeat_node = root.children.get().remove(0);
        let children = repeat_node.children.get();
        assert_eq!(children.len(), 3);

        let xs: Vec<_> = children
            .iter()
            .map(|child| {
                borrow!(child.get_common_properties())
                    .x
                    .get()
                    .map(|size| size.expect_pixels().to_float())
            })
            .collect();

        assert_eq!(xs, vec![Some(0.0), Some(10.0), Some(20.0)]);
    }

    #[test]
    fn keyed_repeat_resolves_distinct_common_properties_for_transition_components() {
        let source_property = Property::new(source(&["a", "b", "c"]));
        let leaf: Rc<dyn InstanceNode> =
            ComponentInstance::instantiate(transitioned_positioned_leaf_args());
        let repeat: Rc<dyn InstanceNode> =
            RepeatInstance::instantiate(repeat_args(source_property, vec![leaf]));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![Rc::clone(&repeat)])));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let repeat_node = root.children.get().remove(0);
        let children = repeat_node.children.get();
        assert_eq!(children.len(), 3);

        let xs: Vec<_> = children
            .iter()
            .map(|child| {
                borrow!(child.get_common_properties())
                    .x
                    .get()
                    .map(|size| size.expect_pixels().to_float())
            })
            .collect();

        assert_eq!(xs, vec![Some(0.0), Some(10.0), Some(20.0)]);
    }

    #[test]
    fn keyed_repeat_resolves_expression_common_properties_for_transition_components() {
        let source_property = Property::new(source_with_x(&[("a", 0.0), ("b", 10.0), ("c", 20.0)]));
        let leaf: Rc<dyn InstanceNode> = ComponentInstance::instantiate(
            transitioned_expression_positioned_leaf_args("(item.x)px"),
        );
        let repeat: Rc<dyn InstanceNode> =
            RepeatInstance::instantiate(repeat_args(source_property, vec![leaf]));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![Rc::clone(&repeat)])));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let repeat_node = root.children.get().remove(0);
        let children = repeat_node.children.get();
        assert_eq!(children.len(), 3);

        let xs: Vec<_> = children
            .iter()
            .map(|child| {
                borrow!(child.get_common_properties())
                    .x
                    .get()
                    .map(|size| size.expect_pixels().to_float())
            })
            .collect();

        assert_eq!(xs, vec![Some(0.0), Some(10.0), Some(20.0)]);
    }

    #[test]
    fn unkeyed_repeat_stale_element_binding_defaults_after_source_shrinks() {
        let source_property = Property::new(source_with_x(&[("a", 0.0), ("b", 10.0)]));
        let leaf: Rc<dyn InstanceNode> =
            ComponentInstance::instantiate(leaf_args(Default::default()));
        let repeat: Rc<dyn InstanceNode> =
            RepeatInstance::instantiate(unkeyed_repeat_args(source_property.clone(), vec![leaf]));
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![Rc::clone(&repeat)])));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);

        root.recurse_update(&context);
        let repeat_node = root.children.get().remove(0);
        let children = repeat_node.children.get();
        assert_eq!(children.len(), 2);

        let stale_item = children[1].stack.resolve_symbol("item").unwrap();
        source_property.set(source_with_x(&[("a", 0.0)]));

        assert_eq!(stale_item.get_as_pax_value(), PaxValue::default());
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum RepeatKey {
    String(String),
    Int(i64),
    Synthetic(usize),
}

#[derive(Clone)]
struct RepeatChildGroup {
    key: RepeatKey,
    elem: Property<PaxValue>,
    i: Property<usize>,
    children: Vec<Rc<ExpandedNode>>,
}

/// Per-iteration bindings exposed inside a `for` template body.
pub struct RepeatItem {
    pub elem: Property<PaxValue>,
    pub i: Property<usize>,
}

impl ToPaxValue for RepeatItem {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(
            vec![
                ("elem".to_string(), self.elem.get().to_pax_value()),
                ("i".to_string(), self.i.get().to_pax_value()),
            ]
            .into_iter()
            .collect(),
        )
    }
}

impl InstanceNode for RepeatInstance {
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
        })
    }

    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _expanded_node: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.debug_struct("Repeat").finish()
    }

    fn base(&self) -> &BaseInstance {
        &self.base
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
}

impl RepeatInstance {
    fn handle_setup(
        self: Rc<Self>,
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
        is_mount: bool,
    ) {
        // No-op: wait with creating child-nodes until update tick, since the
        // condition has then been evaluated
        let weak_ref_self = Rc::downgrade(expanded_node);
        let cloned_self = Rc::clone(&self);
        let cloned_context = Rc::clone(context);
        let source_expression =
            expanded_node.with_properties_unwrapped(|properties: &mut RepeatProperties| {
                properties.source_expression.clone()
            });

        let i_symbol =
            expanded_node.with_properties_unwrapped(|properties: &mut RepeatProperties| {
                properties.iterator_i_symbol.clone()
            });
        let elem_symbol =
            expanded_node.with_properties_unwrapped(|properties: &mut RepeatProperties| {
                properties.iterator_elem_symbol.clone()
            });
        let repeat_key_expression =
            expanded_node.with_properties_unwrapped(|properties: &mut RepeatProperties| {
                properties.repeat_key_expression.clone()
            });

        let mut deps = vec![
            source_expression.untyped(),
            i_symbol.untyped(),
            elem_symbol.untyped(),
        ];
        if let Some(repeat_key_expression) = &repeat_key_expression {
            for dependency in &repeat_key_expression.dependencies {
                if let Some(property) = expanded_node
                    .stack
                    .resolve_symbol_as_erased_property(dependency)
                {
                    deps.push(property);
                }
            }
        }

        let last_length = Rc::new(RefCell::new(0));
        let last_elem_sym = Rc::new(RefCell::new(None));
        let last_i_sym = Rc::new(RefCell::new(None));
        let cached_children: Rc<RefCell<Vec<Rc<ExpandedNode>>>> = Default::default();
        let keyed_groups: Rc<RefCell<Vec<RepeatChildGroup>>> = Default::default();

        let children = Property::computed_with_name(
            move || {
                let Some(cloned_expanded_node) = weak_ref_self.upgrade() else {
                    panic!("ran evaluator after expanded node dropped (repeat elem)")
                };
                let source_len = source_expression.read(Self::source_len);
                if let Some(repeat_key_expression) = &repeat_key_expression {
                    let i_symbol_value = i_symbol.get();
                    let elem_symbol_value = elem_symbol.get();
                    let symbols_changed = i_symbol_value != *borrow!(last_i_sym)
                        || elem_symbol_value != *borrow!(last_elem_sym);
                    *borrow_mut!(last_i_sym) = i_symbol_value.clone();
                    *borrow_mut!(last_elem_sym) = elem_symbol_value.clone();
                    if symbols_changed {
                        borrow_mut!(keyed_groups).clear();
                    }
                    return Self::compute_keyed_children(
                        &cloned_expanded_node,
                        &cloned_context,
                        Rc::clone(&cloned_self),
                        &source_expression,
                        i_symbol_value,
                        elem_symbol_value,
                        repeat_key_expression,
                        Rc::clone(&keyed_groups),
                        source_len,
                        is_mount,
                    );
                }

                if source_len == *borrow!(last_length)
                    && i_symbol.read(|i| i == &*borrow!(last_i_sym))
                    && elem_symbol.read(|e| e == &*borrow!(last_elem_sym))
                {
                    return if cloned_expanded_node.attached.get() > 0 {
                        cloned_expanded_node.current_attached_children()
                    } else {
                        borrow!(cached_children).clone()
                    };
                }
                *borrow_mut!(last_length) = source_len;
                *borrow_mut!(last_i_sym) = i_symbol.get();
                *borrow_mut!(last_elem_sym) = elem_symbol.get();

                let template_children = cloned_self.base().get_instance_children();
                let children_with_envs = iter::repeat(template_children)
                    .take(source_len)
                    .enumerate()
                    .flat_map(|(i, children)| {
                        let property_i = Property::new(i);
                        let cp_source_expression = source_expression.clone();
                        let property_elem = Property::computed_with_name(
                            move || {
                                cp_source_expression.read(|source| Self::source_elem(source, i))
                            },
                            &[source_expression.untyped()],
                            "repeat elem",
                        );

                        let scope = Self::repeat_scope(
                            i_symbol.get(),
                            elem_symbol.get(),
                            property_i,
                            property_elem,
                        );

                        let new_env = cloned_expanded_node.stack.push(scope);
                        borrow!(children)
                            .clone()
                            .into_iter()
                            .zip(iter::repeat(new_env))
                    });
                let ret = cloned_expanded_node.generate_children(
                    children_with_envs,
                    &cloned_context,
                    &cloned_expanded_node.parent_frame,
                    is_mount,
                );
                *borrow_mut!(cached_children) = ret.clone();
                ret
            },
            &deps,
            &format!("repeat_children (node id: {})", expanded_node.id.0),
        );
        expanded_node.children.replace_with(children);
    }

    fn source_len(source: &PaxValue) -> usize {
        if let PaxValue::Range(start, end) = source {
            (isize::try_coerce(*end.clone()).unwrap() - isize::try_coerce(*start.clone()).unwrap())
                as usize
        } else if let PaxValue::Vec(v) = source {
            v.len()
        } else {
            log::warn!("source is not a vec");
            0
        }
    }

    fn source_elem(source: &PaxValue, i: usize) -> PaxValue {
        if let PaxValue::Range(start, _) = source {
            let start = isize::try_coerce(*start.clone()).unwrap();
            (start + i as isize).to_pax_value()
        } else if let PaxValue::Vec(v) = source {
            v.get(i).cloned().unwrap_or_default()
        } else {
            log::warn!("source is not a vec");
            Default::default()
        }
    }

    fn repeat_scope(
        i_symbol: Option<String>,
        elem_symbol: Option<String>,
        property_i: Property<usize>,
        property_elem: Property<PaxValue>,
    ) -> HashMap<String, Variable> {
        let mut scope = HashMap::new();
        if let Some(i_symbol) = i_symbol {
            scope.insert(i_symbol, Variable::new_from_typed_property(property_i));
        }
        if let Some(elem_symbol) = elem_symbol {
            scope.insert(
                elem_symbol,
                Variable::new_from_typed_property(property_elem),
            );
        }
        scope
    }

    fn repeat_key_from_value(value: PaxValue, index: usize) -> Option<RepeatKey> {
        match value {
            PaxValue::String(value) => Some(RepeatKey::String(value)),
            PaxValue::Numeric(value) if !value.is_float() => Some(RepeatKey::Int(value.to_int())),
            _ => {
                log::warn!(
                    "repeat key for index {} must evaluate to a string or integer; using positional fallback",
                    index
                );
                None
            }
        }
    }

    fn evaluate_key(
        key_expression: &ExpressionInfo,
        env: Rc<RuntimePropertiesStackFrame>,
        index: usize,
    ) -> Option<RepeatKey> {
        key_expression
            .expression
            .compute(env)
            .map(|value| Self::repeat_key_from_value(value, index))
            .unwrap_or_else(|err| {
                log::warn!(
                    "failed to compute repeat key for index {}: {}; using positional fallback",
                    index,
                    err
                );
                None
            })
    }

    fn compute_keyed_children(
        expanded_node: &Rc<ExpandedNode>,
        context: &Rc<RuntimeContext>,
        repeat: Rc<RepeatInstance>,
        source_expression: &Property<PaxValue>,
        i_symbol_value: Option<String>,
        elem_symbol_value: Option<String>,
        key_expression: &ExpressionInfo,
        keyed_groups: Rc<RefCell<Vec<RepeatChildGroup>>>,
        source_len: usize,
        is_mount: bool,
    ) -> Vec<Rc<ExpandedNode>> {
        let mut old_groups_by_key: HashMap<RepeatKey, RepeatChildGroup> = borrow_mut!(keyed_groups)
            .drain(..)
            .map(|group| (group.key.clone(), group))
            .collect();
        let mut new_groups = Vec::new();
        let mut new_children = Vec::new();
        let mut used_keys = HashSet::new();
        let template_children = repeat.base().get_instance_children();

        for i in 0..source_len {
            let elem = source_expression.read(|source| Self::source_elem(source, i));
            let key_probe_i = Property::new(i);
            let key_probe_elem = Property::new(elem.clone());
            let key_probe_scope = Self::repeat_scope(
                i_symbol_value.clone(),
                elem_symbol_value.clone(),
                key_probe_i,
                key_probe_elem,
            );
            let key_probe_env = expanded_node.stack.push(key_probe_scope);
            let evaluated_key = Self::evaluate_key(key_expression, key_probe_env, i);
            let key = match evaluated_key {
                Some(key) if used_keys.insert(key.clone()) => key,
                Some(key) => {
                    log::warn!(
                        "duplicate repeat key {:?} at index {}; using positional fallback",
                        key,
                        i
                    );
                    RepeatKey::Synthetic(i)
                }
                None => RepeatKey::Synthetic(i),
            };

            let group = if let Some(group) = old_groups_by_key.remove(&key) {
                group.i.set(i);
                group.elem.set(elem);
                group
            } else {
                let property_i = Property::new(i);
                let property_elem = Property::new(elem);
                let scope = Self::repeat_scope(
                    i_symbol_value.clone(),
                    elem_symbol_value.clone(),
                    property_i.clone(),
                    property_elem.clone(),
                );
                let new_env = expanded_node.stack.push(scope);
                let children_with_env = borrow!(template_children)
                    .clone()
                    .into_iter()
                    .zip(iter::repeat(new_env));
                let children = expanded_node.create_children_detached(
                    children_with_env,
                    context,
                    &Rc::downgrade(expanded_node),
                );
                if !is_mount {
                    for child in &children {
                        child.recurse_control_flow_expansion(context);
                    }
                }
                RepeatChildGroup {
                    key: key.clone(),
                    elem: property_elem,
                    i: property_i,
                    children,
                }
            };

            new_children.extend(group.children.iter().cloned());
            new_groups.push(group);
        }

        *borrow_mut!(keyed_groups) = new_groups;
        if is_mount {
            expanded_node.attach_children(new_children, context, &expanded_node.parent_frame)
        } else {
            new_children
        }
    }
}
