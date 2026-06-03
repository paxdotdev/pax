//! Runtime routing primitives used by declarative `Router` / `Route` control flow.

use std::collections::HashMap;
use std::iter;
use std::ops::Range;
use std::rc::Rc;
use_RefCell!();

use pax_manifest::ControlFlowRouteBranchDefinition;
use pax_runtime_api::pax_value::ImplToFromPaxAny;
use pax_runtime_api::{
    borrow, borrow_mut, use_RefCell, Interpolatable, PaxValue, Property, ToPaxValue, Variable,
};

use crate::api::Layer;
use crate::{
    BaseInstance, ExpandedNode, InstanceFlags, InstanceNode, InstantiationArgs, RuntimeContext,
};

/// Internal stack symbol carrying the router input location for nested scopes.
pub const INTERNAL_ROUTE_LOCATION_SYMBOL: &str = "$route_location";
/// Internal stack symbol carrying the current route match object.
pub const INTERNAL_ROUTE_MATCH_SYMBOL: &str = "$route_match";
const ROUTE_SYMBOL: &str = "route";

/// Structured application location shared across platforms.
///
/// On web targets this is serialized to and from `window.location`, while on
/// non-web targets it remains a platform-agnostic route state model.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RouteLocation {
    /// Decoded path segments, with `/` represented by an empty vector.
    pub path_segments: Vec<String>,
    /// Query-string values keyed by parameter name, preserving repeated keys.
    pub query: HashMap<String, Vec<String>>,
    /// Optional fragment without the leading `#`.
    pub fragment: Option<String>,
}

impl RouteLocation {
    /// Returns the canonical root location.
    pub fn root() -> Self {
        Self::default()
    }

    /// Clones this location while replacing only the path segments.
    pub fn with_path_segments(&self, path_segments: Vec<String>) -> Self {
        Self {
            path_segments,
            query: self.query.clone(),
            fragment: self.fragment.clone(),
        }
    }
}

impl From<pax_message::RouteChangeInterruptArgs> for RouteLocation {
    fn from(value: pax_message::RouteChangeInterruptArgs) -> Self {
        Self {
            path_segments: value.path_segments,
            query: value.query,
            fragment: value.fragment,
        }
    }
}

impl From<&pax_message::RouteChangeInterruptArgs> for RouteLocation {
    fn from(value: &pax_message::RouteChangeInterruptArgs) -> Self {
        Self {
            path_segments: value.path_segments.clone(),
            query: value.query.clone(),
            fragment: value.fragment.clone(),
        }
    }
}

impl Interpolatable for RouteLocation {}

impl ToPaxValue for RouteLocation {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(vec![
            (
                "path_segments".to_string(),
                self.path_segments.to_pax_value(),
            ),
            ("query".to_string(), multimap_to_pax_value(&self.query)),
            ("fragment".to_string(), self.fragment.to_pax_value()),
        ])
    }
}

/// Structured match data exposed to the active route subtree as `route`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RouteMatch {
    /// Location as seen by the current router scope.
    pub location: RouteLocation,
    /// Full global application location.
    pub global_location: RouteLocation,
    /// Named values captured from `:param` segments.
    pub params: HashMap<String, String>,
    /// Number of path segments consumed by the selected branch.
    pub consumed_segments: usize,
    /// Remaining path tail left after the selected branch.
    pub remainder: Vec<String>,
    /// Whether the selected branch consumed the entire scoped path.
    pub is_exact: bool,
}

impl RouteMatch {
    /// Converts the remainder into a `RouteLocation` for nested router input.
    pub fn remainder_location(&self) -> RouteLocation {
        self.location.with_path_segments(self.remainder.clone())
    }

    fn default_match(location: &RouteLocation, global_location: &RouteLocation) -> Self {
        Self {
            location: location.clone(),
            global_location: global_location.clone(),
            params: HashMap::new(),
            consumed_segments: 0,
            remainder: location.path_segments.clone(),
            is_exact: location.path_segments.is_empty(),
        }
    }
}

impl Interpolatable for RouteMatch {}

impl ToPaxValue for RouteMatch {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(vec![
            ("location".to_string(), self.location.to_pax_value()),
            (
                "global_location".to_string(),
                self.global_location.to_pax_value(),
            ),
            ("params".to_string(), string_map_to_pax_value(&self.params)),
            (
                "consumed_segments".to_string(),
                self.consumed_segments.to_pax_value(),
            ),
            ("remainder".to_string(), self.remainder.to_pax_value()),
            ("is_exact".to_string(), self.is_exact.to_pax_value()),
        ])
    }
}

/// Internal router inputs carried into a `RouterInstance`.
#[derive(Default)]
pub struct RouterProperties {
    /// Location scoped to the current router.
    pub input_location: Property<RouteLocation>,
    /// Full application location retained for diagnostics and coordination.
    pub global_location: Property<RouteLocation>,
}

impl ImplToFromPaxAny for RouterProperties {}

impl ToPaxValue for RouterProperties {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(vec![
            (
                "input_location".to_string(),
                self.input_location.to_pax_value(),
            ),
            (
                "global_location".to_string(),
                self.global_location.to_pax_value(),
            ),
        ])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RouteSegmentPattern {
    Literal(String),
    Param(String),
    CatchAll,
}

/// Compiled route branch used by the runtime matcher.
#[derive(Clone, Debug)]
pub struct CompiledRouteBranch {
    /// Whether this branch is the fallback `default=true` branch.
    pub default: bool,
    pattern: Vec<RouteSegmentPattern>,
}

impl CompiledRouteBranch {
    fn from_definition(definition: &ControlFlowRouteBranchDefinition) -> Self {
        Self {
            default: definition.default,
            pattern: definition
                .path
                .as_deref()
                .map(compile_route_pattern)
                .unwrap_or_default(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct RouteSelection {
    branch_index: Option<usize>,
    route_match: Option<RouteMatch>,
}

pub struct RouterInstance {
    base: BaseInstance,
    branches: Vec<CompiledRouteBranch>,
    branch_child_ranges: Vec<Range<usize>>,
}

impl InstanceNode for RouterInstance {
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
            branches: vec![],
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
        f.debug_struct("Router").finish()
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }
}

impl RouterInstance {
    /// Creates a router instance with precompiled route branches.
    pub fn instantiate_with_branches(
        args: InstantiationArgs,
        branches: Vec<CompiledRouteBranch>,
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
            branches,
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

        let input_location =
            expanded_node.with_properties_unwrapped(|properties: &mut RouterProperties| {
                properties.input_location.clone()
            });
        let global_location =
            expanded_node.with_properties_unwrapped(|properties: &mut RouterProperties| {
                properties.global_location.clone()
            });

        let deps = vec![input_location.untyped(), global_location.untyped()];
        let branches = self.branches.clone();
        let branch_child_ranges = self.branch_child_ranges.clone();
        let old_selection = RefCell::new(RouteSelection::default());

        expanded_node
            .children
            .replace_with(Property::computed_with_name(
                move || {
                    let Some(cloned_expanded_node) = weak_ref_self.upgrade() else {
                        panic!("ran evaluator after expanded node dropped (router)")
                    };

                    let input = input_location.get();
                    let global = global_location.get();
                    let selection = select_route_branch(&branches, &input, &global);

                    if *borrow!(old_selection) == selection {
                        return cloned_expanded_node.current_attached_children();
                    }
                    *borrow_mut!(old_selection) = selection.clone();

                    let selected_range = selection
                        .branch_index
                        .and_then(|branch_index| branch_child_ranges.get(branch_index).cloned())
                        .unwrap_or(0..0);
                    let children = borrow!(cloned_self.base().get_instance_children());
                    let start = selected_range.start.min(children.len());
                    let end = selected_range.end.min(children.len());
                    let env = selection
                        .route_match
                        .map(route_scope)
                        .map(|scope| cloned_expanded_node.stack.push(scope))
                        .unwrap_or_else(|| Rc::clone(&cloned_expanded_node.stack));
                    let children_with_envs =
                        children[start..end].iter().cloned().zip(iter::repeat(env));
                    cloned_expanded_node.generate_children(
                        children_with_envs,
                        &cloned_context,
                        &cloned_expanded_node.parent_frame,
                        is_mount,
                    )
                },
                &deps,
                &format!("router_children (node id: {})", expanded_node.id.0),
            ));
    }
}

pub fn compile_route_branches(
    definitions: &[ControlFlowRouteBranchDefinition],
) -> Vec<CompiledRouteBranch> {
    definitions
        .iter()
        .map(CompiledRouteBranch::from_definition)
        .collect()
}

fn compile_route_pattern(path: &str) -> Vec<RouteSegmentPattern> {
    if path.is_empty() {
        panic!("Route path must not be empty");
    }

    let normalized = if path == "/" {
        "/"
    } else {
        path.trim_start_matches('/').trim_end_matches('/')
    };

    if normalized.is_empty() {
        return vec![];
    }

    let segments = normalized
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let mut pattern = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        if *segment == "*" {
            if index != segments.len() - 1 {
                panic!("Route catch-all '*' must be terminal");
            }
            pattern.push(RouteSegmentPattern::CatchAll);
        } else if let Some(param_name) = segment.strip_prefix(':') {
            if param_name.is_empty() {
                panic!("Route params must be named");
            }
            pattern.push(RouteSegmentPattern::Param(param_name.to_string()));
        } else if segment.contains('*') {
            panic!("Route only supports terminal '*'");
        } else {
            pattern.push(RouteSegmentPattern::Literal(segment.to_string()));
        }
    }

    pattern
}

fn select_route_branch(
    branches: &[CompiledRouteBranch],
    input: &RouteLocation,
    global: &RouteLocation,
) -> RouteSelection {
    let mut default_index = None;

    for (index, branch) in branches.iter().enumerate() {
        if branch.default {
            default_index.get_or_insert(index);
            continue;
        }

        if let Some(route_match) = match_route_branch(branch, input, global) {
            return RouteSelection {
                branch_index: Some(index),
                route_match: Some(route_match),
            };
        }
    }

    default_index
        .map(|index| RouteSelection {
            branch_index: Some(index),
            route_match: Some(RouteMatch::default_match(input, global)),
        })
        .unwrap_or_default()
}

fn match_route_branch(
    branch: &CompiledRouteBranch,
    input: &RouteLocation,
    global: &RouteLocation,
) -> Option<RouteMatch> {
    let mut params = HashMap::new();
    let mut input_index = 0usize;

    for (pattern_index, segment_pattern) in branch.pattern.iter().enumerate() {
        match segment_pattern {
            RouteSegmentPattern::Literal(expected) => {
                let actual = input.path_segments.get(input_index)?;
                if actual != expected {
                    return None;
                }
                input_index += 1;
            }
            RouteSegmentPattern::Param(name) => {
                let actual = input.path_segments.get(input_index)?;
                params.insert(name.clone(), actual.clone());
                input_index += 1;
            }
            RouteSegmentPattern::CatchAll => {
                if pattern_index != branch.pattern.len() - 1 {
                    return None;
                }

                let remainder = input.path_segments[input_index..].to_vec();
                return Some(RouteMatch {
                    location: input.clone(),
                    global_location: global.clone(),
                    params,
                    consumed_segments: input_index,
                    is_exact: remainder.is_empty(),
                    remainder,
                });
            }
        }
    }

    if input_index != input.path_segments.len() {
        return None;
    }

    Some(RouteMatch {
        location: input.clone(),
        global_location: global.clone(),
        params,
        consumed_segments: input_index,
        remainder: vec![],
        is_exact: true,
    })
}

fn route_scope(route_match: RouteMatch) -> HashMap<String, Variable> {
    let route_property = Property::new(route_match);
    vec![
        (
            ROUTE_SYMBOL.to_string(),
            Variable::new_from_typed_property(route_property.clone()),
        ),
        (
            INTERNAL_ROUTE_MATCH_SYMBOL.to_string(),
            Variable::new_from_typed_property(route_property),
        ),
    ]
    .into_iter()
    .collect()
}

fn multimap_to_pax_value(map: &HashMap<String, Vec<String>>) -> PaxValue {
    let mut entries = map
        .iter()
        .map(|(key, values)| (key.clone(), values.clone().to_pax_value()))
        .collect::<Vec<_>>();
    entries.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
    PaxValue::Object(entries)
}

fn string_map_to_pax_value(map: &HashMap<String, String>) -> PaxValue {
    let mut entries = map
        .iter()
        .map(|(key, value)| (key.clone(), value.clone().to_pax_value()))
        .collect::<Vec<_>>();
    entries.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
    PaxValue::Object(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::math::{Point2, Transform2};
    use crate::api::{CommonProperties, Layer, Size, Window};
    use crate::{
        BaseInstance, ComponentInstance, Globals, InstanceFlags, RuntimePropertiesStackFrame,
        TransformAndBounds,
    };
    use pax_manifest::cartridge_generation::{
        ComponentTransitionConfig, TRANSITION_PHASE_ENTER, TRANSITION_PHASE_EXIT,
        TRANSITION_PHASE_IDLE,
    };
    use pax_manifest::ValueDefinition;
    use pax_runtime_api::pax_value::{PaxAny, ToFromPaxAny};
    use pax_runtime_api::{Duration, Platform, TargetInfo, OS};
    use std::cell::RefCell;

    fn route(path_segments: &[&str]) -> RouteLocation {
        RouteLocation {
            path_segments: path_segments
                .iter()
                .map(|segment| (*segment).to_string())
                .collect(),
            ..Default::default()
        }
    }

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

    fn leaf_args() -> InstantiationArgs {
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
            transition_config: ComponentTransitionConfig {
                has_enter: true,
                enter_frame_count: 10,
                has_exit: true,
                exit_frame_count: 10,
                timeout_ms: 5_000,
                ..Default::default()
            },
            properties_scope: crate::PropertiesScopeInit::None,
        }
    }

    fn leaf() -> Rc<dyn InstanceNode> {
        ComponentInstance::instantiate(leaf_args())
    }

    struct HitBoxInstance {
        base: BaseInstance,
    }

    impl InstanceNode for HitBoxInstance {
        fn base(&self) -> &BaseInstance {
            &self.base
        }

        fn instantiate(args: InstantiationArgs) -> Rc<Self> {
            Rc::new(Self {
                base: BaseInstance::new(
                    args,
                    InstanceFlags {
                        invisible_to_slot: false,
                        invisible_to_raycasting: false,
                        layer: Layer::Canvas,
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
            write!(f, "HitBox")
        }
    }

    fn hit_box_args(transition_config: ComponentTransitionConfig) -> InstantiationArgs {
        InstantiationArgs {
            prototypical_common_properties: crate::CommonPropertiesInit::Factory(Box::new(
                |_, expanded_node| {
                    expanded_node.is_none().then(|| {
                        let mut cp = CommonProperties::default();
                        cp.width = Property::new(Some(Size::Pixels(40.into())));
                        cp.height = Property::new(Some(Size::Pixels(40.into())));
                        Rc::new(RefCell::new(cp))
                    })
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
            transition_config,
            properties_scope: crate::PropertiesScopeInit::None,
        }
    }

    fn hit_box(transition_config: ComponentTransitionConfig) -> Rc<dyn InstanceNode> {
        HitBoxInstance::instantiate(hit_box_args(transition_config))
    }

    fn dynamic_exit_duration_leaf(frames: f64) -> Rc<dyn InstanceNode> {
        let mut args = leaf_args();
        args.transition_config = ComponentTransitionConfig {
            has_exit: true,
            exit_dynamic_durations: vec![ValueDefinition::LiteralValue(PaxValue::Duration(
                Duration::Frames(frames.into()),
            ))],
            timeout_ms: 5_000,
            ..Default::default()
        };
        ComponentInstance::instantiate(args)
    }

    fn router_args(
        input_location: Property<RouteLocation>,
        global_location: Property<RouteLocation>,
        children: Vec<Rc<dyn InstanceNode>>,
    ) -> InstantiationArgs {
        let input_location_for_factory = input_location.clone();
        let global_location_for_factory = global_location.clone();
        InstantiationArgs {
            prototypical_common_properties: crate::CommonPropertiesInit::Factory(
                default_common_properties_factory(),
            ),
            prototypical_properties: crate::PropertiesInit::Factory(Box::new(
                move |_, expanded_node| {
                    if let Some(expanded_node) = expanded_node {
                        expanded_node.with_properties_unwrapped(
                            |properties: &mut RouterProperties| {
                                properties
                                    .input_location
                                    .replace_with(input_location_for_factory.clone());
                                properties
                                    .global_location
                                    .replace_with(global_location_for_factory.clone());
                            },
                        );
                        return None;
                    }

                    Some(Rc::new(RefCell::new({
                        let mut properties = RouterProperties::default();
                        properties.input_location = input_location_for_factory.clone();
                        properties.global_location = global_location_for_factory.clone();
                        properties.to_pax_any()
                    })))
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

    fn mounted_router(
        input_location: Property<RouteLocation>,
        branch_definitions: Vec<ControlFlowRouteBranchDefinition>,
        branch_child_ranges: Vec<Range<usize>>,
        children: Vec<Rc<dyn InstanceNode>>,
    ) -> (Rc<ExpandedNode>, Rc<ExpandedNode>, Rc<RuntimeContext>) {
        let router: Rc<dyn InstanceNode> = RouterInstance::instantiate_with_branches(
            router_args(input_location.clone(), input_location, children),
            compile_route_branches(&branch_definitions),
            branch_child_ranges,
        );
        let root_component =
            ComponentInstance::instantiate(component_args(Some(vec![Rc::clone(&router)])));
        let context = Rc::new(RuntimeContext::new(test_globals()));
        let root = ExpandedNode::initialize_root(root_component, &context);
        root.recurse_update(&context);
        let router_node = root.children.get().remove(0);
        (root, router_node, context)
    }

    fn route_param(node: &Rc<ExpandedNode>, name: &str) -> Option<String> {
        let route = node.stack.resolve_symbol(ROUTE_SYMBOL)?.get_as_pax_value();
        let PaxValue::Object(route_fields) = route else {
            return None;
        };
        let params = route_fields
            .into_iter()
            .find_map(|(field, value)| (field == "params").then_some(value))?;
        let PaxValue::Object(param_fields) = params else {
            return None;
        };
        param_fields
            .into_iter()
            .find_map(|(field, value)| (field == name).then_some(value))
            .and_then(|value| match value {
                PaxValue::String(value) => Some(value),
                _ => None,
            })
    }

    #[test]
    fn terminal_catch_all_preserves_remainder() {
        let branches = compile_route_branches(&[ControlFlowRouteBranchDefinition {
            path: Some("/settings/*".to_string()),
            default: false,
            child_ids: vec![],
        }]);
        let input = route(&["settings", "team", "42"]);
        let selection = select_route_branch(&branches, &input, &input);

        assert_eq!(selection.branch_index, Some(0));
        let route_match = selection.route_match.unwrap();
        assert_eq!(route_match.consumed_segments, 1);
        assert_eq!(
            route_match.remainder,
            vec!["team".to_string(), "42".to_string()]
        );
        assert!(!route_match.is_exact);
    }

    #[test]
    fn params_are_captured_for_exact_routes() {
        let branches = compile_route_branches(&[ControlFlowRouteBranchDefinition {
            path: Some("/docs/:slug".to_string()),
            default: false,
            child_ids: vec![],
        }]);
        let input = route(&["docs", "router"]);
        let selection = select_route_branch(&branches, &input, &input);

        assert_eq!(selection.branch_index, Some(0));
        let route_match = selection.route_match.unwrap();
        assert_eq!(route_match.params.get("slug"), Some(&"router".to_string()));
        assert_eq!(route_match.remainder, Vec::<String>::new());
        assert!(route_match.is_exact);
    }

    #[test]
    fn relative_paths_compile_like_absolute_paths() {
        let branches = compile_route_branches(&[ControlFlowRouteBranchDefinition {
            path: Some("settings/*".to_string()),
            default: false,
            child_ids: vec![],
        }]);
        let input = route(&["settings", "integrations", "logs"]);
        let selection = select_route_branch(&branches, &input, &input);

        assert_eq!(selection.branch_index, Some(0));
        let route_match = selection.route_match.unwrap();
        assert_eq!(
            route_match.remainder,
            vec!["integrations".to_string(), "logs".to_string()]
        );
    }

    #[test]
    fn default_branch_receives_unmatched_input() {
        let branches = compile_route_branches(&[
            ControlFlowRouteBranchDefinition {
                path: Some("/docs/:slug".to_string()),
                default: false,
                child_ids: vec![],
            },
            ControlFlowRouteBranchDefinition {
                path: None,
                default: true,
                child_ids: vec![],
            },
        ]);
        let input = route(&["missing"]);
        let selection = select_route_branch(&branches, &input, &input);

        assert_eq!(selection.branch_index, Some(1));
        let route_match = selection.route_match.unwrap();
        assert_eq!(route_match.remainder, vec!["missing".to_string()]);
        assert_eq!(route_match.location, input);
    }

    #[test]
    fn same_branch_route_match_change_enters_new_tree_and_exits_old_tree() {
        let input_location = Property::new(route(&["users", "1"]));
        let (_root, router_node, context) = mounted_router(
            input_location.clone(),
            vec![ControlFlowRouteBranchDefinition {
                path: Some("/users/:id".to_string()),
                default: false,
                child_ids: vec![],
            }],
            vec![0..1],
            vec![leaf()],
        );

        let initial_active = borrow!(router_node.active_children).clone();
        assert_eq!(initial_active.len(), 1);
        assert_eq!(route_param(&initial_active[0], "id"), Some("1".to_string()));

        input_location.set(route(&["users", "2"]));
        router_node.recurse_update(&context);

        let active = borrow!(router_node.active_children).clone();
        let exiting = borrow!(router_node.exiting_children).clone();
        assert_eq!(active.len(), 1);
        assert_eq!(exiting.len(), 1);
        assert_ne!(active[0].id.0, initial_active[0].id.0);
        assert_eq!(route_param(&active[0], "id"), Some("2".to_string()));
        assert_eq!(route_param(&exiting[0], "id"), Some("1".to_string()));
        assert_eq!(active[0].transition_phase.get(), TRANSITION_PHASE_ENTER);
        assert_eq!(exiting[0].transition_phase.get(), TRANSITION_PHASE_EXIT);
    }

    #[test]
    fn branch_change_transitions_each_root_of_multi_root_routes() {
        let input_location = Property::new(route(&["alpha"]));
        let (_root, router_node, context) = mounted_router(
            input_location.clone(),
            vec![
                ControlFlowRouteBranchDefinition {
                    path: Some("/alpha".to_string()),
                    default: false,
                    child_ids: vec![],
                },
                ControlFlowRouteBranchDefinition {
                    path: Some("/beta".to_string()),
                    default: false,
                    child_ids: vec![],
                },
            ],
            vec![0..2, 2..4],
            vec![leaf(), leaf(), leaf(), leaf()],
        );

        let initial_active = borrow!(router_node.active_children).clone();
        assert_eq!(initial_active.len(), 2);

        input_location.set(route(&["beta"]));
        router_node.recurse_update(&context);

        let active = borrow!(router_node.active_children).clone();
        let exiting = borrow!(router_node.exiting_children).clone();
        assert_eq!(active.len(), 2);
        assert_eq!(exiting.len(), 2);
        assert!(active
            .iter()
            .all(|child| child.transition_phase.get() == TRANSITION_PHASE_ENTER));
        assert!(exiting
            .iter()
            .all(|child| child.transition_phase.get() == TRANSITION_PHASE_EXIT));
        assert!(initial_active
            .iter()
            .all(|old_child| exiting.iter().any(|child| child.id.0 == old_child.id.0)));
    }

    #[test]
    fn branch_can_be_reselected_while_previous_instance_is_exiting() {
        let input_location = Property::new(route(&["alpha"]));
        let (_root, router_node, context) = mounted_router(
            input_location.clone(),
            vec![
                ControlFlowRouteBranchDefinition {
                    path: Some("/alpha".to_string()),
                    default: false,
                    child_ids: vec![],
                },
                ControlFlowRouteBranchDefinition {
                    path: Some("/beta".to_string()),
                    default: false,
                    child_ids: vec![],
                },
            ],
            vec![0..1, 1..2],
            vec![leaf(), leaf()],
        );

        let initial_alpha = borrow!(router_node.active_children)[0].id;

        input_location.set(route(&["beta"]));
        router_node.recurse_update(&context);
        assert_eq!(borrow!(router_node.active_children).len(), 1);
        assert_eq!(borrow!(router_node.exiting_children).len(), 1);
        assert_eq!(borrow!(router_node.exiting_children)[0].id, initial_alpha);

        input_location.set(route(&["alpha"]));
        router_node.recurse_update(&context);

        let active = borrow!(router_node.active_children).clone();
        let exiting = borrow!(router_node.exiting_children).clone();
        assert_eq!(active.len(), 1);
        assert_ne!(active[0].id, initial_alpha);
        assert_eq!(active[0].transition_phase.get(), TRANSITION_PHASE_ENTER);
        assert!(exiting.iter().any(|child| child.id == initial_alpha));
    }

    #[test]
    fn retained_exiting_route_tree_does_not_intercept_hit_testing() {
        let input_location = Property::new(route(&["alpha"]));
        let exiting_route_config = ComponentTransitionConfig {
            has_exit: true,
            exit_frame_count: 30,
            timeout_ms: 5_000,
            ..Default::default()
        };
        let (root, router_node, context) = mounted_router(
            input_location.clone(),
            vec![
                ControlFlowRouteBranchDefinition {
                    path: Some("/alpha".to_string()),
                    default: false,
                    child_ids: vec![],
                },
                ControlFlowRouteBranchDefinition {
                    path: Some("/beta".to_string()),
                    default: false,
                    child_ids: vec![],
                },
            ],
            vec![0..1, 1..2],
            vec![hit_box(exiting_route_config), hit_box(Default::default())],
        );

        input_location.set(route(&["beta"]));
        router_node.recurse_update(&context);

        let active = borrow!(router_node.active_children).clone();
        assert_eq!(active.len(), 1);
        assert_eq!(borrow!(router_node.exiting_children).len(), 1);

        let hits = context.get_elements_beneath_ray(
            Some(root),
            Point2::<Window>::new(5.0, 5.0),
            true,
            vec![],
            false,
        );
        assert_eq!(hits.first().map(|node| node.id), Some(active[0].id));
    }

    #[test]
    fn dynamic_exit_duration_controls_retained_route_cleanup() {
        let input_location = Property::new(route(&["alpha"]));
        let (_root, router_node, context) = mounted_router(
            input_location.clone(),
            vec![
                ControlFlowRouteBranchDefinition {
                    path: Some("/alpha".to_string()),
                    default: false,
                    child_ids: vec![],
                },
                ControlFlowRouteBranchDefinition {
                    path: Some("/beta".to_string()),
                    default: false,
                    child_ids: vec![],
                },
            ],
            vec![0..1, 1..2],
            vec![dynamic_exit_duration_leaf(3.0), leaf()],
        );

        input_location.set(route(&["beta"]));
        router_node.recurse_update(&context);
        assert_eq!(borrow!(router_node.exiting_children).len(), 1);

        context.globals().elapsed_frames.set(2);
        context.drain_node_effects();
        assert_eq!(borrow!(router_node.exiting_children).len(), 1);

        context.globals().elapsed_frames.set(3);
        context.drain_node_effects();
        assert!(borrow!(router_node.exiting_children).is_empty());
    }

    #[test]
    fn active_enter_transition_returns_to_idle_after_duration() {
        let input_location = Property::new(route(&["alpha"]));
        let (_root, router_node, context) = mounted_router(
            input_location.clone(),
            vec![
                ControlFlowRouteBranchDefinition {
                    path: Some("/alpha".to_string()),
                    default: false,
                    child_ids: vec![],
                },
                ControlFlowRouteBranchDefinition {
                    path: Some("/beta".to_string()),
                    default: false,
                    child_ids: vec![],
                },
            ],
            vec![0..1, 1..2],
            vec![leaf(), leaf()],
        );

        input_location.set(route(&["beta"]));
        router_node.recurse_update(&context);

        let active = borrow!(router_node.active_children).clone();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].transition_phase.get(), TRANSITION_PHASE_ENTER);

        context.globals().elapsed_frames.set(9);
        context.drain_node_effects();
        assert_eq!(active[0].transition_phase.get(), TRANSITION_PHASE_ENTER);

        context.globals().elapsed_frames.set(10);
        context.drain_node_effects();
        assert_eq!(active[0].transition_phase.get(), TRANSITION_PHASE_IDLE);
    }

    #[test]
    fn no_match_transitions_out_previous_route_tree() {
        let input_location = Property::new(route(&["alpha"]));
        let (_root, router_node, context) = mounted_router(
            input_location.clone(),
            vec![ControlFlowRouteBranchDefinition {
                path: Some("/alpha".to_string()),
                default: false,
                child_ids: vec![],
            }],
            vec![0..1],
            vec![leaf()],
        );

        assert_eq!(borrow!(router_node.active_children).len(), 1);

        input_location.set(route(&["missing"]));
        router_node.recurse_update(&context);

        let active = borrow!(router_node.active_children).clone();
        let exiting = borrow!(router_node.exiting_children).clone();
        assert!(active.is_empty());
        assert_eq!(exiting.len(), 1);
        assert_eq!(exiting[0].transition_phase.get(), TRANSITION_PHASE_EXIT);
    }
}
