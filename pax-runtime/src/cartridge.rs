use_RefCell!();
use crate::api::NodeContext;
use crate::{
    ConditionalProperties, ExpandedNode, Handler, HandlerRegistry, InstanceNode, InstantiationArgs,
    ReusableInstanceNodeArgs, RouteLocation, RouteMatch, RuntimePropertiesStackFrame,
    RuntimeResolvedPropertyColumns, RuntimeResolvedPropertyEntry, RuntimeSettingsCondition,
    RuntimeSettingsLayer, RuntimeSettingsSource, INTERNAL_ROUTE_LOCATION_SYMBOL,
    INTERNAL_ROUTE_MATCH_SYMBOL,
};
use pax_language::Computable;
use pax_manifest::cartridge_generation::{
    TRANSITION_GENERATION_SYMBOL, TRANSITION_PHASE_ENTER, TRANSITION_PHASE_EXIT,
    TRANSITION_PHASE_SYMBOL, TRANSITION_PLAYHEAD_MILLIS_SYMBOL, TRANSITION_PLAYHEAD_SYMBOL,
    TRANSITION_TAKEOVER_SYMBOL,
};
use pax_manifest::{
    ExpressionInfo, GradientDefinition, GradientElement, GradientShapeDefinition,
    InOutInterruption, LiteralBlockDefinition, LocationInfo, PaxIdentifier, SelectorExpr,
    SettingElement, SettingsBlockElement, TemplateNodeDefinition, TimelineBlockElement,
    TimelineDefinition, TimelineKeyframe, TimelineMarker, TimelineSelectorElement,
    TimelineTrackDefinition, TimelineTrackElement, TransitionDefinition, TypeId, ValueDefinition,
};
use pax_message::{borrow, borrow_mut};
use pax_runtime_api::constants::{COMMON_PROPERTIES, COMMON_PROPERTIES_2D};
use pax_runtime_api::pax_value::functions::{call_function, Functions};
use pax_runtime_api::pax_value::{CoercionRules, PaxAny, ToFromPaxAny, ToPaxValue};
use pax_runtime_api::properties::{PropertyValue, UntypedProperty};
use pax_runtime_api::{
    use_RefCell, CommonProperties, Duration, EasingCurve, Numeric, PaxValue, Property, Rotation,
    Size, Variable,
};
use std::any::Any;
use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;
use std::rc::Rc;

pub trait PaxCartridge {}

#[cfg(all(test, not(feature = "designtime")))]
mod initialization_tests;
#[cfg(test)]
mod typed_binding_tests;

/// PAXEL symbol bound while resolving a property layer to the value from the
/// preceding layer in that same property's precedence stack.
pub const BASE_SYMBOL: &str = "$base";

struct ResolvedRuntimeSettings {
    #[allow(dead_code)]
    defined_properties: BTreeMap<String, ValueDefinition>,
    columns: RuntimeResolvedPropertyColumns,
    provenance: BTreeMap<String, RuntimeResolvedPropertyEntry>,
}

fn imported_settings_layers_for_node(
    expanded_node: Option<&Rc<ExpandedNode>>,
) -> Vec<RuntimeSettingsLayer> {
    let mut layers = Vec::new();
    let Some(expanded_node) = expanded_node else {
        return layers;
    };
    if expanded_node.is_in_import_settings_subtree() {
        return layers;
    }
    let Some(containing_component) = expanded_node.containing_component.upgrade() else {
        return layers;
    };
    layers.extend(
        borrow!(containing_component.imported_settings_layers)
            .iter()
            .cloned(),
    );
    layers
}

fn append_resolved_property_entry(
    columns: &mut RuntimeResolvedPropertyColumns,
    key: &str,
    entry: RuntimeResolvedPropertyEntry,
) {
    columns.entry(key.to_string()).or_default().push(entry);
}

fn add_removed_property_resets(
    previous: &RuntimeResolvedPropertyColumns,
    current: &mut RuntimeResolvedPropertyColumns,
) {
    for property_name in previous.keys() {
        current.entry(property_name.clone()).or_default();
    }
}

fn runtime_property_entry(
    source: RuntimeSettingsSource,
    selector: Option<pax_manifest::SelectorExpr>,
    source_location: Option<LocationInfo>,
    source_stack: Option<Rc<RuntimePropertiesStackFrame>>,
    value: ValueDefinition,
    condition: Option<RuntimeSettingsCondition>,
    axis_index: Option<usize>,
) -> RuntimeResolvedPropertyEntry {
    RuntimeResolvedPropertyEntry {
        source,
        selector,
        source_location,
        source_stack,
        value,
        condition,
        axis_index,
    }
}

fn common_property_2d_axes(name: &str) -> Option<[&'static str; 2]> {
    COMMON_PROPERTIES_2D
        .iter()
        .find_map(|(group_name, axes)| (*group_name == name).then_some(*axes))
}

fn setting_value_can_define_property(value: &ValueDefinition) -> bool {
    matches!(
        value,
        ValueDefinition::LiteralValue(_)
            | ValueDefinition::Block(_)
            | ValueDefinition::Timeline(_)
            | ValueDefinition::Gradient(_)
            | ValueDefinition::Transition(_)
            | ValueDefinition::Expression(_)
            | ValueDefinition::Identifier(_)
            | ValueDefinition::DoubleBinding(_)
    )
}

fn is_lifecycle_transition_setting_key(key: &str) -> bool {
    matches!(key, "in" | "out")
}

fn append_2d_common_property_entries(
    columns: &mut RuntimeResolvedPropertyColumns,
    key: &str,
    value: &ValueDefinition,
    source: RuntimeSettingsSource,
    selector: Option<pax_manifest::SelectorExpr>,
    source_location: Option<LocationInfo>,
    source_stack: Option<Rc<RuntimePropertiesStackFrame>>,
    condition: Option<RuntimeSettingsCondition>,
) -> bool {
    let Some(axes) = common_property_2d_axes(key) else {
        return false;
    };

    append_resolved_property_entry(
        columns,
        key,
        runtime_property_entry(
            source.clone(),
            selector.clone(),
            source_location.clone(),
            source_stack.clone(),
            value.clone(),
            condition.clone(),
            None,
        ),
    );
    for (axis_index, axis_name) in axes.iter().enumerate() {
        append_resolved_property_entry(
            columns,
            axis_name,
            runtime_property_entry(
                source.clone(),
                selector.clone(),
                source_location.clone(),
                source_stack.clone(),
                value.clone(),
                condition.clone(),
                Some(axis_index),
            ),
        );
    }
    true
}

/// Converts a flattened property map into one-entry columns for legacy callers
/// that do not participate in selector/import precedence layering.
pub fn property_columns_from_defined_properties(
    defined_properties: &BTreeMap<String, ValueDefinition>,
) -> RuntimeResolvedPropertyColumns {
    let mut columns = BTreeMap::new();
    for (key, value) in defined_properties {
        if setting_value_can_define_property(value) {
            append_2d_common_property_entries(
                &mut columns,
                key,
                value,
                RuntimeSettingsSource::Inline,
                None,
                None,
                None,
                None,
            );
        }
    }
    for (key, value) in defined_properties {
        if common_property_2d_axes(key).is_some() || !setting_value_can_define_property(value) {
            continue;
        }
        append_resolved_property_entry(
            &mut columns,
            key,
            runtime_property_entry(
                RuntimeSettingsSource::Inline,
                None,
                None,
                None,
                value.clone(),
                None,
                None,
            ),
        );
    }
    columns
}

fn append_setting_elements(
    columns: &mut RuntimeResolvedPropertyColumns,
    elements: &[SettingElement],
    source: RuntimeSettingsSource,
    selector: Option<pax_manifest::SelectorExpr>,
    source_stack: Option<Rc<RuntimePropertiesStackFrame>>,
    condition: Option<RuntimeSettingsCondition>,
) {
    for element in elements {
        let SettingElement::Setting(key, value) = element else {
            continue;
        };
        if is_lifecycle_transition_setting_key(&key.token_value) {
            continue;
        }
        if setting_value_can_define_property(value) {
            append_2d_common_property_entries(
                columns,
                &key.token_value,
                value,
                source.clone(),
                selector.clone(),
                key.token_location.clone(),
                source_stack.clone(),
                condition.clone(),
            );
        }
    }

    for element in elements {
        let SettingElement::Setting(key, value) = element else {
            continue;
        };
        if is_lifecycle_transition_setting_key(&key.token_value) {
            continue;
        }
        if common_property_2d_axes(&key.token_value).is_some()
            || !setting_value_can_define_property(value)
        {
            continue;
        }
        append_resolved_property_entry(
            columns,
            &key.token_value,
            runtime_property_entry(
                source.clone(),
                selector.clone(),
                key.token_location.clone(),
                source_stack.clone(),
                value.clone(),
                condition.clone(),
                None,
            ),
        );
    }
}

#[derive(Clone)]
struct SelectorLayerBlock {
    selector: String,
    elements: Vec<SettingElement>,
    condition: Option<RuntimeSettingsCondition>,
}

fn combine_settings_conditions(
    parent: Option<&RuntimeSettingsCondition>,
    branch_condition: Option<&ExpressionInfo>,
    prior_branch_conditions: &[ExpressionInfo],
) -> Option<RuntimeSettingsCondition> {
    let mut condition = parent.cloned().unwrap_or_default();
    condition
        .negative
        .extend(prior_branch_conditions.iter().cloned());
    if let Some(branch_condition) = branch_condition {
        condition.positive.push(branch_condition.clone());
    }

    if condition.positive.is_empty() && condition.negative.is_empty() {
        None
    } else {
        Some(condition)
    }
}

fn collect_selector_layer_blocks(
    settings: &[SettingsBlockElement],
    condition: Option<&RuntimeSettingsCondition>,
    blocks: &mut Vec<SelectorLayerBlock>,
) {
    for setting in settings {
        match setting {
            SettingsBlockElement::SelectorBlock(token, block) => {
                blocks.push(SelectorLayerBlock {
                    selector: token.token_value.clone(),
                    elements: block.elements.clone(),
                    condition: condition.cloned(),
                });
            }
            SettingsBlockElement::Conditional(block) => {
                let mut prior_branch_conditions = Vec::new();
                for branch in &block.branches {
                    let branch_condition = combine_settings_conditions(
                        condition,
                        branch.condition_expression.as_ref(),
                        &prior_branch_conditions,
                    );
                    collect_selector_layer_blocks(
                        &branch.elements,
                        branch_condition.as_ref(),
                        blocks,
                    );
                    if let Some(condition_expression) = &branch.condition_expression {
                        prior_branch_conditions.push(condition_expression.clone());
                    }
                }
            }
            SettingsBlockElement::Handler(_, _)
            | SettingsBlockElement::Transition(_, _)
            | SettingsBlockElement::Comment(_) => {}
        }
    }
}

fn append_selector_layer_entries(
    columns: &mut RuntimeResolvedPropertyColumns,
    tnd: &TemplateNodeDefinition,
    classes: &[String],
    settings_block: &Option<Vec<SettingsBlockElement>>,
    source: RuntimeSettingsSource,
    source_stack: Option<Rc<RuntimePropertiesStackFrame>>,
) {
    let Some(settings_block) = settings_block else {
        return;
    };

    let mut selector_blocks = Vec::new();
    collect_selector_layer_blocks(settings_block, None, &mut selector_blocks);

    for settings_value in &selector_blocks {
        let Ok(selector) = pax_manifest::SelectorExpr::parse(&settings_value.selector) else {
            continue;
        };
        if matches!(selector, pax_manifest::SelectorExpr::Type(_))
            && tnd.selector_info.matches(&tnd.type_id, &selector)
        {
            append_setting_elements(
                columns,
                &settings_value.elements,
                source.clone(),
                Some(selector),
                source_stack.clone(),
                settings_value.condition.clone(),
            );
        }
    }

    for class in classes {
        let selector = pax_manifest::SelectorExpr::Class(class.clone());
        for settings_value in &selector_blocks {
            let Ok(candidate) = pax_manifest::SelectorExpr::parse(&settings_value.selector) else {
                continue;
            };
            if candidate == selector {
                append_setting_elements(
                    columns,
                    &settings_value.elements,
                    source.clone(),
                    Some(selector.clone()),
                    source_stack.clone(),
                    settings_value.condition.clone(),
                );
            }
        }
    }
    if let Some(id) = &tnd.selector_info.id {
        let selector = pax_manifest::SelectorExpr::Id(id.token_value.clone());
        for settings_value in &selector_blocks {
            let Ok(candidate) = pax_manifest::SelectorExpr::parse(&settings_value.selector) else {
                continue;
            };
            if candidate == selector {
                append_setting_elements(
                    columns,
                    &settings_value.elements,
                    source.clone(),
                    Some(selector.clone()),
                    source_stack.clone(),
                    settings_value.condition.clone(),
                );
            }
        }
    }
}

fn overlay_static_runtime_properties(
    columns: &mut RuntimeResolvedPropertyColumns,
    base_defined_properties: &BTreeMap<String, ValueDefinition>,
) {
    for (key, value) in base_defined_properties {
        if matches!(
            value,
            ValueDefinition::Timeline(_) | ValueDefinition::Transition(_)
        ) {
            if append_2d_common_property_entries(
                columns,
                key,
                value,
                RuntimeSettingsSource::Inline,
                None,
                None,
                None,
                None,
            ) {
                continue;
            }
            append_resolved_property_entry(
                columns,
                key,
                runtime_property_entry(
                    RuntimeSettingsSource::Inline,
                    None,
                    None,
                    None,
                    value.clone(),
                    None,
                    None,
                ),
            );
        }
    }
}

fn timeline_transition_kind(
    timeline: &TimelineDefinition,
    settings: &Option<Vec<SettingsBlockElement>>,
) -> Option<u64> {
    let timeline_name = timeline.name.as_ref()?.token_value.as_str();
    settings.as_ref()?.iter().find_map(|setting| {
        let SettingsBlockElement::Transition(kind, name) = setting else {
            return None;
        };
        if name.token_value != timeline_name {
            return None;
        }
        match kind.token_value.as_str() {
            "in" => Some(TRANSITION_PHASE_ENTER),
            "out" => Some(TRANSITION_PHASE_EXIT),
            _ => None,
        }
    })
}

fn timeline_target_matches_node(
    target: &str,
    tnd: &TemplateNodeDefinition,
    classes: &[String],
) -> bool {
    if let Some(class_name) = target.strip_prefix('.') {
        return classes.iter().any(|candidate| candidate == class_name);
    }
    if let Some(id) = target.strip_prefix('#') {
        return tnd
            .selector_info
            .id
            .as_ref()
            .map(|candidate| candidate.token_value == id)
            .unwrap_or(false);
    }
    false
}

fn append_timeline_selector_entries(
    columns: &mut RuntimeResolvedPropertyColumns,
    tnd: &TemplateNodeDefinition,
    timelines: &[TimelineDefinition],
    component_settings: &Option<Vec<SettingsBlockElement>>,
    classes: &[String],
    transition_classes: Option<&[String]>,
) {
    let mut values =
        BTreeMap::<String, (SelectorExpr, Option<LocationInfo>, ValueDefinition)>::new();
    let empty_base = BTreeMap::new();

    for timeline in timelines {
        let transition_kind = timeline_transition_kind(timeline, component_settings);
        let selector_classes = transition_kind.and(transition_classes).unwrap_or(classes);
        for element in &timeline.elements {
            let TimelineBlockElement::SelectorBlock(target, block) = element else {
                continue;
            };
            if !timeline_target_matches_node(&target.token_value, tnd, selector_classes) {
                continue;
            }
            let Ok(selector) = SelectorExpr::parse(&target.token_value) else {
                continue;
            };
            for element in &block.elements {
                let TimelineSelectorElement::Track(property, track) = element else {
                    continue;
                };
                let value = if let Some(transition_kind) = transition_kind {
                    let track = pax_manifest::PaxManifest::prepare_transition_track(
                        timeline,
                        track,
                        &empty_base,
                        &property.token_value,
                    );
                    let existing = values
                        .remove(&property.token_value)
                        .map(|(_, _, value)| value);
                    let mut transition = match existing {
                        Some(ValueDefinition::Transition(transition)) => transition,
                        Some(value) => TransitionDefinition {
                            starting_value: Some(Box::new(value)),
                            ..Default::default()
                        },
                        None => TransitionDefinition::default(),
                    };
                    match transition_kind {
                        TRANSITION_PHASE_ENTER => transition.enter = Some(track),
                        TRANSITION_PHASE_EXIT => transition.exit = Some(track),
                        _ => {}
                    }
                    ValueDefinition::Transition(transition)
                } else {
                    ValueDefinition::Timeline(pax_manifest::PaxManifest::prepare_timeline_track(
                        timeline, track,
                    ))
                };
                values.insert(
                    property.token_value.clone(),
                    (selector.clone(), property.token_location.clone(), value),
                );
            }
        }
    }

    for (property, (selector, source_location, value)) in values {
        append_resolved_property_entry(
            columns,
            &property,
            runtime_property_entry(
                RuntimeSettingsSource::ComponentSettings,
                Some(selector),
                source_location,
                None,
                value,
                None,
                None,
            ),
        );
    }
}

fn resolve_runtime_settings_with_layers_for_node(
    tnd: &TemplateNodeDefinition,
    base_defined_properties: &BTreeMap<String, ValueDefinition>,
    containing_component_settings: &Option<Vec<SettingsBlockElement>>,
    imported_layers: &[RuntimeSettingsLayer],
    classes: &[String],
    transition_classes: Option<&[String]>,
    timelines: &[TimelineDefinition],
) -> ResolvedRuntimeSettings {
    let mut columns = BTreeMap::new();

    append_selector_layer_entries(
        &mut columns,
        tnd,
        classes,
        containing_component_settings,
        RuntimeSettingsSource::ComponentSettings,
        None,
    );
    for layer in imported_layers {
        append_selector_layer_entries(
            &mut columns,
            tnd,
            classes,
            &Some(layer.settings.clone()),
            RuntimeSettingsSource::ImportedLayer {
                provider_id: layer.provider_id,
                provider_type_id: layer.provider_type_id.clone(),
            },
            Some(Rc::clone(&layer.provider_stack)),
        );
    }
    append_timeline_selector_entries(
        &mut columns,
        tnd,
        timelines,
        containing_component_settings,
        classes,
        transition_classes,
    );
    if let Some(inline_settings) = &tnd.settings {
        append_setting_elements(
            &mut columns,
            inline_settings,
            RuntimeSettingsSource::Inline,
            None,
            None,
            None,
        );
    }
    overlay_static_runtime_properties(&mut columns, base_defined_properties);

    let mut defined_properties = BTreeMap::new();
    let mut provenance = BTreeMap::new();
    for (key, entries) in &columns {
        let Some(last_entry) = entries.last() else {
            continue;
        };
        defined_properties.insert(key.clone(), last_entry.value.clone());
        provenance.insert(key.clone(), last_entry.clone());
    }

    ResolvedRuntimeSettings {
        defined_properties,
        columns,
        provenance,
    }
}

fn resolve_runtime_settings_for_node(
    tnd: &TemplateNodeDefinition,
    base_defined_properties: &BTreeMap<String, ValueDefinition>,
    containing_component_settings: &Option<Vec<SettingsBlockElement>>,
    timelines: &[TimelineDefinition],
    expanded_node: Option<&Rc<ExpandedNode>>,
) -> ResolvedRuntimeSettings {
    let imported_layers = imported_settings_layers_for_node(expanded_node);
    let classes = expanded_node
        .map(|node| node.selector_metadata.borrow().classes.get())
        .unwrap_or_else(|| tnd.selector_info.literal_classes());
    let transition_classes =
        expanded_node.and_then(|node| node.transition_selector_classes.borrow().clone());
    resolve_runtime_settings_with_layers_for_node(
        tnd,
        base_defined_properties,
        containing_component_settings,
        &imported_layers,
        &classes,
        transition_classes.as_deref(),
        timelines,
    )
}

#[allow(dead_code)]
fn resolve_defined_properties_for_node(
    tnd: &TemplateNodeDefinition,
    base_defined_properties: &BTreeMap<String, ValueDefinition>,
    containing_component_settings: &Option<Vec<SettingsBlockElement>>,
    timelines: &[TimelineDefinition],
    expanded_node: Option<&Rc<ExpandedNode>>,
) -> BTreeMap<String, ValueDefinition> {
    let resolved = resolve_runtime_settings_for_node(
        tnd,
        base_defined_properties,
        containing_component_settings,
        timelines,
        expanded_node,
    );
    if let Some(expanded_node) = expanded_node {
        *expanded_node.resolved_property_columns.borrow_mut() = resolved.columns.clone();
        *expanded_node.resolved_property_provenance.borrow_mut() = resolved.provenance.clone();
    }
    resolved.defined_properties
}

fn resolve_property_columns_for_node(
    tnd: &TemplateNodeDefinition,
    base_defined_properties: &BTreeMap<String, ValueDefinition>,
    containing_component_settings: &Option<Vec<SettingsBlockElement>>,
    timelines: &[TimelineDefinition],
    expanded_node: Option<&Rc<ExpandedNode>>,
) -> RuntimeResolvedPropertyColumns {
    let mut resolved = resolve_runtime_settings_for_node(
        tnd,
        base_defined_properties,
        containing_component_settings,
        timelines,
        expanded_node,
    );
    if let Some(expanded_node) = expanded_node {
        if expanded_node.reset_removed_runtime_properties.get() {
            add_removed_property_resets(
                &expanded_node.resolved_property_columns.borrow(),
                &mut resolved.columns,
            );
        }
        *expanded_node.resolved_property_columns.borrow_mut() = resolved.columns.clone();
        *expanded_node.resolved_property_provenance.borrow_mut() = resolved.provenance.clone();
    }
    resolved.columns
}

/// Immutable template inputs shared by common and component property binding.
/// Resolved columns and reactive properties are always node-local; this plan
/// never caches evaluated values or selector matches across nodes or rebinds.
pub struct TemplatePropertyPlan {
    node: TemplateNodeDefinition,
    base_defined_properties: BTreeMap<String, ValueDefinition>,
    component_settings: Option<Vec<SettingsBlockElement>>,
    timelines: Vec<TimelineDefinition>,
    pub(crate) requires_pre_node_binding: bool,
    #[cfg(test)]
    resolutions: std::cell::Cell<usize>,
}

impl TemplatePropertyPlan {
    pub(crate) fn new(
        node: TemplateNodeDefinition,
        base_defined_properties: BTreeMap<String, ValueDefinition>,
        component_settings: Option<Vec<SettingsBlockElement>>,
        timelines: Vec<TimelineDefinition>,
    ) -> Self {
        let requires_pre_node_binding = base_defined_properties
            .values()
            .any(value_uses_local_property_scope)
            || node.settings.iter().flatten().any(|setting| {
                matches!(setting,
                SettingElement::Setting(_, value) if value_uses_local_property_scope(value))
            });
        Self {
            node,
            base_defined_properties,
            component_settings,
            timelines,
            requires_pre_node_binding,
            #[cfg(test)]
            resolutions: std::cell::Cell::new(0),
        }
    }

    pub(crate) fn resolve(
        &self,
        node: Option<&Rc<ExpandedNode>>,
    ) -> RuntimeResolvedPropertyColumns {
        #[cfg(test)]
        self.resolutions.set(self.resolutions.get() + 1);
        resolve_property_columns_for_node(
            &self.node,
            &self.base_defined_properties,
            &self.component_settings,
            &self.timelines,
            node,
        )
    }
}

pub(crate) fn value_uses_local_property_scope(value: &ValueDefinition) -> bool {
    match value {
        ValueDefinition::Timeline(track) => track.use_local_property_scope,
        ValueDefinition::Transition(transition) => transition
            .enter
            .iter()
            .chain(transition.exit.iter())
            .any(|track| track.use_local_property_scope),
        _ => false,
    }
}

pub struct HandlerDescriptor {
    pub name: &'static str,
    pub function: fn(Rc<RefCell<PaxAny>>, &NodeContext, Option<PaxAny>),
}

impl HandlerDescriptor {
    pub const fn new(
        name: &'static str,
        function: fn(Rc<RefCell<PaxAny>>, &NodeContext, Option<PaxAny>),
    ) -> Self {
        Self { name, function }
    }
}

pub struct PropertyScopeDescriptor<T> {
    pub name: &'static str,
    pub variable: fn(&T) -> Variable,
    marker: PhantomData<fn(&T)>,
}

impl<T> PropertyScopeDescriptor<T> {
    pub const fn new(name: &'static str, variable: fn(&T) -> Variable) -> Self {
        Self {
            name,
            variable,
            marker: PhantomData,
        }
    }
}

/// Runtime descriptor for applying all resolved layers of a generated component
/// property.
pub struct ComponentPropertyDescriptor<T> {
    /// Property name as it appears in Pax templates and settings.
    pub name: &'static str,
    /// Generated applicator for all resolved layers of this property.
    pub apply_entries:
        fn(&mut T, &[RuntimeResolvedPropertyEntry], &Rc<RuntimePropertiesStackFrame>),
    marker: PhantomData<fn(&T)>,
}

impl<T> ComponentPropertyDescriptor<T> {
    /// Creates a descriptor for a generated component property.
    pub const fn new(
        name: &'static str,
        apply_entries: fn(
            &mut T,
            &[RuntimeResolvedPropertyEntry],
            &Rc<RuntimePropertiesStackFrame>,
        ),
    ) -> Self {
        Self {
            name,
            apply_entries,
            marker: PhantomData,
        }
    }
}

pub struct ComponentDescriptor<T: Default + ToFromPaxAny + 'static> {
    pub type_id: &'static str,
    pub property_scope_descriptors: &'static [PropertyScopeDescriptor<T>],
    pub property_descriptors: &'static [ComponentPropertyDescriptor<T>],
    pub handler_descriptors: &'static [HandlerDescriptor],
    pub instantiate: fn(InstantiationArgs) -> Rc<dyn InstanceNode>,
    marker: PhantomData<fn(&T)>,
}

impl<T: Default + ToFromPaxAny + 'static> ComponentDescriptor<T> {
    pub const fn new(
        type_id: &'static str,
        property_scope_descriptors: &'static [PropertyScopeDescriptor<T>],
        property_descriptors: &'static [ComponentPropertyDescriptor<T>],
        handler_descriptors: &'static [HandlerDescriptor],
        instantiate: fn(InstantiationArgs) -> Rc<dyn InstanceNode>,
    ) -> Self {
        Self {
            type_id,
            property_scope_descriptors,
            property_descriptors,
            handler_descriptors,
            instantiate,
            marker: PhantomData,
        }
    }
}

pub struct ErasedComponentDescriptor {
    pub type_id: &'static str,
    pub typed_descriptor: &'static (dyn Any + Sync),
    pub create_properties: fn() -> PaxAny,
    pub apply_defined_properties: fn(
        &'static (dyn Any + Sync),
        &mut PaxAny,
        &RuntimeResolvedPropertyColumns,
        &Rc<RuntimePropertiesStackFrame>,
    ),
    pub build_property_scope: fn(&'static (dyn Any + Sync), &PaxAny) -> HashMap<String, Variable>,
    pub handler_descriptors: &'static [HandlerDescriptor],
    pub instantiate: fn(InstantiationArgs) -> Rc<dyn InstanceNode>,
}

impl ErasedComponentDescriptor {
    pub const fn new(
        type_id: &'static str,
        typed_descriptor: &'static (dyn Any + Sync),
        create_properties: fn() -> PaxAny,
        apply_defined_properties: fn(
            &'static (dyn Any + Sync),
            &mut PaxAny,
            &RuntimeResolvedPropertyColumns,
            &Rc<RuntimePropertiesStackFrame>,
        ),
        build_property_scope: fn(&'static (dyn Any + Sync), &PaxAny) -> HashMap<String, Variable>,
        handler_descriptors: &'static [HandlerDescriptor],
        instantiate: fn(InstantiationArgs) -> Rc<dyn InstanceNode>,
    ) -> Self {
        Self {
            type_id,
            typed_descriptor,
            create_properties,
            apply_defined_properties,
            build_property_scope,
            handler_descriptors,
            instantiate,
        }
    }
}

pub struct ComponentDescriptorRegistryEntry {
    pub type_id: &'static str,
    pub descriptor: &'static ErasedComponentDescriptor,
}

impl ComponentDescriptorRegistryEntry {
    pub const fn new(
        type_id: &'static str,
        descriptor: &'static ErasedComponentDescriptor,
    ) -> Self {
        Self {
            type_id,
            descriptor,
        }
    }
}

pub struct TypeFieldDescriptor<T> {
    pub name: &'static str,
    pub apply: fn(&mut T, &ValueDefinition, &Rc<RuntimePropertiesStackFrame>),
    pub dependency: Option<fn(&T) -> UntypedProperty>,
    pub touch: Option<fn(&T)>,
    marker: PhantomData<fn(&T)>,
}

impl<T> TypeFieldDescriptor<T> {
    pub const fn new(
        name: &'static str,
        apply: fn(&mut T, &ValueDefinition, &Rc<RuntimePropertiesStackFrame>),
    ) -> Self {
        Self {
            name,
            apply,
            dependency: None,
            touch: None,
            marker: PhantomData,
        }
    }

    pub const fn with_property_dependency(
        name: &'static str,
        apply: fn(&mut T, &ValueDefinition, &Rc<RuntimePropertiesStackFrame>),
        dependency: fn(&T) -> UntypedProperty,
        touch: fn(&T),
    ) -> Self {
        Self {
            name,
            apply,
            dependency: Some(dependency),
            touch: Some(touch),
            marker: PhantomData,
        }
    }
}

pub struct TypeDescriptor<T: 'static> {
    pub name: &'static str,
    pub fields: &'static [TypeFieldDescriptor<T>],
    marker: PhantomData<fn(&T)>,
}

impl<T> TypeDescriptor<T> {
    pub const fn new(name: &'static str, fields: &'static [TypeFieldDescriptor<T>]) -> Self {
        Self {
            name,
            fields,
            marker: PhantomData,
        }
    }
}

fn missing_handler(_: Rc<RefCell<PaxAny>>, _: &NodeContext, _: Option<PaxAny>) {}

pub fn resolve_handler(
    descriptors: &[HandlerDescriptor],
    fn_name: &str,
) -> fn(Rc<RefCell<PaxAny>>, &NodeContext, Option<PaxAny>) {
    descriptors
        .iter()
        .find(|descriptor| descriptor.name == fn_name)
        .map(|descriptor| descriptor.function)
        .unwrap_or_else(|| {
            log::warn!("Unknown handler name {}", fn_name);
            missing_handler
        })
}

pub fn build_component_handler_registry(
    descriptors: &[HandlerDescriptor],
    handlers: Vec<(String, Vec<String>)>,
) -> Rc<RefCell<HandlerRegistry>> {
    let mut handler_registry = HandlerRegistry::default();
    for (event, functions) in handlers {
        handler_registry.handlers.insert(
            event,
            functions
                .iter()
                .map(|fn_name| {
                    Handler::new_component_handler(resolve_handler(descriptors, fn_name))
                })
                .collect(),
        );
    }
    Rc::new(RefCell::new(handler_registry))
}

pub fn add_inline_handlers_from_descriptors(
    descriptors: &[HandlerDescriptor],
    handlers: Vec<(String, String)>,
    registry: Rc<RefCell<HandlerRegistry>>,
) -> Rc<RefCell<HandlerRegistry>> {
    {
        let mut registry_mut = borrow_mut!(registry);
        for (event, fn_name) in handlers {
            let handler_vec = registry_mut.handlers.entry(event).or_insert_with(Vec::new);
            handler_vec.push(Handler::new_inline_handler(resolve_handler(
                descriptors,
                &fn_name,
            )));
        }
    }
    registry
}

pub fn build_property_scope<T>(
    properties: &T,
    descriptors: &[PropertyScopeDescriptor<T>],
) -> HashMap<String, Variable> {
    descriptors
        .iter()
        .map(|descriptor| {
            (
                descriptor.name.to_string(),
                (descriptor.variable)(properties),
            )
        })
        .collect()
}

pub fn apply_component_descriptor_properties<T: Default + ToFromPaxAny + 'static>(
    properties: &mut T,
    descriptor: &'static ComponentDescriptor<T>,
    property_columns: &RuntimeResolvedPropertyColumns,
    stack_frame: &Rc<RuntimePropertiesStackFrame>,
) {
    thread_local! {
        static WARNED_INCOMPATIBLE_SELECTOR_PROPERTIES: RefCell<std::collections::HashSet<String>> =
            RefCell::new(std::collections::HashSet::new());
    }

    for (property_name, entries) in property_columns {
        let is_known = descriptor
            .property_descriptors
            .iter()
            .any(|property| property.name == property_name)
            || COMMON_PROPERTIES.contains(&property_name.as_str())
            || matches!(
                property_name.as_str(),
                "unclippable" | "_raycastable" | "_suspended"
            );
        if is_known {
            continue;
        }
        for entry in entries.iter().filter(|entry| entry.selector.is_some()) {
            let key = format!(
                "{}:{property_name}:{:?}",
                descriptor.type_id, entry.selector
            );
            WARNED_INCOMPATIBLE_SELECTOR_PROPERTIES.with(|warnings| {
                if warnings.borrow_mut().insert(key) {
                    let location = entry
                        .source_location
                        .as_ref()
                        .map(|location| {
                            format!(
                                " at {}:{}",
                                location.start_line_col.0, location.start_line_col.1
                            )
                        })
                        .unwrap_or_default();
                    log::warn!(
                        "selector {:?} assigns property '{}' which does not apply to {}{}; ignoring it",
                        entry.selector,
                        property_name,
                        descriptor.type_id,
                        location,
                    );
                }
            });
        }
    }

    for property_descriptor in descriptor.property_descriptors {
        if let Some(entries) = property_columns.get(property_descriptor.name) {
            (property_descriptor.apply_entries)(properties, entries, stack_frame);
        }
    }
}

pub fn resolve_component_descriptor(
    entries: &[ComponentDescriptorRegistryEntry],
    type_id: &TypeId,
) -> Option<&'static ErasedComponentDescriptor> {
    let identifier = type_id.get_unique_identifier();
    entries
        .iter()
        .find(|entry| entry.type_id == identifier)
        .map(|entry| entry.descriptor)
}

pub fn erased_create_properties<T: Default + ToFromPaxAny + 'static>() -> PaxAny {
    T::default().to_pax_any()
}

pub fn erased_apply_defined_properties<T: Default + ToFromPaxAny + 'static>(
    typed_descriptor: &'static (dyn Any + Sync),
    pax_any: &mut PaxAny,
    property_columns: &RuntimeResolvedPropertyColumns,
    stack_frame: &Rc<RuntimePropertiesStackFrame>,
) {
    let descriptor = (typed_descriptor as &dyn Any)
        .downcast_ref::<ComponentDescriptor<T>>()
        .expect("Failed to downcast erased component descriptor");
    let properties = T::mut_from_pax_any(pax_any).unwrap_or_else(|err| {
        panic!(
            "Failed to downcast properties to {}: {}",
            descriptor.type_id, err
        )
    });
    apply_component_descriptor_properties(properties, descriptor, property_columns, stack_frame);
}

pub fn erased_build_property_scope<T: Default + ToFromPaxAny + 'static>(
    typed_descriptor: &'static (dyn Any + Sync),
    pax_any: &PaxAny,
) -> HashMap<String, Variable> {
    let descriptor = (typed_descriptor as &dyn Any)
        .downcast_ref::<ComponentDescriptor<T>>()
        .expect("Failed to downcast erased component descriptor");
    let properties = T::ref_from_pax_any(pax_any).unwrap_or_else(|err| {
        panic!(
            "Failed to downcast properties to {}: {}",
            descriptor.type_id, err
        )
    });
    build_property_scope(properties, descriptor.property_scope_descriptors)
}

pub fn build_component_handler_registry_for_descriptor(
    descriptor: &'static ErasedComponentDescriptor,
    handlers: Vec<(String, Vec<String>)>,
) -> Rc<RefCell<HandlerRegistry>> {
    build_component_handler_registry(descriptor.handler_descriptors, handlers)
}

pub fn add_inline_handlers_from_component_descriptor(
    descriptor: &'static ErasedComponentDescriptor,
    handlers: Vec<(String, String)>,
    registry: Rc<RefCell<HandlerRegistry>>,
) -> Rc<RefCell<HandlerRegistry>> {
    add_inline_handlers_from_descriptors(descriptor.handler_descriptors, handlers, registry)
}

pub fn instantiate_component_from_descriptor(
    descriptor: &'static ErasedComponentDescriptor,
    args: InstantiationArgs,
) -> Rc<dyn InstanceNode> {
    (descriptor.instantiate)(args)
}

pub fn build_literal_block_property<T: PropertyValue>(
    descriptor: &'static TypeDescriptor<T>,
    args: &LiteralBlockDefinition,
    stack_frame: Rc<RuntimePropertiesStackFrame>,
) -> Property<T> {
    if descriptor.fields.is_empty() {
        for setting in &args.elements {
            if let SettingElement::Setting(k, _) = setting {
                panic!("Unknown property name {}", k.token_value);
            }
        }
        return Property::new_with_name(Default::default(), descriptor.name);
    }

    let mut properties = T::default();
    for setting in &args.elements {
        if let SettingElement::Setting(k, value_definition) = setting {
            let field_descriptor = descriptor
                .fields
                .iter()
                .find(|descriptor| descriptor.name == k.token_value)
                .unwrap_or_else(|| panic!("Unknown property name {}", k.token_value));
            (field_descriptor.apply)(&mut properties, value_definition, &stack_frame);
        }
    }

    let dependents: Vec<_> = descriptor
        .fields
        .iter()
        .filter_map(|descriptor| {
            descriptor
                .dependency
                .map(|dependency| dependency(&properties))
        })
        .collect();

    if dependents.is_empty() {
        return Property::new_with_name(properties, descriptor.name);
    }

    let touchers: Vec<_> = descriptor
        .fields
        .iter()
        .filter_map(|descriptor| descriptor.touch)
        .collect();
    let computed_properties = properties.clone();
    Property::computed_with_name(
        move || {
            let cloned_properties = computed_properties.clone();
            for touch in &touchers {
                touch(&cloned_properties);
            }
            cloned_properties
        },
        &dependents,
        descriptor.name,
    )
}

fn build_conditional_branch_property(
    branch_kind: pax_manifest::ControlFlowConditionalBranchKind,
    condition_expression: Option<ExpressionInfo>,
    stack_frame: Rc<RuntimePropertiesStackFrame>,
) -> Property<bool> {
    match condition_expression {
        Some(expr_info) => build_conditional_expression_property(expr_info, stack_frame),
        None => Property::new(matches!(
            branch_kind,
            pax_manifest::ControlFlowConditionalBranchKind::Else
        )),
    }
}

fn build_conditional_expression_property(
    expr_info: ExpressionInfo,
    stack_frame: Rc<RuntimePropertiesStackFrame>,
) -> Property<bool> {
    let cloned_stack = stack_frame.clone();
    let expr_ast = expr_info.expression.clone();

    let mut dependencies = Vec::new();
    for dependency in &expr_info.dependencies {
        if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
            dependencies.push(p);
        } else {
            log::warn!("Failed to resolve symbol {}", dependency);
        }
    }

    let name = format!("conditional (if) expr ({})", expr_ast);
    Property::computed_with_name(
        move || {
            let new_value = expr_ast
                .compute(cloned_stack.clone())
                .unwrap_or_else(|err| {
                    log::warn!("Failed to compute expression: {:?}", err);
                    Default::default()
                });
            bool::try_coerce(new_value).unwrap_or_else(|_e| {
                log::warn!("Failed to parse boolean expression: {}", expr_ast);
                Default::default()
            })
        },
        &dependencies,
        &name,
    )
}

pub trait DefinitionToInstanceTraverser {
    fn new(manifest: pax_manifest::PaxManifest) -> Self
    where
        Self: Sized;

    fn get_manifest(&self) -> std::cell::Ref<'_, pax_manifest::PaxManifest>;

    #[cfg(feature = "designtime")]
    fn get_designtime_manager(
        &self,
        project_query: String,
    ) -> std::option::Option<std::rc::Rc<RefCell<pax_designtime::DesigntimeManager>>>;

    fn get_main_component(&self, id: &str) -> std::rc::Rc<crate::ComponentInstance> {
        let main_component_type_id = {
            let manifest = self.get_manifest();
            manifest.main_component_type_id.clone()
        };

        let wrapper_type_id = TypeId::build_singleton(id, Some("RootComponent"));

        let mut args = self.build_component_args(&main_component_type_id);
        args.template_node_identifier = Some(pax_manifest::UniqueTemplateNodeIdentifier::build(
            wrapper_type_id,
            pax_manifest::TemplateNodeId::build(0),
        ));
        let main_component = crate::ComponentInstance::instantiate(args);
        main_component
    }

    fn get_component(
        &mut self,
        type_id: &pax_manifest::TypeId,
    ) -> std::rc::Rc<dyn crate::rendering::InstanceNode> {
        let descriptor = self
            .get_component_descriptor(type_id)
            .expect("Failed to get component descriptor");
        let args = self.build_component_args(type_id);
        instantiate_component_from_descriptor(descriptor, args)
    }

    fn get_component_descriptor(
        &self,
        type_id: &pax_manifest::TypeId,
    ) -> Option<&'static ErasedComponentDescriptor>;

    fn build_component_args(
        &self,
        type_id: &pax_manifest::TypeId,
    ) -> crate::rendering::InstantiationArgs {
        let manifest: std::cell::Ref<pax_manifest::PaxManifest> = self.get_manifest();
        if let None = manifest.components.get(type_id) {
            panic!("Components with type_id {} not found in manifest", type_id);
        }
        let component = manifest.components.get(type_id).unwrap();
        let descriptor = self
            .get_component_descriptor(&type_id)
            .expect(&format!("No component descriptor for type: {}", type_id));
        let prototypical_common_properties = crate::CommonPropertiesInit::Default;
        let mut prototypical_properties = crate::PropertiesInit::DescriptorDefault(descriptor);

        // pull handlers for this component
        let handlers = manifest.get_component_handlers(type_id);
        let handler_registry = Some(build_component_handler_registry_for_descriptor(
            descriptor, handlers,
        ));

        let mut component_template = None;
        if let Some(template) = &component.template {
            let root = template.get_root();
            let mut instances = Vec::new();
            for node_id in root {
                let node = template.get_node(&node_id).unwrap();
                match node.type_id.get_pax_type() {
                    pax_manifest::PaxType::If
                    | pax_manifest::PaxType::Router
                    | pax_manifest::PaxType::Slot
                    | pax_manifest::PaxType::Repeat => {
                        instances.push(self.build_control_flow(type_id, &node_id, None));
                    }
                    pax_manifest::PaxType::Comment => continue,
                    _ => {
                        instances.push(self.build_template_node(type_id, &node_id, None));
                    }
                }
            }
            component_template = Some(RefCell::new(instances));
        }

        let mut component_self_timeline_properties = BTreeMap::new();
        manifest.merge_component_self_timelines_with_properties(
            type_id,
            &mut component_self_timeline_properties,
        );
        if !component_self_timeline_properties.is_empty() {
            prototypical_properties = crate::PropertiesInit::DescriptorInline {
                descriptor,
                defined_properties: component_self_timeline_properties,
            };
        }

        crate::rendering::InstantiationArgs {
            prototypical_common_properties,
            prototypical_properties,
            handler_registry,
            component_template,
            children: None,
            component_settings: component.settings.clone(),
            template_node_identifier: None,
            template_node_type_id: None,
            template_node_selector_info: None,
            transition_config: manifest.get_component_transition_config(type_id),
            properties_scope: crate::PropertiesScopeInit::Descriptor(descriptor),
        }
    }

    fn build_control_flow(
        &self,
        containing_component_type_id: &pax_manifest::TypeId,
        node_id: &pax_manifest::TemplateNodeId,
        prior_node: Option<ReusableInstanceNodeArgs>,
    ) -> std::rc::Rc<dyn crate::rendering::InstanceNode> {
        let manifest = self.get_manifest();
        let prototypical_common_properties = crate::CommonPropertiesInit::Default;

        let containing_component = manifest
            .components
            .get(containing_component_type_id)
            .unwrap();
        let containing_template = containing_component.template.as_ref().unwrap();
        let tnd = containing_template.get_node(node_id).unwrap();
        let unique_identifier = pax_manifest::UniqueTemplateNodeIdentifier::build(
            containing_component_type_id.clone(),
            node_id.clone(),
        );

        let children: RefCell<Vec<Rc<dyn InstanceNode>>> = if let Some(prior_node) = prior_node {
            prior_node.children
        } else {
            RefCell::new(self.build_children(containing_component_type_id, &node_id))
        };
        match tnd.type_id.get_pax_type() {
            pax_manifest::PaxType::If => {
                let control_flow_settings = tnd.control_flow_settings.as_ref().unwrap();
                let conditional_branch_definitions =
                    if control_flow_settings.conditional_branches.is_empty() {
                        control_flow_settings
                            .condition_expression
                            .clone()
                            .map(|condition| {
                                vec![(
                                    pax_manifest::ControlFlowConditionalBranchKind::If,
                                    Some(condition),
                                )]
                            })
                            .unwrap_or_default()
                    } else {
                        control_flow_settings
                            .conditional_branches
                            .iter()
                            .map(|branch| {
                                (
                                    branch.branch_kind.clone(),
                                    branch.condition_expression.clone(),
                                )
                            })
                            .collect::<Vec<_>>()
                    };
                let branch_child_ranges = if control_flow_settings.conditional_branches.is_empty() {
                    vec![]
                } else {
                    let mut start = 0;
                    control_flow_settings
                        .conditional_branches
                        .iter()
                        .map(|branch| {
                            let branch_instance_count = branch
                                .child_ids
                                .iter()
                                .filter(|child_id| {
                                    containing_template
                                        .get_node(child_id)
                                        .map(|child| {
                                            child.type_id.get_pax_type()
                                                != &pax_manifest::PaxType::Comment
                                        })
                                        .unwrap_or(false)
                                })
                                .count();
                            let end = start + branch_instance_count;
                            let range = start..end;
                            start = end;
                            range
                        })
                        .collect()
                };
                let prototypical_properties_factory: Box<
                    dyn Fn(
                        std::rc::Rc<crate::RuntimePropertiesStackFrame>,
                        Option<std::rc::Rc<ExpandedNode>>,
                    )
                        -> Option<std::rc::Rc<RefCell<pax_runtime_api::pax_value::PaxAny>>>,
                > = Box::new(move |stack_frame, expanded_node| {
                    let conditional_branch_properties = conditional_branch_definitions
                        .iter()
                        .cloned()
                        .map(|(branch_kind, condition_expression)| {
                            build_conditional_branch_property(
                                branch_kind,
                                condition_expression,
                                stack_frame.clone(),
                            )
                        })
                        .collect::<Vec<_>>();
                    let boolean_expression = conditional_branch_properties
                        .first()
                        .cloned()
                        .unwrap_or_else(|| Property::new(false));

                    if let Some(expanded_node) = &expanded_node {
                        let expanded_node = borrow!(**expanded_node);
                        let outer_ref = expanded_node.properties.borrow();
                        let rc = Rc::clone(&outer_ref);
                        let mut inner_ref = (*rc).borrow_mut();
                        let cp = ConditionalProperties::mut_from_pax_any(&mut inner_ref).unwrap();
                        cp.boolean_expression.replace_with(boolean_expression);
                        cp.conditional_branches = conditional_branch_properties;
                        return None;
                    }

                    Some(std::rc::Rc::new(RefCell::new({
                        let mut properties = crate::ConditionalProperties::default();
                        properties.boolean_expression = boolean_expression;
                        properties.conditional_branches = conditional_branch_properties;
                        properties.to_pax_any()
                    })))
                });
                crate::ConditionalInstance::instantiate_with_branch_child_ranges(
                    crate::rendering::InstantiationArgs {
                        prototypical_common_properties,
                        prototypical_properties: crate::PropertiesInit::Factory(
                            prototypical_properties_factory,
                        ),
                        handler_registry: None,
                        component_template: None,
                        children: Some(children),
                        component_settings: None,
                        template_node_identifier: Some(unique_identifier),
                        template_node_type_id: Some(tnd.type_id.clone()),
                        template_node_selector_info: Some(tnd.selector_info.clone()),
                        transition_config: Default::default(),
                        properties_scope: crate::PropertiesScopeInit::None,
                    },
                    branch_child_ranges,
                )
            }
            pax_manifest::PaxType::Router => {
                let control_flow_settings = tnd.control_flow_settings.as_ref().unwrap();
                let route_branch_definitions = control_flow_settings.route_branches.clone();
                let compiled_route_branches =
                    crate::compile_route_branches(&route_branch_definitions);
                let branch_child_ranges = if route_branch_definitions.is_empty() {
                    vec![]
                } else {
                    let mut start = 0;
                    route_branch_definitions
                        .iter()
                        .map(|branch| {
                            let branch_instance_count = branch
                                .child_ids
                                .iter()
                                .filter(|child_id| {
                                    containing_template
                                        .get_node(child_id)
                                        .map(|child| {
                                            child.type_id.get_pax_type()
                                                != &pax_manifest::PaxType::Comment
                                        })
                                        .unwrap_or(false)
                                })
                                .count();
                            let end = start + branch_instance_count;
                            let range = start..end;
                            start = end;
                            range
                        })
                        .collect()
                };

                let prototypical_properties_factory: Box<
                    dyn Fn(
                        std::rc::Rc<crate::RuntimePropertiesStackFrame>,
                        Option<std::rc::Rc<ExpandedNode>>,
                    )
                        -> Option<std::rc::Rc<RefCell<pax_runtime_api::pax_value::PaxAny>>>,
                > = Box::new(move |stack_frame, expanded_node| {
                    let global_location = stack_frame
                        .resolve_symbol_as_erased_property(INTERNAL_ROUTE_LOCATION_SYMBOL)
                        .map(Property::<RouteLocation>::new_from_untyped)
                        .unwrap_or_else(|| Property::new(RouteLocation::root()));
                    let input_location = stack_frame
                        .resolve_symbol_as_erased_property(INTERNAL_ROUTE_MATCH_SYMBOL)
                        .map(Property::<RouteMatch>::new_from_untyped)
                        .map(|route_match| {
                            let dependency = route_match.untyped();
                            Property::computed_with_name(
                                move || route_match.get().remainder_location(),
                                &[dependency],
                                "router input location",
                            )
                        })
                        .unwrap_or_else(|| global_location.clone());

                    if let Some(expanded_node) = &expanded_node {
                        let expanded_node = borrow!(**expanded_node);
                        let outer_ref = expanded_node.properties.borrow();
                        let rc = Rc::clone(&outer_ref);
                        let mut inner_ref = (*rc).borrow_mut();
                        let router =
                            crate::RouterProperties::mut_from_pax_any(&mut inner_ref).unwrap();
                        router.input_location.replace_with(input_location);
                        router.global_location.replace_with(global_location);
                        return None;
                    }

                    Some(std::rc::Rc::new(RefCell::new({
                        let mut properties = crate::RouterProperties::default();
                        properties.input_location = input_location;
                        properties.global_location = global_location;
                        properties.to_pax_any()
                    })))
                });

                crate::RouterInstance::instantiate_with_branches(
                    crate::rendering::InstantiationArgs {
                        prototypical_common_properties,
                        prototypical_properties: crate::PropertiesInit::Factory(
                            prototypical_properties_factory,
                        ),
                        handler_registry: None,
                        component_template: None,
                        children: Some(children),
                        component_settings: None,
                        template_node_identifier: Some(unique_identifier),
                        template_node_type_id: Some(tnd.type_id.clone()),
                        template_node_selector_info: Some(tnd.selector_info.clone()),
                        transition_config: Default::default(),
                        properties_scope: crate::PropertiesScopeInit::None,
                    },
                    compiled_route_branches,
                    branch_child_ranges,
                )
            }
            pax_manifest::PaxType::Slot => {
                let expr_info = tnd
                    .control_flow_settings
                    .as_ref()
                    .unwrap()
                    .slot_index_expression
                    .clone();

                let prototypical_properties_factory: Box<
                    dyn Fn(
                        std::rc::Rc<crate::RuntimePropertiesStackFrame>,
                        Option<std::rc::Rc<ExpandedNode>>,
                    )
                        -> Option<std::rc::Rc<RefCell<pax_runtime_api::pax_value::PaxAny>>>,
                > = Box::new(move |stack_frame, expanded_node| {
                    let Some(expr_info) = expr_info.clone() else {
                        if let Some(expanded_node) = &expanded_node {
                            let expanded_node = borrow!(**expanded_node);
                            let outer_ref = expanded_node.properties.borrow();
                            let rc = Rc::clone(&outer_ref);
                            let mut inner_ref = (*rc).borrow_mut();
                            let slot_properties =
                                crate::Slot::mut_from_pax_any(&mut inner_ref).unwrap();
                            slot_properties.is_remainder.set(true);
                            return None;
                        }

                        return Some(std::rc::Rc::new(RefCell::new({
                            let mut properties = crate::Slot::default();
                            properties.is_remainder = Property::new(true);
                            properties.to_pax_any()
                        })));
                    };

                    let cloned_stack = stack_frame.clone();
                    let expr_ast = expr_info.expression.clone();

                    let mut dependencies = Vec::new();
                    for dependency in &expr_info.dependencies {
                        if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                            dependencies.push(p);
                        } else {
                            log::warn!("Failed to resolve symbol {}", dependency);
                        }
                    }

                    if let Some(expanded_node) = &expanded_node {
                        let expanded_node = borrow!(**expanded_node);
                        let outer_ref = expanded_node.properties.borrow();
                        let rc = Rc::clone(&outer_ref);
                        let mut inner_ref = (*rc).borrow_mut();
                        let slot_properties =
                            crate::Slot::mut_from_pax_any(&mut inner_ref).unwrap();
                        slot_properties
                            .index
                            .replace_with(Property::computed_with_name(
                                move || {
                                    let new_value = expr_ast
                                        .compute(cloned_stack.clone())
                                        .unwrap_or_else(|op_err| {
                                            log::warn!(
                                                "Failed to compute expression: {:?}",
                                                op_err
                                            );
                                            Default::default()
                                        });
                                    let coerced: Numeric = Numeric::try_coerce(new_value)
                                        .unwrap_or_else(|_| {
                                            log::warn!(
                                                "Failed to parse slot index expression: {}",
                                                expr_ast
                                            );
                                            Default::default()
                                        });
                                    coerced
                                },
                                &dependencies,
                                "slot index",
                            ));
                        return None;
                    }

                    Some(std::rc::Rc::new(RefCell::new({
                        let mut properties = crate::Slot::default();

                        properties.index = Property::computed_with_name(
                            move || {
                                let new_value = expr_ast
                                    .compute(cloned_stack.clone())
                                    .unwrap_or_else(|op_err| {
                                        log::warn!("Failed to compute expression: {:?}", op_err);
                                        Default::default()
                                    });
                                let coerced: Numeric = Numeric::try_coerce(new_value)
                                    .unwrap_or_else(|_| {
                                        log::warn!(
                                            "Failed to parse slot index expression: {}",
                                            expr_ast
                                        );
                                        Default::default()
                                    });
                                coerced
                            },
                            &dependencies,
                            "slot index",
                        );
                        properties.to_pax_any()
                    })))
                });
                crate::SlotInstance::instantiate(crate::rendering::InstantiationArgs {
                    prototypical_common_properties,
                    prototypical_properties: crate::PropertiesInit::Factory(
                        prototypical_properties_factory,
                    ),
                    handler_registry: None,
                    component_template: None,
                    children: Some(children),
                    component_settings: None,
                    template_node_identifier: Some(unique_identifier),
                    template_node_type_id: Some(tnd.type_id.clone()),
                    template_node_selector_info: Some(tnd.selector_info.clone()),
                    transition_config: Default::default(),
                    properties_scope: crate::PropertiesScopeInit::None,
                })
            }
            pax_manifest::PaxType::Repeat => {
                let source_expression_info = tnd
                    .control_flow_settings
                    .as_ref()
                    .unwrap()
                    .repeat_source_expression
                    .clone()
                    .unwrap();
                let repeat_key_expression = tnd
                    .control_flow_settings
                    .as_ref()
                    .unwrap()
                    .repeat_key_expression
                    .clone();
                let predictate_definition = tnd
                    .control_flow_settings
                    .as_ref()
                    .unwrap()
                    .repeat_predicate_definition
                    .clone()
                    .unwrap();
                let prototypical_properties_factory: Box<
                    dyn Fn(
                        std::rc::Rc<crate::RuntimePropertiesStackFrame>,
                        Option<std::rc::Rc<ExpandedNode>>,
                    )
                        -> Option<std::rc::Rc<RefCell<pax_runtime_api::pax_value::PaxAny>>>,
                > = Box::new(move |stack_frame, expanded_node| {
                    let cloned_stack = stack_frame.clone();
                    let expr = source_expression_info.expression.clone();
                    let deps = source_expression_info.dependencies.clone();

                    let mut dependencies = Vec::new();
                    for dependency in &deps {
                        if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                            dependencies.push(p);
                        } else {
                            log::warn!("Failed to resolve symbol {}", dependency);
                        }
                    }

                    let (elem, index) = match &predictate_definition {
                        pax_manifest::ControlFlowRepeatPredicateDefinition::ElemId(id) => {
                            (Some(id.clone()), None)
                        }
                        pax_manifest::ControlFlowRepeatPredicateDefinition::ElemIdIndexId(
                            t1,
                            t2,
                        ) => (Some(t1.clone()), Some(t2.clone())),
                    };

                    if let Some(expanded_node) = &expanded_node {
                        let expanded_node = borrow!(**expanded_node);
                        let outer_ref = expanded_node.properties.borrow();
                        let rc = Rc::clone(&outer_ref);
                        let mut inner_ref = (*rc).borrow_mut();
                        let repeat =
                            crate::RepeatProperties::mut_from_pax_any(&mut inner_ref).unwrap();
                        repeat
                            .source_expression
                            .replace_with(Property::computed_with_name(
                                move || {
                                    expr.compute(cloned_stack.clone()).unwrap_or_else(|op_err| {
                                        log::warn!("Failed to compute expression: {:?}", op_err);
                                        Default::default()
                                    })
                                },
                                &dependencies,
                                "repeat source vec",
                            ));
                        repeat.iterator_i_symbol.replace_with(Property::new(index));
                        repeat
                            .iterator_elem_symbol
                            .replace_with(Property::new(elem));
                        repeat.repeat_key_expression = repeat_key_expression.clone();
                        return None;
                    }

                    Some(std::rc::Rc::new(RefCell::new({
                        let mut properties = crate::RepeatProperties::default();

                        properties.source_expression = Property::computed_with_name(
                            move || {
                                expr.compute(cloned_stack.clone()).unwrap_or_else(|op_err| {
                                    log::warn!("Failed to compute expression: {:?}", op_err);
                                    Default::default()
                                })
                            },
                            &dependencies,
                            "repeat source vec",
                        );

                        properties
                            .iterator_i_symbol
                            .replace_with(Property::new(index));
                        properties
                            .iterator_elem_symbol
                            .replace_with(Property::new(elem));
                        properties.repeat_key_expression = repeat_key_expression.clone();
                        properties.to_pax_any()
                    })))
                });
                crate::RepeatInstance::instantiate(crate::rendering::InstantiationArgs {
                    prototypical_common_properties,
                    prototypical_properties: crate::PropertiesInit::Factory(
                        prototypical_properties_factory,
                    ),
                    handler_registry: None,
                    component_template: None,
                    children: Some(children),
                    component_settings: None,
                    template_node_identifier: Some(unique_identifier),
                    template_node_type_id: Some(tnd.type_id.clone()),
                    template_node_selector_info: Some(tnd.selector_info.clone()),
                    transition_config: Default::default(),
                    properties_scope: crate::PropertiesScopeInit::None,
                })
            }
            _ => {
                unreachable!("Unexpected control flow type {}", tnd.type_id)
            }
        }
    }

    fn build_children(
        &self,
        containing_component_type_id: &pax_manifest::TypeId,
        node_id: &pax_manifest::TemplateNodeId,
    ) -> Vec<std::rc::Rc<dyn crate::rendering::InstanceNode>> {
        let manifest = self.get_manifest();
        let containing_component = manifest
            .components
            .get(containing_component_type_id)
            .unwrap();
        let containing_template = containing_component.template.as_ref().unwrap();
        let children = containing_template.get_children(node_id);

        let mut children_instances = Vec::new();
        for child_id in &children.unwrap_or_default() {
            let child = containing_template.get_node(&child_id).unwrap();
            match child.type_id.get_pax_type() {
                pax_manifest::PaxType::If
                | pax_manifest::PaxType::Router
                | pax_manifest::PaxType::Slot
                | pax_manifest::PaxType::Repeat => {
                    children_instances.push(self.build_control_flow(
                        containing_component_type_id,
                        &child_id,
                        None,
                    ));
                }
                pax_manifest::PaxType::Comment => continue,
                _ => {
                    children_instances.push(self.build_template_node(
                        containing_component_type_id,
                        child_id,
                        None,
                    ));
                }
            }
        }
        children_instances
    }

    fn build_template_node(
        &self,
        containing_component_type_id: &pax_manifest::TypeId,
        node_id: &pax_manifest::TemplateNodeId,
        prior_node: Option<ReusableInstanceNodeArgs>,
    ) -> std::rc::Rc<dyn crate::rendering::InstanceNode> {
        let manifest = self.get_manifest();

        let containing_component = manifest
            .components
            .get(containing_component_type_id)
            .unwrap();
        let containing_template = containing_component.template.as_ref().unwrap();
        let node = containing_template.get_node(node_id).unwrap();
        let node = node.clone();
        let containing_component_settings = containing_component.settings.clone();
        let containing_component_timelines = containing_component.timelines.clone();
        let containing_component_descriptor = self
            .get_component_descriptor(containing_component_type_id)
            .unwrap();

        let mut args = self.build_component_args(&node.type_id);
        let node_component_descriptor = self.get_component_descriptor(&node.type_id).unwrap();

        if let Some(prior_node) = prior_node {
            args.handler_registry = prior_node.handler_registry;
            args.children = Some(prior_node.children);
            args.template_node_identifier = prior_node.template_node_identifier;
            args.template_node_type_id = prior_node.template_node_type_id;
            args.template_node_selector_info = prior_node.template_node_selector_info;
        } else {
            let handlers_from_tnd = manifest.get_inline_event_handlers(&node);
            let updated_registry = if let Some(registry) = args.handler_registry {
                add_inline_handlers_from_component_descriptor(
                    containing_component_descriptor,
                    handlers_from_tnd,
                    registry,
                )
            } else {
                add_inline_handlers_from_component_descriptor(
                    containing_component_descriptor,
                    handlers_from_tnd,
                    std::rc::Rc::new(RefCell::new(crate::HandlerRegistry::default())),
                )
            };
            // update handlers from tnd
            args.handler_registry = Some(updated_registry);

            // update children from tnd
            args.children = Some(RefCell::new(
                self.build_children(containing_component_type_id, node_id),
            ));

            // update id
            args.template_node_identifier =
                Some(pax_manifest::UniqueTemplateNodeIdentifier::build(
                    containing_component_type_id.clone(),
                    node_id.clone(),
                ));
            args.template_node_type_id = Some(node.type_id.clone());
            args.template_node_selector_info = Some(node.selector_info.clone());
        }

        // update properties from tnd
        let mut inline_properties =
            manifest.get_inline_properties(containing_component_type_id, node_id, &node);
        manifest
            .merge_component_self_timelines_with_properties(&node.type_id, &mut inline_properties);
        let plan = Rc::new(TemplatePropertyPlan::new(
            node,
            inline_properties,
            containing_component_settings,
            containing_component_timelines,
        ));
        args.prototypical_properties = crate::PropertiesInit::Template {
            descriptor: node_component_descriptor,
            plan: Rc::clone(&plan),
        };
        args.prototypical_common_properties = crate::CommonPropertiesInit::Template(plan);

        args.transition_config.merge_from(
            manifest.get_template_node_transition_config(containing_component_type_id, node_id),
        );

        instantiate_component_from_descriptor(node_component_descriptor, args)
    }

    fn get_template_node_by_id(
        &self,
        id: &str,
    ) -> Option<std::rc::Rc<dyn crate::rendering::InstanceNode>> {
        let manifest = self.get_manifest();
        let main_component_type_id = manifest.main_component_type_id.clone();
        let main_component = manifest.components.get(&main_component_type_id).unwrap();
        let template = main_component.template.as_ref().unwrap();
        for node_id in template.get_ids() {
            if let Some(found) =
                self.recurse_get_template_node_by_id(id, &main_component_type_id, node_id)
            {
                return Some(self.build_template_node(&found.0, &found.1, None));
            }
        }
        None
    }

    fn check_for_id_in_template_node(
        &self,
        id: &str,
        tnd: &pax_manifest::TemplateNodeDefinition,
    ) -> bool {
        if let Some(settings) = &tnd.settings {
            for setting in settings {
                if let pax_manifest::SettingElement::Setting(token, value) = setting {
                    if &token.token_value == "id" {
                        if let pax_manifest::ValueDefinition::Identifier(ident) = value {
                            if ident.name == id {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    fn recurse_get_template_node_by_id<'a>(
        &'a self,
        id: &str,
        containing_component_type_id: &'a pax_manifest::TypeId,
        node_id: &'a pax_manifest::TemplateNodeId,
    ) -> Option<(pax_manifest::TypeId, pax_manifest::TemplateNodeId)> {
        let manifest = self.get_manifest();
        let containing_component = manifest
            .components
            .get(containing_component_type_id)
            .unwrap();
        let containing_template = containing_component.template.as_ref().unwrap();
        let tnd = containing_template.get_node(node_id).unwrap();

        if self.check_for_id_in_template_node(id, tnd) {
            return Some((containing_component_type_id.clone(), node_id.clone()));
        }

        if let Some(component) = &manifest.components.get(&tnd.type_id) {
            if let Some(template) = &component.template {
                for node_id in template.get_ids() {
                    if let Some(found) =
                        self.recurse_get_template_node_by_id(id, &tnd.type_id, node_id)
                    {
                        return Some(found.clone());
                    }
                }
            }
        }
        None
    }
}

// Shared by rich and release-baked cartridges. Keep eligibility in the runtime
// rather than baking a second expression form or changing value serialization.
fn try_typed_property_binding<T: PropertyValue + CoercionRules>(
    name: &str,
    definition: &ValueDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> Option<Property<T>> {
    use pax_language::interpreter::{PaxExpression, PaxPrimary};
    fn identifier(expression: &PaxExpression) -> Option<&str> {
        match expression {
            PaxExpression::Primary(primary) => match primary.as_ref() {
                PaxPrimary::Identifier(id, accessors) if accessors.is_empty() => Some(&id.name),
                PaxPrimary::Grouped(expression, None) => identifier(expression),
                _ => None,
            },
            _ => None,
        }
    }
    let symbol = match definition {
        ValueDefinition::Identifier(id) => &id.name,
        ValueDefinition::Expression(info) => identifier(&info.expression)?,
        _ => return None,
    };
    let variable = stack.resolve_symbol_as_variable(symbol)?;
    if let ValueDefinition::Expression(info) = definition {
        // Preserve unusual/manual expression dependency metadata via fallback.
        if info.dependencies.len() != 1
            || stack
                .resolve_symbol_as_erased_property(&info.dependencies[0])?
                .get_id()
                != variable.get_untyped_property().get_id()
        {
            return None;
        }
    }
    variable.try_typed_binding(name)
}

fn build_common_property_value<T>(
    name: &str,
    value_definition: &ValueDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> Property<Option<T>>
where
    T: CoercionRules + PropertyValue + ToPaxValue,
{
    if let Some(property) = try_typed_property_binding(name, value_definition, stack) {
        return property;
    }
    let cloned_stack = stack.clone();
    match value_definition {
        pax_manifest::ValueDefinition::LiteralValue(lv) => {
            let val =
                Option::<T>::try_coerce(resolve_literal_value(lv.clone())).unwrap_or_else(|err| {
                    log::warn!(
                        "Failed to coerce new value for property {name}. Error: {:?}",
                        err
                    );
                    Default::default()
                });
            Property::new_with_name(val, name)
        }
        pax_manifest::ValueDefinition::Timeline(track) => {
            let track = timeline_track_with_base_starting_value(track);
            build_timeline_property(name, &track, cloned_stack.clone())
        }
        pax_manifest::ValueDefinition::Transition(transition) => {
            let transition = transition_with_base_starting_value(transition);
            build_transition_property(name, &transition, cloned_stack.clone())
        }
        pax_manifest::ValueDefinition::DoubleBinding(identifier) => {
            let untyped_property =
                if let Some(p) = stack.resolve_symbol_as_erased_property(&identifier.name) {
                    p
                } else {
                    log::warn!("Failed to resolve symbol {}", identifier.name);
                    return Default::default();
                };
            Property::new_from_untyped(untyped_property.clone())
        }
        pax_manifest::ValueDefinition::Expression(info) => {
            let mut dependents = vec![];
            for dependency in &info.dependencies {
                if let Some(p) = stack.resolve_symbol_as_erased_property(dependency) {
                    dependents.push(p);
                } else {
                    log::warn!("Failed to resolve symbol {}", dependency);
                }
            }
            let cloned_ast = info.expression.clone();
            let expression_label = cloned_ast.to_string();
            let property_name = name.to_string();
            Property::computed_with_name(
                move || {
                    let new_value = cloned_ast
                        .compute(cloned_stack.clone())
                        .unwrap_or_else(|_| {
                            log::warn!("Failed to compute expr: {}", expression_label);
                            Default::default()
                        });
                    Option::<T>::try_coerce(new_value).unwrap_or_else(|err| {
                        log::warn!(
                            "Failed to coerce new value for property {property_name}. Error: {:?}",
                            err
                        );
                        Default::default()
                    })
                },
                &dependents,
                name,
            )
        }
        pax_manifest::ValueDefinition::Gradient(_) => {
            let mut dependents = Vec::new();
            collect_value_definition_dependencies(value_definition, stack, &mut dependents);
            let value_definition = value_definition.clone();
            let property_name = name.to_string();
            Property::computed_with_name(
                move || {
                    evaluate_value_definition_to_pax_value(&value_definition, &cloned_stack)
                        .and_then(|new_value| {
                            Option::<T>::try_coerce(new_value)
                                .map_err(|err| {
                                    log::warn!(
                                        "Failed to coerce new value for property {property_name}. Error: {:?}",
                                        err
                                    );
                                    err
                                })
                                .ok()
                        })
                        .unwrap_or_default()
                },
                &dependents,
                name,
            )
        }
        pax_manifest::ValueDefinition::Identifier(ident) => {
            if let Some(variable) = stack.resolve_symbol_as_variable(&ident.name) {
                let property_name = name.to_string();
                let untyped = variable.get_untyped_property().clone();
                let cloned_variable = variable.clone();
                Property::computed_with_name(
                    move || {
                        let new_value = cloned_variable.get_as_pax_value();
                        Option::<T>::try_coerce(new_value).unwrap_or_else(|err| {
                            log::warn!(
                                "Failed to coerce new value for property {property_name}. Error: {:?}",
                                err
                            );
                            Default::default()
                        })
                    },
                    &[untyped],
                    &ident.name,
                )
            } else {
                log::warn!("Failed to resolve symbol {}", ident.name);
                Property::default()
            }
        }
        _ => unreachable!("Invalid value definition for {name}"),
    }
}

fn axis_component_value(value: PaxValue, axis_index: usize) -> Result<PaxValue, String> {
    match value {
        PaxValue::Vec(values) => {
            if values.len() != 2 {
                return Err(format!(
                    "expected 2 elements for multi-axis common property, got {}",
                    values.len()
                ));
            }
            Ok(values[axis_index].clone())
        }
        PaxValue::Option(value) => match *value {
            Some(value) => axis_component_value(value, axis_index),
            None => Ok(PaxValue::Option(Box::new(None))),
        },
        value => Ok(value),
    }
}

fn coerce_axis_component<T>(
    value: PaxValue,
    axis_index: usize,
    name: &str,
) -> Result<Option<T>, String>
where
    T: CoercionRules + PropertyValue,
{
    let axis_value = axis_component_value(value, axis_index)?;
    Option::<T>::try_coerce(axis_value)
        .map_err(|err| format!("failed to coerce axis {axis_index} for {name}: {err}"))
}

fn sample_axis_timeline_track<T: CoercionRules + PropertyValue>(
    track: &TimelineTrackDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
    axis_index: usize,
    name: &str,
    initial_override: Option<&T>,
) -> Option<T> {
    match resolve_timeline_sample(track, stack, |_| true)? {
        TimelineSample::Value { value, initial } => {
            if initial {
                if let Some(value) = initial_override {
                    return Some(value.clone());
                }
            }
            coerce_axis_component::<T>(resolve_literal_value(value), axis_index, name)
                .ok()
                .flatten()
        }
        TimelineSample::Interpolated {
            current,
            next,
            easing,
            progress,
            initial,
        } => {
            let current = if initial {
                initial_override.cloned()
            } else {
                None
            }
            .or_else(|| {
                coerce_axis_component::<T>(resolve_literal_value(current), axis_index, name)
                    .ok()
                    .flatten()
            })?;
            let next = coerce_axis_component::<T>(resolve_literal_value(next), axis_index, name)
                .ok()
                .flatten()?;
            let curve = easing_curve_from_name(easing.as_deref());
            Some(curve.interpolate(&current, &next, progress))
        }
    }
}

fn build_axis_timeline_property<T: CoercionRules + PropertyValue>(
    name: &str,
    track: &TimelineTrackDefinition,
    stack: Rc<RuntimePropertiesStackFrame>,
    axis_index: usize,
) -> Property<Option<T>> {
    let mut dependents = Vec::new();
    collect_value_definition_dependencies(
        &ValueDefinition::Timeline(track.clone()),
        &stack,
        &mut dependents,
    );

    let cloned_stack = stack.clone();
    let cloned_track = track.clone();
    let property_name = name.to_string();
    Property::computed_with_name(
        move || {
            sample_axis_timeline_track(
                &cloned_track,
                &cloned_stack,
                axis_index,
                &property_name,
                None,
            )
        },
        &dependents,
        name,
    )
}

fn coerce_axis_transition_starting_value<T: CoercionRules + PropertyValue>(
    transition: &TransitionDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
    axis_index: usize,
    name: &str,
) -> Option<T> {
    transition
        .starting_value
        .as_ref()
        .and_then(|value| evaluate_value_definition_to_pax_value(value, stack))
        .and_then(|value| coerce_axis_component::<T>(value, axis_index, name).ok())
        .flatten()
}

fn sample_axis_transition_track<T: CoercionRules + PropertyValue>(
    transition: &TransitionDefinition,
    track: Option<&TimelineTrackDefinition>,
    stack: &Rc<RuntimePropertiesStackFrame>,
    axis_index: usize,
    name: &str,
    initial_override: Option<&T>,
) -> Option<T> {
    let mut track = track?.clone();
    if track.starting_value.is_none() {
        track.starting_value = transition.starting_value.clone();
    }
    align_transition_track_playhead_to_duration(&mut track, stack);
    sample_axis_timeline_track(&track, stack, axis_index, name, initial_override)
}

fn sample_axis_transition_property<T: CoercionRules + PropertyValue>(
    transition: &TransitionDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
    axis_index: usize,
    name: &str,
    initial_override: Option<&T>,
) -> Option<T> {
    let phase = stack
        .resolve_symbol_as_variable(TRANSITION_PHASE_SYMBOL)
        .and_then(|variable| Numeric::try_coerce(variable.get_as_pax_value()).ok())
        .map(|value| value.to_int() as u64)
        .unwrap_or_default();

    match phase {
        TRANSITION_PHASE_ENTER => sample_axis_transition_track(
            transition,
            transition.enter.as_ref(),
            stack,
            axis_index,
            name,
            initial_override,
        )
        .or_else(|| coerce_axis_transition_starting_value(transition, stack, axis_index, name)),
        TRANSITION_PHASE_EXIT => sample_axis_transition_track(
            transition,
            transition.exit.as_ref(),
            stack,
            axis_index,
            name,
            initial_override,
        )
        .or_else(|| {
            sample_axis_transition_track(
                transition,
                transition.enter.as_ref(),
                stack,
                axis_index,
                name,
                initial_override,
            )
        })
        .or_else(|| coerce_axis_transition_starting_value(transition, stack, axis_index, name)),
        _ => coerce_axis_transition_starting_value(transition, stack, axis_index, name),
    }
}

fn build_axis_transition_property<T: CoercionRules + PropertyValue>(
    name: &str,
    transition: &TransitionDefinition,
    stack: Rc<RuntimePropertiesStackFrame>,
    axis_index: usize,
) -> Property<Option<T>> {
    let mut dependents = Vec::new();
    collect_value_definition_dependencies(
        &ValueDefinition::Transition(transition.clone()),
        &stack,
        &mut dependents,
    );

    let cloned_stack = stack.clone();
    let cloned_transition = transition.clone();
    let property_name = name.to_string();
    let takeover_state = RefCell::new(TransitionTakeoverState::<T>::default());
    Property::computed_with_name(
        move || {
            let generation = transition_generation(&cloned_stack);
            let mut state = borrow_mut!(takeover_state);
            if generation != state.generation {
                let should_take_over = transition_takeover(&cloned_stack)
                    && active_transition_track(&cloned_transition, &cloned_stack)
                        .map(|track| track.interruption == InOutInterruption::Takeover)
                        .unwrap_or(false);
                state.initial_override = should_take_over
                    .then(|| state.last_output.clone())
                    .flatten();
                state.generation = generation;
            }
            let output = sample_axis_transition_property(
                &cloned_transition,
                &cloned_stack,
                axis_index,
                &property_name,
                state.initial_override.as_ref(),
            );
            state.last_output = output.clone();
            output
        },
        &dependents,
        name,
    )
}

fn build_common_property_axis_component_value<T>(
    name: &str,
    value_definition: &ValueDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
    axis_index: usize,
) -> Property<Option<T>>
where
    T: CoercionRules + PropertyValue + ToPaxValue,
{
    let cloned_stack = stack.clone();
    match value_definition {
        ValueDefinition::Timeline(track) => {
            let track = timeline_track_with_base_starting_value(track);
            build_axis_timeline_property(name, &track, cloned_stack, axis_index)
        }
        ValueDefinition::Transition(transition) => {
            let transition = transition_with_base_starting_value(transition);
            build_axis_transition_property(name, &transition, cloned_stack, axis_index)
        }
        ValueDefinition::LiteralValue(_)
        | ValueDefinition::Block(_)
        | ValueDefinition::Gradient(_)
        | ValueDefinition::Expression(_)
        | ValueDefinition::Identifier(_)
        | ValueDefinition::DoubleBinding(_) => {
            let mut dependents = Vec::new();
            collect_value_definition_dependencies(value_definition, stack, &mut dependents);
            let value_definition = value_definition.clone();
            let property_name = name.to_string();
            Property::computed_with_name(
                move || {
                    evaluate_value_definition_to_pax_value(&value_definition, &cloned_stack)
                        .and_then(|value| {
                            coerce_axis_component::<T>(value, axis_index, &property_name)
                                .map_err(|err| {
                                    log::warn!("Failed to resolve multi-axis property: {err}");
                                    err
                                })
                                .ok()
                        })
                        .flatten()
                },
                &dependents,
                name,
            )
        }
        _ => unreachable!("Invalid value definition for {name}"),
    }
}

fn resolve_property<T>(
    name: &str,
    property_columns: &RuntimeResolvedPropertyColumns,
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> Property<Option<T>>
where
    T: CoercionRules + PropertyValue + ToPaxValue,
{
    let Some(entries) = property_columns.get(name) else {
        return Property::default();
    };
    let mut layered_property = Property::default();
    for entry in entries {
        let previous_property = layered_property.clone();
        let source_stack = entry.source_stack.as_ref().unwrap_or(stack);
        let stack_with_base = stack_with_optional_base_or(
            source_stack,
            previous_property.clone(),
            common_property_base_fallback(name),
        );
        let candidate_property = if let Some(axis_index) = entry.axis_index {
            build_common_property_axis_component_value(
                name,
                &entry.value,
                &stack_with_base,
                axis_index,
            )
        } else {
            build_common_property_value(name, &entry.value, &stack_with_base)
        };
        layered_property = apply_runtime_settings_condition(
            name,
            entry.condition.as_ref(),
            source_stack,
            previous_property,
            candidate_property,
        );
    }
    layered_property
}

fn common_property_base_fallback<T>(name: &str) -> T
where
    T: CoercionRules + PropertyValue,
{
    // Optional common properties need the semantic layout fallback, not the Rust
    // type default. For example, unset `y` lays out at 0px while `Size::default()`
    // is 100%.
    let value = match name {
        "x" | "y" | "anchor_x" | "anchor_y" => Some(PaxValue::Size(Size::ZERO())),
        "rotate" | "skew_x" | "skew_y" => Some(PaxValue::Rotation(Rotation::ZERO())),
        "opacity" => Some(PaxValue::Numeric(1.0.into())),
        _ => None,
    };

    value
        .and_then(|value| T::try_coerce(value).ok())
        .unwrap_or_default()
}

fn collect_value_definition_dependencies(
    value_definition: &ValueDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
    dependents: &mut Vec<UntypedProperty>,
) {
    match value_definition {
        ValueDefinition::Expression(info) => {
            for dependency in &info.dependencies {
                if let Some(property) = stack.resolve_symbol_as_erased_property(dependency) {
                    dependents.push(property);
                }
            }
        }
        ValueDefinition::Identifier(identifier) | ValueDefinition::DoubleBinding(identifier) => {
            if let Some(property) = stack.resolve_symbol_as_erased_property(&identifier.name) {
                dependents.push(property);
            }
        }
        ValueDefinition::Block(block) => {
            for element in &block.elements {
                if let SettingElement::Setting(_, value_definition) = element {
                    collect_value_definition_dependencies(value_definition, stack, dependents);
                }
            }
        }
        ValueDefinition::Gradient(gradient) => {
            match &gradient.shape {
                GradientShapeDefinition::Linear { start, end } => {
                    if let Some(start) = start {
                        collect_value_definition_dependencies(start, stack, dependents);
                    }
                    if let Some(end) = end {
                        collect_value_definition_dependencies(end, stack, dependents);
                    }
                }
                GradientShapeDefinition::Radial { start, end, radius } => {
                    collect_value_definition_dependencies(start, stack, dependents);
                    collect_value_definition_dependencies(end, stack, dependents);
                    collect_value_definition_dependencies(radius, stack, dependents);
                }
            }
            for element in &gradient.elements {
                if let GradientElement::Stop(stop) = element {
                    collect_value_definition_dependencies(&stop.color, stack, dependents);
                }
            }
        }
        ValueDefinition::Timeline(track) => {
            if let Some(playhead) = &track.playhead {
                collect_value_definition_dependencies(playhead, stack, dependents);
            } else {
                if let Some(property) = stack.resolve_symbol_as_erased_property("$frames") {
                    dependents.push(property);
                }
                if let Some(property) = stack.resolve_symbol_as_erased_property("$millis") {
                    dependents.push(property);
                }
            }
            if let Some(duration) = &track.duration {
                collect_value_definition_dependencies(duration, stack, dependents);
            }
            if let Some(starting_value) = &track.starting_value {
                collect_value_definition_dependencies(starting_value, stack, dependents);
            }
            for element in &track.elements {
                if let TimelineTrackElement::Keyframe(keyframe) = element {
                    collect_value_definition_dependencies(&keyframe.value, stack, dependents);
                }
            }
        }
        ValueDefinition::Transition(transition) => {
            if let Some(property) = stack.resolve_symbol_as_erased_property(TRANSITION_PHASE_SYMBOL)
            {
                dependents.push(property);
            }
            if let Some(property) =
                stack.resolve_symbol_as_erased_property(TRANSITION_PLAYHEAD_SYMBOL)
            {
                dependents.push(property);
            }
            if let Some(property) =
                stack.resolve_symbol_as_erased_property(TRANSITION_PLAYHEAD_MILLIS_SYMBOL)
            {
                dependents.push(property);
            }
            if let Some(property) =
                stack.resolve_symbol_as_erased_property(TRANSITION_GENERATION_SYMBOL)
            {
                dependents.push(property);
            }
            if let Some(property) =
                stack.resolve_symbol_as_erased_property(TRANSITION_TAKEOVER_SYMBOL)
            {
                dependents.push(property);
            }
            if let Some(starting_value) = &transition.starting_value {
                collect_value_definition_dependencies(starting_value, stack, dependents);
            }
            for track in [&transition.enter, &transition.exit].into_iter().flatten() {
                if let Some(playhead) = &track.playhead {
                    collect_value_definition_dependencies(playhead, stack, dependents);
                }
                if let Some(duration) = &track.duration {
                    collect_value_definition_dependencies(duration, stack, dependents);
                }
                if let Some(starting_value) = &track.starting_value {
                    collect_value_definition_dependencies(starting_value, stack, dependents);
                }
                for element in &track.elements {
                    if let TimelineTrackElement::Keyframe(keyframe) = element {
                        collect_value_definition_dependencies(&keyframe.value, stack, dependents);
                    }
                }
            }
        }
        ValueDefinition::LiteralValue(_)
        | ValueDefinition::EventBindingTarget(_)
        | ValueDefinition::Undefined => {}
    }
}

fn evaluate_literal_block_to_pax_value(
    block: &LiteralBlockDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> Option<PaxValue> {
    let mut values = Vec::new();
    for element in &block.elements {
        if let SettingElement::Setting(token, value_definition) = element {
            let value = evaluate_value_definition_to_pax_value(value_definition, stack)?;
            values.push((token.token_value.clone(), value));
        }
    }
    Some(PaxValue::Object(values))
}

fn gradient_default_start() -> PaxValue {
    PaxValue::Vec(vec![
        PaxValue::Size(Size::Percent(Numeric::F64(0.0))),
        PaxValue::Size(Size::Percent(Numeric::F64(0.0))),
    ])
}

fn gradient_default_end() -> PaxValue {
    PaxValue::Vec(vec![
        PaxValue::Size(Size::Percent(Numeric::F64(100.0))),
        PaxValue::Size(Size::Percent(Numeric::F64(0.0))),
    ])
}

fn evaluate_gradient_to_pax_value(
    gradient: &GradientDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> Option<PaxValue> {
    let stops = gradient
        .elements
        .iter()
        .filter_map(|element| match element {
            GradientElement::Stop(stop) => {
                let color = evaluate_value_definition_to_pax_value(&stop.color, stack)?;
                Some(PaxValue::Enum(Box::new((
                    "GradientStop".to_string(),
                    "get".to_string(),
                    vec![color, PaxValue::Size(stop.position.clone())],
                ))))
            }
            GradientElement::Comment(_) => None,
        })
        .collect::<Vec<_>>();

    if stops.len() < 2 {
        log::warn!("@gradient requires at least two stops");
        return None;
    }

    let stops = PaxValue::Vec(stops);
    let (variant, args) = match &gradient.shape {
        GradientShapeDefinition::Linear { start, end } => {
            let start = start
                .as_ref()
                .and_then(|value| evaluate_value_definition_to_pax_value(value, stack))
                .unwrap_or_else(gradient_default_start);
            let end = end
                .as_ref()
                .and_then(|value| evaluate_value_definition_to_pax_value(value, stack))
                .unwrap_or_else(gradient_default_end);
            ("linearGradient", vec![start, end, stops])
        }
        GradientShapeDefinition::Radial { start, end, radius } => {
            let start = evaluate_value_definition_to_pax_value(start, stack)?;
            let end = evaluate_value_definition_to_pax_value(end, stack)?;
            let radius = evaluate_value_definition_to_pax_value(radius, stack)?;
            ("radialGradient", vec![start, end, stops, radius])
        }
    };

    Some(PaxValue::Enum(Box::new((
        "Fill".to_string(),
        variant.to_string(),
        args,
    ))))
}

fn resolve_literal_value(value: PaxValue) -> PaxValue {
    match value {
        PaxValue::Enum(contents) => {
            let (scope, name, args) = *contents;
            let resolved_args: Vec<_> = args.into_iter().map(resolve_literal_value).collect();
            if Functions::has_function(&scope, &name) {
                call_function(scope.clone(), name.clone(), resolved_args.clone()).unwrap_or_else(
                    |err| {
                        log::warn!(
                            "Failed to evaluate literal helper {}::{}: {}",
                            scope,
                            name,
                            err
                        );
                        PaxValue::Enum(Box::new((scope, name, resolved_args)))
                    },
                )
            } else {
                PaxValue::Enum(Box::new((scope, name, resolved_args)))
            }
        }
        PaxValue::Vec(values) => {
            PaxValue::Vec(values.into_iter().map(resolve_literal_value).collect())
        }
        PaxValue::Object(values) => PaxValue::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, resolve_literal_value(value)))
                .collect(),
        ),
        PaxValue::Option(value) => PaxValue::Option(Box::new(value.map(resolve_literal_value))),
        PaxValue::Range(start, end) => PaxValue::Range(
            Box::new(resolve_literal_value(*start)),
            Box::new(resolve_literal_value(*end)),
        ),
        value => value,
    }
}

fn evaluate_value_definition_to_pax_value(
    value_definition: &ValueDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> Option<PaxValue> {
    match value_definition {
        ValueDefinition::LiteralValue(value) => Some(resolve_literal_value(value.clone())),
        ValueDefinition::Block(block) => evaluate_literal_block_to_pax_value(block, stack),
        ValueDefinition::Gradient(gradient) => evaluate_gradient_to_pax_value(gradient, stack),
        ValueDefinition::Expression(info) => info.expression.compute(stack.clone()).ok(),
        ValueDefinition::Identifier(identifier) | ValueDefinition::DoubleBinding(identifier) => {
            stack
                .resolve_symbol_as_variable(&identifier.name)
                .map(|variable| variable.get_as_pax_value())
        }
        ValueDefinition::Timeline(_) | ValueDefinition::Transition(_) => None,
        ValueDefinition::EventBindingTarget(_) | ValueDefinition::Undefined => None,
    }
}

fn evaluate_settings_condition_expression(
    condition: &ExpressionInfo,
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> bool {
    match condition.expression.compute(stack.clone()) {
        Ok(value) => bool::try_coerce(value).unwrap_or_else(|err| {
            log::warn!(
                "Settings conditional `{}` did not evaluate to bool: {}",
                condition,
                err
            );
            false
        }),
        Err(err) => {
            log::warn!(
                "Failed to compute settings conditional `{}`: {}",
                condition,
                err
            );
            false
        }
    }
}

fn settings_condition_is_active(
    condition: &RuntimeSettingsCondition,
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> bool {
    condition
        .negative
        .iter()
        .all(|expression| !evaluate_settings_condition_expression(expression, stack))
        && condition
            .positive
            .iter()
            .all(|expression| evaluate_settings_condition_expression(expression, stack))
}

fn collect_settings_condition_dependencies(
    condition: &RuntimeSettingsCondition,
    stack: &Rc<RuntimePropertiesStackFrame>,
    dependents: &mut Vec<UntypedProperty>,
) {
    for expression in condition.positive.iter().chain(condition.negative.iter()) {
        for dependency in &expression.dependencies {
            if let Some(property) = stack.resolve_symbol_as_erased_property(dependency) {
                dependents.push(property);
            }
        }
    }
}

pub fn apply_runtime_settings_condition<T>(
    name: &str,
    condition: Option<&RuntimeSettingsCondition>,
    stack: &Rc<RuntimePropertiesStackFrame>,
    previous_property: Property<T>,
    candidate_property: Property<T>,
) -> Property<T>
where
    T: PropertyValue,
{
    let Some(condition) = condition.cloned() else {
        return candidate_property;
    };

    let mut dependents = vec![previous_property.untyped(), candidate_property.untyped()];
    collect_settings_condition_dependencies(&condition, stack, &mut dependents);
    let cloned_stack = stack.clone();
    Property::computed_with_name(
        move || {
            if settings_condition_is_active(&condition, &cloned_stack) {
                candidate_property.get()
            } else {
                previous_property.get()
            }
        },
        &dependents,
        name,
    )
}

fn easing_curve_from_name(name: Option<&str>) -> EasingCurve {
    match name {
        Some("Linear") | None => EasingCurve::Linear,
        Some("Hold") => EasingCurve::Hold,
        Some("InQuad") => EasingCurve::InQuad,
        Some("OutQuad") => EasingCurve::OutQuad,
        Some("InOutQuad") => EasingCurve::InOutQuad,
        Some("InBack") => EasingCurve::InBack,
        Some("OutBack") => EasingCurve::OutBack,
        Some("InOutBack") => EasingCurve::InOutBack,
        Some(other) => {
            log::warn!("Unknown easing curve '{}', defaulting to Linear", other);
            EasingCurve::Linear
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TimelineClockUnit {
    Frames,
    Millis,
}

#[derive(Clone, Copy, Debug)]
struct TimelineScale {
    unit: TimelineClockUnit,
    total: f64,
}

fn duration_to_unit(duration: Duration, unit: TimelineClockUnit) -> f64 {
    match unit {
        TimelineClockUnit::Frames => duration.as_frames_f64(),
        TimelineClockUnit::Millis => duration.as_milliseconds_f64(),
    }
}

fn duration_unit(duration: Duration) -> TimelineClockUnit {
    if duration.is_frame_based() {
        TimelineClockUnit::Frames
    } else {
        TimelineClockUnit::Millis
    }
}

pub(crate) fn evaluate_timeline_duration(
    value_definition: &ValueDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> Option<Duration> {
    evaluate_value_definition_to_pax_value(value_definition, stack)
        .and_then(|value| Duration::try_coerce(value).ok())
}

fn timeline_duration(
    track: &TimelineTrackDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> Option<Duration> {
    track
        .duration
        .as_ref()
        .and_then(|duration| evaluate_timeline_duration(duration, stack))
}

fn timeline_scale(
    track: &TimelineTrackDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> TimelineScale {
    if let Some(duration) = timeline_duration(track, stack) {
        let unit = duration_unit(duration);
        return TimelineScale {
            unit,
            total: duration_to_unit(duration, unit).max(0.0),
        };
    }

    let mut max_frame: f64 = 0.0;
    let mut max_millis: f64 = 0.0;
    let mut uses_percent_markers = false;
    let mut uses_millis_markers = false;

    for keyframe in track.keyframes() {
        match keyframe.marker {
            TimelineMarker::Frame(frame) => max_frame = max_frame.max(frame as f64),
            TimelineMarker::Duration(duration) => {
                if duration.is_frame_based() {
                    max_frame = max_frame.max(duration.as_frames_f64());
                } else {
                    uses_millis_markers = true;
                    max_millis = max_millis.max(duration.as_milliseconds_f64());
                }
            }
            TimelineMarker::Percent(_) => uses_percent_markers = true,
        }
    }

    if uses_millis_markers {
        let total = if uses_percent_markers {
            max_millis.max(100.0)
        } else {
            max_millis
        };
        TimelineScale {
            unit: TimelineClockUnit::Millis,
            total,
        }
    } else {
        let total = if uses_percent_markers {
            max_frame.max(100.0)
        } else {
            max_frame
        };
        TimelineScale {
            unit: TimelineClockUnit::Frames,
            total,
        }
    }
}

fn timeline_marker_to_position(marker: &TimelineMarker, scale: TimelineScale) -> f64 {
    match marker {
        TimelineMarker::Frame(frame) => {
            duration_to_unit(Duration::Frames((*frame).into()), scale.unit)
        }
        TimelineMarker::Duration(duration) => duration_to_unit(*duration, scale.unit),
        TimelineMarker::Percent(percent) => scale.total * (*percent / 100.0),
    }
}

fn timeline_value_passes_coercion<T: CoercionRules>(value: &PaxValue) -> bool {
    T::try_coerce(resolve_literal_value(value.clone())).is_ok()
}

fn evaluate_valid_timeline_value(
    value_definition: &ValueDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
    is_valid_value: fn(&PaxValue) -> bool,
) -> Option<PaxValue> {
    let value = evaluate_value_definition_to_pax_value(value_definition, stack)?;
    is_valid_value(&value).then_some(value)
}

fn loop_target_value(
    track: &TimelineTrackDefinition,
    first_keyframe: &ResolvedTimelineKeyframe,
    stack: &Rc<RuntimePropertiesStackFrame>,
    is_valid_value: fn(&PaxValue) -> bool,
) -> PaxValue {
    if first_keyframe.frame == 0.0 {
        first_keyframe.value.clone()
    } else {
        track
            .starting_value
            .as_ref()
            .and_then(|value| evaluate_valid_timeline_value(value, stack, is_valid_value))
            .unwrap_or_else(|| first_keyframe.value.clone())
    }
}

struct ResolvedTimelineKeyframe {
    frame: f64,
    source_order: usize,
    value: PaxValue,
    easing: Option<String>,
}

enum TimelineSample {
    Value {
        value: PaxValue,
        initial: bool,
    },
    Interpolated {
        current: PaxValue,
        next: PaxValue,
        easing: Option<String>,
        progress: f64,
        initial: bool,
    },
}

fn sample_timeline_playhead(
    track: &TimelineTrackDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
    scale: TimelineScale,
) -> f64 {
    let raw_playhead = track
        .playhead
        .as_ref()
        .and_then(|playhead| evaluate_value_definition_to_pax_value(playhead, stack))
        .and_then(|value| timeline_playhead_value_to_position(value, scale.unit))
        .or_else(|| {
            let symbol = match scale.unit {
                TimelineClockUnit::Frames => "$frames",
                TimelineClockUnit::Millis => "$millis",
            };
            stack
                .resolve_symbol_as_variable(symbol)
                .and_then(|variable| {
                    Numeric::try_coerce(variable.get_as_pax_value())
                        .ok()
                        .map(|value| value.to_float())
                })
        })
        .unwrap_or_default();

    let repeat = track.repeat.unwrap_or(true);
    if !repeat {
        return raw_playhead.clamp(0.0, scale.total.max(0.0));
    }

    let cycle_len = match scale.unit {
        TimelineClockUnit::Frames => (scale.total + 1.0).max(1.0),
        TimelineClockUnit::Millis => scale.total.max(1.0),
    };
    raw_playhead.rem_euclid(cycle_len)
}

fn timeline_playhead_value_to_position(value: PaxValue, unit: TimelineClockUnit) -> Option<f64> {
    if let PaxValue::Duration(duration) = value {
        return Some(duration_to_unit(duration, unit));
    }
    Numeric::try_coerce(value)
        .ok()
        .map(|value| value.to_float())
}

fn resolve_timeline_sample(
    track: &TimelineTrackDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
    is_valid_value: fn(&PaxValue) -> bool,
) -> Option<TimelineSample> {
    let scale = timeline_scale(track, stack);
    let repeat = track.repeat.unwrap_or(true);
    let sample_frame = sample_timeline_playhead(track, stack, scale);

    let mut resolved_keyframes: Vec<ResolvedTimelineKeyframe> = track
        .keyframes()
        .enumerate()
        .filter_map(|(source_order, keyframe): (usize, &TimelineKeyframe)| {
            let value = evaluate_valid_timeline_value(&keyframe.value, stack, is_valid_value)?;
            Some(ResolvedTimelineKeyframe {
                frame: timeline_marker_to_position(&keyframe.marker, scale),
                source_order,
                value,
                easing: keyframe
                    .easing
                    .as_ref()
                    .map(|token| token.token_value.clone()),
            })
        })
        .collect();

    resolved_keyframes.sort_unstable_by(|lhs, rhs| {
        lhs.frame
            .partial_cmp(&rhs.frame)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| lhs.source_order.cmp(&rhs.source_order))
    });

    let first_keyframe = resolved_keyframes.first()?;

    if sample_frame < first_keyframe.frame {
        return Some(TimelineSample::Value {
            value: track
                .starting_value
                .as_ref()
                .and_then(|value| evaluate_valid_timeline_value(value, stack, is_valid_value))
                .unwrap_or_else(|| first_keyframe.value.clone()),
            initial: true,
        });
    }

    for (segment_index, keyframes) in resolved_keyframes.windows(2).enumerate() {
        let current = &keyframes[0];
        let next = &keyframes[1];
        if sample_frame <= next.frame {
            if sample_frame <= current.frame {
                return Some(TimelineSample::Value {
                    value: current.value.clone(),
                    initial: segment_index == 0,
                });
            }
            let span = next.frame - current.frame;
            if span <= f64::EPSILON {
                return Some(TimelineSample::Value {
                    value: next.value.clone(),
                    initial: false,
                });
            }
            let progress = (sample_frame - current.frame) / span;
            return Some(TimelineSample::Interpolated {
                current: current.value.clone(),
                next: next.value.clone(),
                easing: current.easing.clone(),
                progress,
                initial: segment_index == 0,
            });
        }
    }

    let last_keyframe = resolved_keyframes.last()?;
    if sample_frame <= last_keyframe.frame || !repeat || scale.total <= last_keyframe.frame {
        return Some(TimelineSample::Value {
            value: last_keyframe.value.clone(),
            initial: resolved_keyframes.len() == 1 && sample_frame <= first_keyframe.frame,
        });
    }

    let loop_target = loop_target_value(track, first_keyframe, stack, is_valid_value);
    let span = scale.total - last_keyframe.frame;
    if span <= f64::EPSILON {
        return Some(TimelineSample::Value {
            value: loop_target,
            initial: false,
        });
    }
    let progress = (sample_frame - last_keyframe.frame) / span;
    Some(TimelineSample::Interpolated {
        current: last_keyframe.value.clone(),
        next: loop_target,
        easing: last_keyframe.easing.clone(),
        progress,
        initial: false,
    })
}

fn sample_timeline_track<T: CoercionRules + PropertyValue>(
    track: &TimelineTrackDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
    initial_override: Option<&T>,
) -> Option<T> {
    match resolve_timeline_sample(track, stack, timeline_value_passes_coercion::<T>)? {
        TimelineSample::Value { value, initial } => {
            if initial {
                if let Some(value) = initial_override {
                    return Some(value.clone());
                }
            }
            T::try_coerce(resolve_literal_value(value)).ok()
        }
        TimelineSample::Interpolated {
            current,
            next,
            easing,
            progress,
            initial,
        } => {
            let current = if initial {
                initial_override.cloned()
            } else {
                None
            }
            .or_else(|| T::try_coerce(resolve_literal_value(current)).ok())?;
            let next = T::try_coerce(resolve_literal_value(next)).ok()?;
            let curve = easing_curve_from_name(easing.as_deref());
            Some(curve.interpolate(&current, &next, progress))
        }
    }
}

pub fn build_timeline_property<T: CoercionRules + PropertyValue>(
    name: &str,
    track: &TimelineTrackDefinition,
    stack: Rc<RuntimePropertiesStackFrame>,
) -> Property<T> {
    let mut dependents = Vec::new();
    if let Some(playhead) = &track.playhead {
        collect_value_definition_dependencies(playhead, &stack, &mut dependents);
    } else {
        if let Some(property) = stack.resolve_symbol_as_erased_property("$frames") {
            dependents.push(property);
        }
        if let Some(property) = stack.resolve_symbol_as_erased_property("$millis") {
            dependents.push(property);
        }
    }
    if let Some(duration) = &track.duration {
        collect_value_definition_dependencies(duration, &stack, &mut dependents);
    }
    if let Some(starting_value) = &track.starting_value {
        collect_value_definition_dependencies(starting_value, &stack, &mut dependents);
    }
    for element in &track.elements {
        if let TimelineTrackElement::Keyframe(keyframe) = element {
            collect_value_definition_dependencies(&keyframe.value, &stack, &mut dependents);
        }
    }

    let cloned_stack = stack.clone();
    let cloned_track = track.clone();
    Property::computed_with_name(
        move || sample_timeline_track(&cloned_track, &cloned_stack, None).unwrap_or_default(),
        &dependents,
        name,
    )
}

fn coerce_transition_starting_value<T: CoercionRules + PropertyValue>(
    transition: &TransitionDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> Option<T> {
    transition
        .starting_value
        .as_ref()
        .and_then(|value| evaluate_value_definition_to_pax_value(value, stack))
        .and_then(|value| T::try_coerce(value).ok())
}

fn sample_transition_track<T: CoercionRules + PropertyValue>(
    transition: &TransitionDefinition,
    track: Option<&TimelineTrackDefinition>,
    stack: &Rc<RuntimePropertiesStackFrame>,
    initial_override: Option<&T>,
) -> Option<T> {
    let mut track = track?.clone();
    if track.starting_value.is_none() {
        track.starting_value = transition.starting_value.clone();
    }
    align_transition_track_playhead_to_duration(&mut track, stack);
    sample_timeline_track(&track, stack, initial_override)
}

fn align_transition_track_playhead_to_duration(
    track: &mut TimelineTrackDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
) {
    let Some(ValueDefinition::Identifier(identifier)) = track.playhead.as_deref() else {
        return;
    };
    if identifier.name != TRANSITION_PLAYHEAD_SYMBOL
        && identifier.name != TRANSITION_PLAYHEAD_MILLIS_SYMBOL
    {
        return;
    }

    let Some(duration) = timeline_duration(track, stack) else {
        return;
    };
    let playhead_symbol = if duration.is_frame_based() {
        TRANSITION_PLAYHEAD_SYMBOL
    } else {
        TRANSITION_PLAYHEAD_MILLIS_SYMBOL
    };
    if identifier.name != playhead_symbol {
        track.playhead = Some(Box::new(ValueDefinition::Identifier(PaxIdentifier::new(
            playhead_symbol,
        ))));
    }
}

fn sample_transition_property<T: CoercionRules + PropertyValue>(
    transition: &TransitionDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
    initial_override: Option<&T>,
) -> Option<T> {
    let phase = stack
        .resolve_symbol_as_variable(TRANSITION_PHASE_SYMBOL)
        .and_then(|variable| Numeric::try_coerce(variable.get_as_pax_value()).ok())
        .map(|value| value.to_int() as u64)
        .unwrap_or_default();

    match phase {
        TRANSITION_PHASE_ENTER => sample_transition_track(
            transition,
            transition.enter.as_ref(),
            stack,
            initial_override,
        )
        .or_else(|| coerce_transition_starting_value(transition, stack)),
        TRANSITION_PHASE_EXIT => sample_transition_track(
            transition,
            transition.exit.as_ref(),
            stack,
            initial_override,
        )
        .or_else(|| {
            sample_transition_track(
                transition,
                transition.enter.as_ref(),
                stack,
                initial_override,
            )
        })
        .or_else(|| coerce_transition_starting_value(transition, stack)),
        _ => coerce_transition_starting_value(transition, stack),
    }
}

#[derive(Default)]
struct TransitionTakeoverState<T> {
    generation: u64,
    last_output: Option<T>,
    initial_override: Option<T>,
}

fn transition_generation(stack: &Rc<RuntimePropertiesStackFrame>) -> u64 {
    stack
        .resolve_symbol_as_variable(TRANSITION_GENERATION_SYMBOL)
        .and_then(|variable| Numeric::try_coerce(variable.get_as_pax_value()).ok())
        .map(|value| value.to_int() as u64)
        .unwrap_or_default()
}

fn transition_takeover(stack: &Rc<RuntimePropertiesStackFrame>) -> bool {
    stack
        .resolve_symbol_as_variable(TRANSITION_TAKEOVER_SYMBOL)
        .and_then(|variable| bool::try_coerce(variable.get_as_pax_value()).ok())
        .unwrap_or(false)
}

fn active_transition_track<'a>(
    transition: &'a TransitionDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> Option<&'a TimelineTrackDefinition> {
    let phase = stack
        .resolve_symbol_as_variable(TRANSITION_PHASE_SYMBOL)
        .and_then(|variable| Numeric::try_coerce(variable.get_as_pax_value()).ok())
        .map(|value| value.to_int() as u64)
        .unwrap_or_default();
    match phase {
        TRANSITION_PHASE_ENTER => transition.enter.as_ref(),
        TRANSITION_PHASE_EXIT => transition.exit.as_ref().or(transition.enter.as_ref()),
        _ => None,
    }
}

pub fn build_transition_property<T: CoercionRules + PropertyValue>(
    name: &str,
    transition: &TransitionDefinition,
    stack: Rc<RuntimePropertiesStackFrame>,
) -> Property<T> {
    let mut dependents = Vec::new();
    collect_value_definition_dependencies(
        &ValueDefinition::Transition(transition.clone()),
        &stack,
        &mut dependents,
    );

    let cloned_stack = stack.clone();
    let cloned_transition = transition.clone();
    let takeover_state = RefCell::new(TransitionTakeoverState::<T>::default());
    Property::computed_with_name(
        move || {
            let generation = transition_generation(&cloned_stack);
            let mut state = borrow_mut!(takeover_state);
            if generation != state.generation {
                let should_take_over = transition_takeover(&cloned_stack)
                    && active_transition_track(&cloned_transition, &cloned_stack)
                        .map(|track| track.interruption == InOutInterruption::Takeover)
                        .unwrap_or(false);
                state.initial_override = should_take_over
                    .then(|| state.last_output.clone())
                    .flatten();
                state.generation = generation;
            }
            let output = sample_transition_property(
                &cloned_transition,
                &cloned_stack,
                state.initial_override.as_ref(),
            )
            .unwrap_or_default();
            state.last_output = Some(output.clone());
            output
        },
        &dependents,
        name,
    )
}

/// Returns a stack frame with `$base` bound to the supplied previous-layer
/// property value.
pub fn stack_with_base<T>(
    stack: &Rc<RuntimePropertiesStackFrame>,
    base_property: Property<T>,
) -> Rc<RuntimePropertiesStackFrame>
where
    T: PropertyValue + ToPaxValue,
{
    stack.push(HashMap::from([(
        BASE_SYMBOL.to_string(),
        Variable::new_from_typed_property(base_property),
    )]))
}

/// Returns a stack frame with `$base` bound to an optional previous common
/// property value, exposing `T::default()` when the previous layer is `None`.
pub fn stack_with_optional_base<T>(
    stack: &Rc<RuntimePropertiesStackFrame>,
    base_property: Property<Option<T>>,
) -> Rc<RuntimePropertiesStackFrame>
where
    T: PropertyValue + ToPaxValue,
{
    stack_with_optional_base_or(stack, base_property, T::default())
}

fn stack_with_optional_base_or<T>(
    stack: &Rc<RuntimePropertiesStackFrame>,
    base_property: Property<Option<T>>,
    fallback: T,
) -> Rc<RuntimePropertiesStackFrame>
where
    T: PropertyValue + ToPaxValue,
{
    let dependency = base_property.untyped();
    let cloned_base = base_property.clone();
    let resolved_base = Property::computed_with_name(
        move || cloned_base.get().unwrap_or_else(|| fallback.clone()),
        &[dependency],
        BASE_SYMBOL,
    );
    stack_with_base(stack, resolved_base)
}

fn base_identifier_value() -> Box<ValueDefinition> {
    Box::new(ValueDefinition::Identifier(PaxIdentifier::new(BASE_SYMBOL)))
}

fn timeline_track_with_base_starting_value(
    track: &TimelineTrackDefinition,
) -> TimelineTrackDefinition {
    let mut track = track.clone();
    if track.starting_value.is_none() {
        track.starting_value = Some(base_identifier_value());
    }
    track
}

fn transition_with_base_starting_value(transition: &TransitionDefinition) -> TransitionDefinition {
    let mut transition = transition.clone();
    if transition.starting_value.is_none() {
        transition.starting_value = Some(base_identifier_value());
    }
    if let Some(track) = transition.enter.as_mut() {
        if track.starting_value.is_none() {
            track.starting_value = Some(base_identifier_value());
        }
    }
    if let Some(track) = transition.exit.as_mut() {
        if track.starting_value.is_none() {
            track.starting_value = Some(base_identifier_value());
        }
    }
    transition
}

/// Builds a typed component property from a single value definition. Callers
/// that apply layered settings should bind `$base` before calling this helper.
pub fn build_component_property<T>(
    name: &str,
    value_definition: &ValueDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
    timeline_stack: Rc<RuntimePropertiesStackFrame>,
    build_block: fn(&LiteralBlockDefinition, Rc<RuntimePropertiesStackFrame>) -> Property<T>,
) -> Property<T>
where
    T: CoercionRules + PropertyValue + ToPaxValue,
{
    if let Some(property) = try_typed_property_binding(name, value_definition, stack) {
        return property;
    }
    match value_definition {
        ValueDefinition::LiteralValue(lv) => {
            let value = T::try_coerce(resolve_literal_value(lv.clone())).unwrap_or_else(|err| {
                log::warn!(
                    "Failed to coerce new value for property {name}. Error: {:?}",
                    err
                );
                Default::default()
            });
            Property::new_with_name(value, name)
        }
        ValueDefinition::DoubleBinding(identifier) => {
            if let Some(untyped_property) =
                stack.resolve_symbol_as_erased_property(&identifier.name)
            {
                Property::new_from_untyped(untyped_property.clone())
            } else {
                log::warn!("Failed to resolve identifier: {}", &identifier.name);
                Property::new_with_name(Default::default(), name)
            }
        }
        ValueDefinition::Identifier(ident) => {
            if let Some(variable) = stack.resolve_symbol_as_variable(&ident.name) {
                let identifier_name = ident.name.clone();
                let property_name = name.to_string();
                let untyped = variable.get_untyped_property().clone();
                let cloned_variable = variable.clone();
                Property::computed_with_name(
                    move || {
                        let new_value = cloned_variable.get_as_pax_value();
                        T::try_coerce(new_value).unwrap_or_else(|err| {
                            log::warn!(
                                "Failed to coerce new value for property {property_name}. Error: {:?}",
                                err
                            );
                            Default::default()
                        })
                    },
                    &[untyped],
                    &identifier_name,
                )
            } else {
                log::warn!("Failed to resolve symbol {}", ident.name);
                Property::new_with_name(Default::default(), name)
            }
        }
        ValueDefinition::Expression(info) => {
            let mut dependents = vec![];
            for dependency in &info.dependencies {
                if let Some(p) = stack.resolve_symbol_as_erased_property(dependency) {
                    dependents.push(p);
                } else {
                    log::warn!("Failed to resolve symbol {}", dependency);
                }
            }
            let cloned_stack = stack.clone();
            let cloned_ast = info.expression.clone();
            let expression_label = cloned_ast.to_string();
            let property_name = name.to_string();
            Property::computed_with_name(
                move || {
                    let new_value = cloned_ast
                        .compute(cloned_stack.clone())
                        .unwrap_or_else(|_| {
                            log::warn!("Failed to compute expr: {}", expression_label);
                            Default::default()
                        });
                    T::try_coerce(new_value).unwrap_or_else(|err| {
                        log::warn!(
                            "Failed to coerce new value for property {property_name}. Error: {:?}",
                            err
                        );
                        Default::default()
                    })
                },
                &dependents,
                name,
            )
        }
        ValueDefinition::Gradient(_) => {
            let mut dependents = Vec::new();
            collect_value_definition_dependencies(value_definition, stack, &mut dependents);
            let value_definition = value_definition.clone();
            let cloned_stack = stack.clone();
            let property_name = name.to_string();
            Property::computed_with_name(
                move || {
                    evaluate_value_definition_to_pax_value(&value_definition, &cloned_stack)
                        .and_then(|new_value| {
                            T::try_coerce(new_value)
                                .map_err(|err| {
                                    log::warn!(
                                        "Failed to coerce new value for property {property_name}. Error: {:?}",
                                        err
                                    );
                                    err
                                })
                                .ok()
                        })
                        .unwrap_or_default()
                },
                &dependents,
                name,
            )
        }
        ValueDefinition::Timeline(track) => {
            let track = timeline_track_with_base_starting_value(track);
            build_timeline_property(name, &track, timeline_stack)
        }
        ValueDefinition::Transition(transition) => {
            let transition = transition_with_base_starting_value(transition);
            build_transition_property(name, &transition, timeline_stack)
        }
        ValueDefinition::Block(block) => build_block(block, stack.clone()),
        _ => unreachable!("Invalid value definition for {name}"),
    }
}

/// Replaces a typed component property with the value produced from a single
/// value definition.
pub fn apply_component_property<T>(
    property: &mut Property<T>,
    name: &str,
    value_definition: &ValueDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
    timeline_stack: Rc<RuntimePropertiesStackFrame>,
    build_block: fn(&LiteralBlockDefinition, Rc<RuntimePropertiesStackFrame>) -> Property<T>,
) where
    T: CoercionRules + PropertyValue + ToPaxValue,
{
    let resolved_property =
        build_component_property(name, value_definition, stack, timeline_stack, build_block);
    if matches!(value_definition, ValueDefinition::DoubleBinding(_)) {
        *property = resolved_property;
    } else {
        property.replace_with(resolved_property);
    }
}

#[cfg(test)]
mod timeline_tests {
    use super::build_timeline_property;
    use super::build_transition_property;
    use super::evaluate_value_definition_to_pax_value;
    use super::BASE_SYMBOL;
    use crate::RuntimePropertiesStackFrame;
    use pax_language::parse_pax_expression;
    use pax_manifest::cartridge_generation::{
        TRANSITION_GENERATION_SYMBOL, TRANSITION_PHASE_ENTER, TRANSITION_PHASE_EXIT,
        TRANSITION_PHASE_SYMBOL, TRANSITION_PLAYHEAD_MILLIS_SYMBOL, TRANSITION_PLAYHEAD_SYMBOL,
        TRANSITION_TAKEOVER_SYMBOL,
    };
    use pax_manifest::{
        ExpressionInfo, GradientDefinition, GradientElement, GradientShapeDefinition,
        GradientStopDefinition, InOutInterruption, PaxIdentifier, TimelineKeyframe, TimelineMarker,
        TimelineTrackDefinition, TimelineTrackElement, Token, TransitionDefinition,
        ValueDefinition,
    };
    use pax_runtime_api::pax_value::CoercionRules;
    use pax_runtime_api::{Color, Duration, Fill, Numeric, PaxValue, Property, Size, Variable};
    use std::collections::HashMap;
    use std::rc::Rc;
    use std::sync::Arc;

    fn build_stack_with_clocks(
        elapsed_frames: &Property<u64>,
        elapsed_millis: &Property<u64>,
    ) -> Rc<RuntimePropertiesStackFrame> {
        let scope: HashMap<String, Variable> = vec![
            (
                "$frames".to_string(),
                Variable::new_from_typed_property(elapsed_frames.clone()),
            ),
            (
                "$millis".to_string(),
                Variable::new_from_typed_property(elapsed_millis.clone()),
            ),
        ]
        .into_iter()
        .collect();
        RuntimePropertiesStackFrame::new(scope)
    }

    fn build_stack(elapsed_frames: &Property<u64>) -> Rc<RuntimePropertiesStackFrame> {
        let elapsed_millis = Property::new(0_u64);
        build_stack_with_clocks(elapsed_frames, &elapsed_millis)
    }

    fn expression(raw: &str) -> ValueDefinition {
        pax_runtime_api::pax_value::functions::Functions::register_all_functions();
        ValueDefinition::Expression(ExpressionInfo::new(parse_pax_expression(raw).unwrap()))
    }

    #[test]
    fn literal_enum_helpers_evaluate_before_coercion() {
        pax_runtime_api::pax_value::functions::register_function(
            "LiteralHelper".to_string(),
            "make_object".to_string(),
            Arc::new(|mut args| {
                Ok(PaxValue::Object(vec![(
                    "value".to_string(),
                    args.remove(0),
                )]))
            }),
        );
        let elapsed_frames = Property::new(0_u64);
        let stack = build_stack(&elapsed_frames);
        let literal = ValueDefinition::LiteralValue(PaxValue::Enum(Box::new((
            "LiteralHelper".to_string(),
            "make_object".to_string(),
            vec![PaxValue::Numeric(7.0.into())],
        ))));

        assert_eq!(
            evaluate_value_definition_to_pax_value(&literal, &stack),
            Some(PaxValue::Object(vec![(
                "value".to_string(),
                PaxValue::Numeric(7.0.into()),
            )]))
        );
    }

    #[test]
    fn gradient_values_evaluate_to_default_linear_fill() {
        let elapsed_frames = Property::new(0_u64);
        let stack = build_stack(&elapsed_frames);
        let gradient = ValueDefinition::Gradient(GradientDefinition {
            shape: GradientShapeDefinition::default(),
            elements: vec![
                GradientElement::Stop(GradientStopDefinition {
                    position: Size::Percent(Numeric::F64(0.0)),
                    color: ValueDefinition::LiteralValue(PaxValue::Color(Box::new(Color::RED))),
                }),
                GradientElement::Stop(GradientStopDefinition {
                    position: Size::Percent(Numeric::F64(100.0)),
                    color: ValueDefinition::LiteralValue(PaxValue::Color(Box::new(Color::BLUE))),
                }),
            ],
        });

        let value = evaluate_value_definition_to_pax_value(&gradient, &stack)
            .expect("gradient should evaluate");
        let fill = Fill::try_coerce(value).expect("gradient should coerce to Fill");
        let Fill::LinearGradient(linear) = fill else {
            panic!("expected linear gradient fill");
        };

        assert!((linear.start.0.expect_percent() - 0.0).abs() < 0.0001);
        assert!((linear.start.1.expect_percent() - 0.0).abs() < 0.0001);
        assert!((linear.end.0.expect_percent() - 1.0).abs() < 0.0001);
        assert!((linear.end.1.expect_percent() - 0.0).abs() < 0.0001);
        assert_eq!(linear.stops.len(), 2);
        assert_eq!(linear.stops[0].color, Color::RED);
        assert_eq!(linear.stops[1].color, Color::BLUE);
    }

    #[test]
    fn timeline_property_loops_over_declared_frame_range() {
        let elapsed_frames = Property::new(0_u64);
        let stack = build_stack(&elapsed_frames);
        let track = TimelineTrackDefinition {
            elements: vec![
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(0),
                    value: ValueDefinition::LiteralValue(PaxValue::Numeric(0.0.into())),
                    easing: Some(Token::new_without_location("Linear".to_string())),
                }),
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(100),
                    value: ValueDefinition::LiteralValue(PaxValue::Numeric(100.0.into())),
                    easing: None,
                }),
            ],
            playhead: None,
            duration: None,
            repeat: Some(true),
            starting_value: None,
            interruption: Default::default(),
            use_local_property_scope: false,
        };
        let property = build_timeline_property::<f64>("progress", &track, stack);

        assert_eq!(property.get(), 0.0);
        elapsed_frames.set(50);
        assert_eq!(property.get(), 50.0);
        elapsed_frames.set(100);
        assert_eq!(property.get(), 100.0);
        elapsed_frames.set(101);
        assert_eq!(property.get(), 0.0);
    }

    #[test]
    fn timeline_property_samples_millisecond_duration_from_elapsed_millis() {
        let elapsed_frames = Property::new(0_u64);
        let elapsed_millis = Property::new(0_u64);
        let stack = build_stack_with_clocks(&elapsed_frames, &elapsed_millis);
        let track = TimelineTrackDefinition {
            elements: vec![
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Percent(0.0),
                    value: ValueDefinition::LiteralValue(PaxValue::Numeric(0.0.into())),
                    easing: Some(Token::new_without_location("Linear".to_string())),
                }),
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Percent(100.0),
                    value: ValueDefinition::LiteralValue(PaxValue::Numeric(100.0.into())),
                    easing: None,
                }),
            ],
            playhead: None,
            duration: Some(Box::new(ValueDefinition::LiteralValue(PaxValue::Duration(
                pax_runtime_api::Duration::Milliseconds(1000.into()),
            )))),
            repeat: Some(false),
            starting_value: None,
            interruption: Default::default(),
            use_local_property_scope: false,
        };
        let property = build_timeline_property::<f64>("progress", &track, stack);

        assert_eq!(property.get(), 0.0);
        elapsed_frames.set(30);
        assert_eq!(property.get(), 0.0);
        elapsed_millis.set(500);
        assert_eq!(property.get(), 50.0);
        elapsed_millis.set(1000);
        assert_eq!(property.get(), 100.0);
    }

    #[test]
    fn timeline_property_holds_starting_value_before_first_keyframe() {
        let elapsed_frames = Property::new(0_u64);
        let stack = build_stack(&elapsed_frames);
        let track = TimelineTrackDefinition {
            elements: vec![
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(50),
                    value: ValueDefinition::LiteralValue(PaxValue::Numeric(10.0.into())),
                    easing: Some(Token::new_without_location("Linear".to_string())),
                }),
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(100),
                    value: ValueDefinition::LiteralValue(PaxValue::Numeric(0.0.into())),
                    easing: None,
                }),
            ],
            playhead: None,
            duration: None,
            repeat: Some(false),
            starting_value: Some(Box::new(ValueDefinition::LiteralValue(PaxValue::Numeric(
                5.0.into(),
            )))),
            interruption: Default::default(),
            use_local_property_scope: false,
        };
        let property = build_timeline_property::<f64>("progress", &track, stack);

        assert_eq!(property.get(), 5.0);
        elapsed_frames.set(25);
        assert_eq!(property.get(), 5.0);
        elapsed_frames.set(75);
        assert_eq!(property.get(), 5.0);
        elapsed_frames.set(100);
        assert_eq!(property.get(), 0.0);
    }

    #[test]
    fn timeline_property_interpolates_rotation_tracks() {
        let elapsed_frames = Property::new(0_u64);
        let stack = build_stack(&elapsed_frames);
        let track = TimelineTrackDefinition {
            elements: vec![
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(0),
                    value: ValueDefinition::LiteralValue(PaxValue::Rotation(
                        pax_runtime_api::Rotation::Degrees((-4.0).into()),
                    )),
                    easing: Some(Token::new_without_location("Linear".to_string())),
                }),
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(100),
                    value: ValueDefinition::LiteralValue(PaxValue::Rotation(
                        pax_runtime_api::Rotation::Degrees(8.0.into()),
                    )),
                    easing: None,
                }),
            ],
            playhead: None,
            duration: None,
            repeat: Some(true),
            starting_value: None,
            interruption: Default::default(),
            use_local_property_scope: false,
        };
        let property =
            build_timeline_property::<pax_runtime_api::Rotation>("rotate", &track, stack);

        assert!((property.get().get_as_degrees() - (-4.0)).abs() < 0.0001);
        elapsed_frames.set(50);
        assert!((property.get().get_as_degrees() - 2.0).abs() < 0.0001);
        elapsed_frames.set(100);
        assert!((property.get().get_as_degrees() - 8.0).abs() < 0.0001);
    }

    #[test]
    fn transition_rotation_tracks_can_arrive_at_base() {
        let phase = Property::new(TRANSITION_PHASE_ENTER);
        let playhead = Property::new(0.0_f64);
        let base = Property::new(pax_runtime_api::Rotation::Degrees(1.0.into()));
        let scope: HashMap<String, Variable> = vec![
            (
                TRANSITION_PHASE_SYMBOL.to_string(),
                Variable::new_from_typed_property(phase.clone()),
            ),
            (
                TRANSITION_PLAYHEAD_SYMBOL.to_string(),
                Variable::new_from_typed_property(playhead.clone()),
            ),
            (
                BASE_SYMBOL.to_string(),
                Variable::new_from_typed_property(base.clone()),
            ),
        ]
        .into_iter()
        .collect();
        let stack = RuntimePropertiesStackFrame::new(scope);
        let playhead_binding = Some(Box::new(ValueDefinition::Identifier(PaxIdentifier::new(
            TRANSITION_PLAYHEAD_SYMBOL,
        ))));
        let enter = TimelineTrackDefinition {
            elements: vec![
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(0),
                    value: expression("$base + 4deg"),
                    easing: Some(Token::new_without_location("Linear".to_string())),
                }),
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(10),
                    value: expression("$base"),
                    easing: None,
                }),
            ],
            playhead: playhead_binding,
            duration: None,
            repeat: Some(false),
            starting_value: None,
            interruption: Default::default(),
            use_local_property_scope: false,
        };
        let transition = TransitionDefinition {
            enter: Some(enter),
            ..Default::default()
        };
        let property =
            build_transition_property::<pax_runtime_api::Rotation>("rotate", &transition, stack);

        assert!((property.get().get_as_degrees() - 5.0).abs() < 0.0001);
        playhead.set(10.0);
        assert!((property.get().get_as_degrees() - 1.0).abs() < 0.0001);
    }

    #[test]
    fn timeline_property_samples_from_bound_playhead_property() {
        let elapsed_frames = Property::new(0_u64);
        let playhead = Property::new(0.0_f64);
        let scope: HashMap<String, Variable> = vec![
            (
                "$frames".to_string(),
                Variable::new_from_typed_property(elapsed_frames.clone()),
            ),
            (
                "phase".to_string(),
                Variable::new_from_typed_property(playhead.clone()),
            ),
        ]
        .into_iter()
        .collect();
        let stack = RuntimePropertiesStackFrame::new(scope);
        let track = TimelineTrackDefinition {
            elements: vec![
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(0),
                    value: ValueDefinition::LiteralValue(PaxValue::Numeric(0.0.into())),
                    easing: Some(Token::new_without_location("Linear".to_string())),
                }),
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(10),
                    value: ValueDefinition::LiteralValue(PaxValue::Numeric(10.0.into())),
                    easing: None,
                }),
            ],
            playhead: Some(Box::new(ValueDefinition::Identifier(
                pax_manifest::PaxIdentifier::new("self.phase"),
            ))),
            duration: None,
            repeat: Some(false),
            starting_value: None,
            interruption: Default::default(),
            use_local_property_scope: false,
        };
        let property = build_timeline_property::<f64>("progress", &track, stack);

        assert_eq!(property.get(), 0.0);
        playhead.set(2.5);
        assert!((property.get() - 2.5).abs() < 0.0001);
        playhead.set(10.0);
        assert!((property.get() - 10.0).abs() < 0.0001);
    }
    #[test]
    fn transition_property_switches_between_enter_and_exit_tracks() {
        let phase = Property::new(0_u64);
        let playhead = Property::new(0.0_f64);
        let scope: HashMap<String, Variable> = vec![
            (
                TRANSITION_PHASE_SYMBOL.to_string(),
                Variable::new_from_typed_property(phase.clone()),
            ),
            (
                TRANSITION_PLAYHEAD_SYMBOL.to_string(),
                Variable::new_from_typed_property(playhead.clone()),
            ),
        ]
        .into_iter()
        .collect();
        let stack = RuntimePropertiesStackFrame::new(scope);
        let playhead_binding = Some(Box::new(ValueDefinition::Identifier(PaxIdentifier::new(
            TRANSITION_PLAYHEAD_SYMBOL,
        ))));
        let enter = TimelineTrackDefinition {
            elements: vec![
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(0),
                    value: ValueDefinition::LiteralValue(PaxValue::Numeric(0.0.into())),
                    easing: Some(Token::new_without_location("Linear".to_string())),
                }),
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(10),
                    value: ValueDefinition::LiteralValue(PaxValue::Numeric(10.0.into())),
                    easing: None,
                }),
            ],
            playhead: playhead_binding.clone(),
            duration: None,
            repeat: Some(false),
            starting_value: None,
            interruption: Default::default(),
            use_local_property_scope: false,
        };
        let exit = TimelineTrackDefinition {
            elements: vec![
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(0),
                    value: ValueDefinition::LiteralValue(PaxValue::Numeric(10.0.into())),
                    easing: Some(Token::new_without_location("Linear".to_string())),
                }),
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(10),
                    value: ValueDefinition::LiteralValue(PaxValue::Numeric(0.0.into())),
                    easing: None,
                }),
            ],
            playhead: playhead_binding,
            duration: None,
            repeat: Some(false),
            starting_value: None,
            interruption: Default::default(),
            use_local_property_scope: false,
        };
        let transition = TransitionDefinition {
            enter: Some(enter),
            exit: Some(exit),
            starting_value: Some(Box::new(ValueDefinition::LiteralValue(PaxValue::Numeric(
                5.0.into(),
            )))),
        };
        let property = build_transition_property::<f64>("opacity", &transition, stack);

        assert_eq!(property.get(), 5.0);
        phase.set(TRANSITION_PHASE_ENTER);
        playhead.set(5.0);
        assert_eq!(property.get(), 5.0);
        phase.set(TRANSITION_PHASE_EXIT);
        assert_eq!(property.get(), 5.0);
        playhead.set(10.0);
        assert_eq!(property.get(), 0.0);
    }

    #[test]
    fn transition_property_takes_over_from_last_typed_sample_on_reversal() {
        let phase = Property::new(TRANSITION_PHASE_ENTER);
        let playhead = Property::new(0.0_f64);
        let generation = Property::new(1_u64);
        let takeover = Property::new(false);
        let stack = RuntimePropertiesStackFrame::new(HashMap::from([
            (
                TRANSITION_PHASE_SYMBOL.to_string(),
                Variable::new_from_typed_property(phase.clone()),
            ),
            (
                TRANSITION_PLAYHEAD_SYMBOL.to_string(),
                Variable::new_from_typed_property(playhead.clone()),
            ),
            (
                TRANSITION_GENERATION_SYMBOL.to_string(),
                Variable::new_from_typed_property(generation.clone()),
            ),
            (
                TRANSITION_TAKEOVER_SYMBOL.to_string(),
                Variable::new_from_typed_property(takeover.clone()),
            ),
        ]));
        let track = |start: f64, end: f64, interruption| TimelineTrackDefinition {
            elements: vec![
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(0),
                    value: ValueDefinition::LiteralValue(PaxValue::Numeric(start.into())),
                    easing: Some(Token::new_without_location("Linear".to_string())),
                }),
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(10),
                    value: ValueDefinition::LiteralValue(PaxValue::Numeric(end.into())),
                    easing: None,
                }),
            ],
            playhead: Some(Box::new(ValueDefinition::Identifier(PaxIdentifier::new(
                TRANSITION_PLAYHEAD_SYMBOL,
            )))),
            duration: None,
            repeat: Some(false),
            starting_value: None,
            interruption,
            use_local_property_scope: false,
        };
        let transition = TransitionDefinition {
            enter: Some(track(0.0, 10.0, InOutInterruption::Takeover)),
            exit: Some(track(10.0, 0.0, InOutInterruption::Takeover)),
            starting_value: None,
        };
        let property = build_transition_property::<f64>("x", &transition, stack);

        assert_eq!(property.get(), 0.0);
        playhead.set(4.0);
        assert_eq!(property.get(), 4.0);

        playhead.set(0.0);
        takeover.set(true);
        generation.set(2);
        phase.set(TRANSITION_PHASE_EXIT);
        assert_eq!(property.get(), 4.0);
        playhead.set(5.0);
        assert_eq!(property.get(), 2.0);

        playhead.set(0.0);
        generation.set(3);
        phase.set(TRANSITION_PHASE_ENTER);
        assert_eq!(property.get(), 2.0);
        playhead.set(5.0);
        assert_eq!(property.get(), 6.0);
    }

    #[test]
    fn transition_property_restart_uses_authored_destination_start() {
        let phase = Property::new(TRANSITION_PHASE_ENTER);
        let playhead = Property::new(5.0_f64);
        let generation = Property::new(1_u64);
        let takeover = Property::new(false);
        let stack = RuntimePropertiesStackFrame::new(HashMap::from([
            (
                TRANSITION_PHASE_SYMBOL.to_string(),
                Variable::new_from_typed_property(phase.clone()),
            ),
            (
                TRANSITION_PLAYHEAD_SYMBOL.to_string(),
                Variable::new_from_typed_property(playhead.clone()),
            ),
            (
                TRANSITION_GENERATION_SYMBOL.to_string(),
                Variable::new_from_typed_property(generation.clone()),
            ),
            (
                TRANSITION_TAKEOVER_SYMBOL.to_string(),
                Variable::new_from_typed_property(takeover.clone()),
            ),
        ]));
        let track = |start: f64, end: f64, interruption| TimelineTrackDefinition {
            elements: vec![
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(0),
                    value: ValueDefinition::LiteralValue(PaxValue::Numeric(start.into())),
                    easing: Some(Token::new_without_location("Linear".to_string())),
                }),
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(10),
                    value: ValueDefinition::LiteralValue(PaxValue::Numeric(end.into())),
                    easing: None,
                }),
            ],
            playhead: Some(Box::new(ValueDefinition::Identifier(PaxIdentifier::new(
                TRANSITION_PLAYHEAD_SYMBOL,
            )))),
            duration: None,
            repeat: Some(false),
            starting_value: None,
            interruption,
            use_local_property_scope: false,
        };
        let transition = TransitionDefinition {
            enter: Some(track(0.0, 10.0, InOutInterruption::Takeover)),
            exit: Some(track(10.0, 0.0, InOutInterruption::Restart)),
            starting_value: None,
        };
        let property = build_transition_property::<f64>("x", &transition, stack);
        assert_eq!(property.get(), 5.0);

        playhead.set(0.0);
        takeover.set(true);
        generation.set(2);
        phase.set(TRANSITION_PHASE_EXIT);
        assert_eq!(property.get(), 10.0);
    }

    #[test]
    fn transition_property_uses_millisecond_playhead_for_dynamic_millisecond_duration() {
        let phase = Property::new(TRANSITION_PHASE_ENTER);
        let frame_playhead = Property::new(0.0_f64);
        let millis_playhead = Property::new(0.0_f64);
        let duration = Property::new(Duration::Milliseconds(1000.into()));
        let scope: HashMap<String, Variable> = vec![
            (
                TRANSITION_PHASE_SYMBOL.to_string(),
                Variable::new_from_typed_property(phase),
            ),
            (
                TRANSITION_PLAYHEAD_SYMBOL.to_string(),
                Variable::new_from_typed_property(frame_playhead.clone()),
            ),
            (
                TRANSITION_PLAYHEAD_MILLIS_SYMBOL.to_string(),
                Variable::new_from_typed_property(millis_playhead.clone()),
            ),
            (
                "duration".to_string(),
                Variable::new_from_typed_property(duration),
            ),
        ]
        .into_iter()
        .collect();
        let stack = RuntimePropertiesStackFrame::new(scope);
        let enter = TimelineTrackDefinition {
            elements: vec![
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Percent(0.0),
                    value: ValueDefinition::LiteralValue(PaxValue::Numeric(0.0.into())),
                    easing: Some(Token::new_without_location("Linear".to_string())),
                }),
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Percent(100.0),
                    value: ValueDefinition::LiteralValue(PaxValue::Numeric(1.0.into())),
                    easing: None,
                }),
            ],
            playhead: Some(Box::new(ValueDefinition::Identifier(PaxIdentifier::new(
                TRANSITION_PLAYHEAD_SYMBOL,
            )))),
            duration: Some(Box::new(ValueDefinition::Identifier(PaxIdentifier::new(
                "self.duration",
            )))),
            repeat: Some(false),
            starting_value: None,
            interruption: Default::default(),
            use_local_property_scope: false,
        };
        let transition = TransitionDefinition {
            enter: Some(enter),
            exit: None,
            starting_value: Some(Box::new(ValueDefinition::LiteralValue(PaxValue::Numeric(
                1.0.into(),
            )))),
        };
        let property = build_transition_property::<f64>("opacity", &transition, stack);

        frame_playhead.set(500.0);
        assert_eq!(property.get(), 0.0);
        millis_playhead.set(500.0);
        assert_eq!(property.get(), 0.5);
        millis_playhead.set(1000.0);
        assert_eq!(property.get(), 1.0);
    }

    #[test]
    fn timeline_property_skips_keyframes_that_fail_typed_coercion() {
        let elapsed_frames = Property::new(0_u64);
        let stack = build_stack(&elapsed_frames);
        let track = TimelineTrackDefinition {
            elements: vec![
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(0),
                    value: ValueDefinition::LiteralValue(PaxValue::Numeric(0.0.into())),
                    easing: Some(Token::new_without_location("Linear".to_string())),
                }),
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(50),
                    value: ValueDefinition::LiteralValue(PaxValue::String("bad".to_string())),
                    easing: Some(Token::new_without_location("Linear".to_string())),
                }),
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(100),
                    value: ValueDefinition::LiteralValue(PaxValue::Numeric(100.0.into())),
                    easing: None,
                }),
            ],
            playhead: None,
            duration: None,
            repeat: Some(false),
            starting_value: None,
            interruption: Default::default(),
            use_local_property_scope: false,
        };
        let property = build_timeline_property::<f64>("progress", &track, stack);

        elapsed_frames.set(75);
        assert!((property.get() - 75.0).abs() < 0.0001);
    }
}

#[cfg(test)]
mod base_symbol_tests {
    use super::{
        build_component_property, create_new_common_properties_from_columns,
        property_columns_from_defined_properties, stack_with_base, RuntimePropertiesStackFrame,
        RuntimeResolvedPropertyColumns, RuntimeResolvedPropertyEntry, RuntimeSettingsCondition,
        RuntimeSettingsSource,
    };
    use pax_language::parse_pax_expression;
    use pax_manifest::cartridge_generation::{
        TRANSITION_PHASE_ENTER, TRANSITION_PHASE_SYMBOL, TRANSITION_PLAYHEAD_SYMBOL,
    };
    use pax_manifest::{
        ExpressionInfo, PaxIdentifier, TimelineKeyframe, TimelineMarker, TimelineTrackDefinition,
        TimelineTrackElement, Token, TransitionDefinition, ValueDefinition,
    };
    use pax_runtime_api::{Duration, Numeric, Opacity, PaxValue, Property, Size, Variable};
    use std::collections::{BTreeMap, HashMap};
    use std::rc::Rc;

    fn expression(raw: &str) -> ValueDefinition {
        pax_runtime_api::pax_value::functions::Functions::register_all_functions();
        ValueDefinition::Expression(ExpressionInfo::new(parse_pax_expression(raw).unwrap()))
    }

    fn size_literal(px: f64) -> ValueDefinition {
        ValueDefinition::LiteralValue(PaxValue::Size(Size::Pixels(Numeric::F64(px))))
    }

    fn percent_literal(percent: f64) -> ValueDefinition {
        ValueDefinition::LiteralValue(PaxValue::Size(Size::Percent(Numeric::F64(percent))))
    }

    fn size_pair_literal(x: f64, y: f64) -> ValueDefinition {
        ValueDefinition::LiteralValue(PaxValue::Vec(vec![
            PaxValue::Size(Size::Percent(Numeric::F64(x))),
            PaxValue::Size(Size::Percent(Numeric::F64(y))),
        ]))
    }

    fn assert_percent(size: Option<Size>, expected: f64) {
        let actual = size.expect("expected common property to be set");
        assert!((actual.expect_percent() - (expected / 100.0)).abs() < 0.0001);
    }

    fn entry(value: ValueDefinition) -> RuntimeResolvedPropertyEntry {
        RuntimeResolvedPropertyEntry {
            source: RuntimeSettingsSource::Inline,
            selector: None,
            source_location: None,
            source_stack: None,
            value,
            condition: None,
            axis_index: None,
        }
    }

    fn conditional_entry(value: ValueDefinition, condition: &str) -> RuntimeResolvedPropertyEntry {
        RuntimeResolvedPropertyEntry {
            condition: Some(RuntimeSettingsCondition {
                positive: vec![ExpressionInfo::new(
                    parse_pax_expression(condition).unwrap(),
                )],
                negative: Vec::new(),
            }),
            ..entry(value)
        }
    }

    fn columns_for(
        name: &str,
        entries: Vec<RuntimeResolvedPropertyEntry>,
    ) -> RuntimeResolvedPropertyColumns {
        BTreeMap::from([(name.to_string(), entries)])
    }

    fn empty_stack() -> Rc<RuntimePropertiesStackFrame> {
        RuntimePropertiesStackFrame::new(HashMap::new())
    }

    #[test]
    fn base_symbol_resolves_previous_component_property_layer() {
        let base_property = Property::new(10.0_f64);
        let stack = stack_with_base(&empty_stack(), base_property);

        let property: Property<f64> = build_component_property(
            "opacity",
            &expression("$base + 0.25"),
            &stack,
            stack.clone(),
            |_, _| unreachable!("literal blocks are not used by this test"),
        );

        assert!((property.get() - 10.25).abs() < 0.0001);
    }

    #[test]
    fn base_symbol_resolves_previous_common_property_layer() {
        let stack = empty_stack();
        let columns = columns_for(
            "x",
            vec![entry(size_literal(10.0)), entry(expression("$base + 5px"))],
        );

        let common = create_new_common_properties_from_columns(&columns, &stack);
        let x = common.borrow().x.get().unwrap();

        assert!((x.get_pixels(100.0) - 15.0).abs() < 0.0001);
    }

    #[test]
    fn base_symbol_tracks_reactive_previous_layers() {
        let offset = Property::new(Size::Pixels(Numeric::F64(10.0)));
        let stack = RuntimePropertiesStackFrame::new(HashMap::from([(
            "offset".to_string(),
            Variable::new_from_typed_property(offset.clone()),
        )]));
        let columns = columns_for(
            "x",
            vec![
                entry(expression("offset")),
                entry(expression("$base + 5px")),
            ],
        );

        let common = create_new_common_properties_from_columns(&columns, &stack);
        assert!((common.borrow().x.get().unwrap().get_pixels(100.0) - 15.0).abs() < 0.0001);

        offset.set(Size::Pixels(Numeric::F64(20.0)));
        assert!((common.borrow().x.get().unwrap().get_pixels(100.0) - 25.0).abs() < 0.0001);
    }

    #[test]
    fn imported_settings_use_the_provider_scope_reactively() {
        let is_dark = Property::new(false);
        let provider_stack = RuntimePropertiesStackFrame::new(HashMap::from([(
            "is_dark".to_string(),
            Variable::new_from_typed_property(is_dark.clone()),
        )]));
        let imported_entry = RuntimeResolvedPropertyEntry {
            source: RuntimeSettingsSource::ImportedLayer {
                provider_id: crate::ExpandedNodeIdentifier(1),
                provider_type_id: pax_manifest::TypeId::build_singleton(
                    "example::Theme",
                    Some("Theme"),
                ),
            },
            source_stack: Some(provider_stack),
            ..entry(expression("self.is_dark ? 50px : 10px"))
        };
        let columns = columns_for("width", vec![imported_entry]);

        let common = create_new_common_properties_from_columns(&columns, &empty_stack());
        assert!((common.borrow().width.get().unwrap().get_pixels(100.0) - 10.0).abs() < 0.0001);

        is_dark.set(true);
        assert!((common.borrow().width.get().unwrap().get_pixels(100.0) - 50.0).abs() < 0.0001);
    }

    #[test]
    fn imported_settings_conditions_use_the_provider_scope_reactively() {
        let is_dark = Property::new(false);
        let provider_stack = RuntimePropertiesStackFrame::new(HashMap::from([(
            "is_dark".to_string(),
            Variable::new_from_typed_property(is_dark.clone()),
        )]));
        let imported_entry = RuntimeResolvedPropertyEntry {
            source: RuntimeSettingsSource::ImportedLayer {
                provider_id: crate::ExpandedNodeIdentifier(1),
                provider_type_id: pax_manifest::TypeId::build_singleton(
                    "example::Theme",
                    Some("Theme"),
                ),
            },
            source_stack: Some(provider_stack),
            condition: Some(RuntimeSettingsCondition {
                positive: vec![ExpressionInfo::new(
                    parse_pax_expression("self.is_dark").unwrap(),
                )],
                negative: Vec::new(),
            }),
            ..entry(size_literal(50.0))
        };
        let columns = columns_for("width", vec![entry(size_literal(10.0)), imported_entry]);

        let common = create_new_common_properties_from_columns(&columns, &empty_stack());
        assert!((common.borrow().width.get().unwrap().get_pixels(100.0) - 10.0).abs() < 0.0001);

        is_dark.set(true);
        assert!((common.borrow().width.get().unwrap().get_pixels(100.0) - 50.0).abs() < 0.0001);
    }

    #[test]
    fn settings_condition_preserves_previous_layer_until_active() {
        let landscape = Property::new(false);
        let stack = RuntimePropertiesStackFrame::new(HashMap::from([(
            "$landscape".to_string(),
            Variable::new_from_typed_property(landscape.clone()),
        )]));
        let columns = columns_for(
            "width",
            vec![
                entry(size_literal(100.0)),
                conditional_entry(size_literal(50.0), "$landscape"),
            ],
        );

        let common = create_new_common_properties_from_columns(&columns, &stack);
        assert!((common.borrow().width.get().unwrap().get_pixels(100.0) - 100.0).abs() < 0.0001);

        landscape.set(true);
        assert!((common.borrow().width.get().unwrap().get_pixels(100.0) - 50.0).abs() < 0.0001);
    }

    #[test]
    fn base_symbol_supplies_timeline_starting_values() {
        let elapsed_frames = Property::new(0_u64);
        let stack = RuntimePropertiesStackFrame::new(HashMap::from([(
            "$frames".to_string(),
            Variable::new_from_typed_property(elapsed_frames.clone()),
        )]));
        let track = TimelineTrackDefinition {
            elements: vec![
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(0),
                    value: expression("$base - 10px"),
                    easing: Some(Token::new_without_location("Linear".to_string())),
                }),
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(10),
                    value: expression("$base"),
                    easing: None,
                }),
            ],
            playhead: None,
            duration: Some(Box::new(ValueDefinition::LiteralValue(PaxValue::Duration(
                Duration::Frames(10.into()),
            )))),
            repeat: Some(false),
            starting_value: None,
            interruption: Default::default(),
            use_local_property_scope: false,
        };
        let columns = columns_for(
            "y",
            vec![
                entry(size_literal(100.0)),
                entry(ValueDefinition::Timeline(track)),
            ],
        );

        let common = create_new_common_properties_from_columns(&columns, &stack);
        assert!((common.borrow().y.get().unwrap().get_pixels(100.0) - 90.0).abs() < 0.0001);

        elapsed_frames.set(10);
        assert!((common.borrow().y.get().unwrap().get_pixels(100.0) - 100.0).abs() < 0.0001);
    }

    #[test]
    fn base_symbol_uses_layout_zero_for_unset_position_transitions() {
        let phase = Property::new(TRANSITION_PHASE_ENTER);
        let playhead = Property::new(5.0_f64);
        let stack = RuntimePropertiesStackFrame::new(HashMap::from([
            (
                TRANSITION_PHASE_SYMBOL.to_string(),
                Variable::new_from_typed_property(phase.clone()),
            ),
            (
                TRANSITION_PLAYHEAD_SYMBOL.to_string(),
                Variable::new_from_typed_property(playhead.clone()),
            ),
        ]));
        let playhead_binding = Some(Box::new(ValueDefinition::Identifier(PaxIdentifier::new(
            TRANSITION_PLAYHEAD_SYMBOL,
        ))));
        let enter = TimelineTrackDefinition {
            elements: vec![
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(0),
                    value: expression("$base - 100px"),
                    easing: Some(Token::new_without_location("InQuad".to_string())),
                }),
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(10),
                    value: expression("$base"),
                    easing: None,
                }),
            ],
            playhead: playhead_binding,
            duration: Some(Box::new(ValueDefinition::LiteralValue(PaxValue::Duration(
                Duration::Frames(10.into()),
            )))),
            repeat: Some(false),
            starting_value: None,
            interruption: Default::default(),
            use_local_property_scope: false,
        };
        let transition = TransitionDefinition {
            enter: Some(enter),
            ..Default::default()
        };
        let columns = columns_for("y", vec![entry(ValueDefinition::Transition(transition))]);

        let common = create_new_common_properties_from_columns(&columns, &stack);
        let y = common.borrow().y.get().unwrap();
        assert!((y.get_pixels(100.0) - -75.0).abs() < 0.0001);

        playhead.set(10.0);
        let y = common.borrow().y.get().unwrap();
        assert!(y.get_pixels(100.0).abs() < 0.0001);
    }

    #[test]
    fn base_symbol_uses_opaque_for_unset_opacity_transitions() {
        let phase = Property::new(TRANSITION_PHASE_ENTER);
        let playhead = Property::new(5.0_f64);
        let stack = RuntimePropertiesStackFrame::new(HashMap::from([
            (
                TRANSITION_PHASE_SYMBOL.to_string(),
                Variable::new_from_typed_property(phase.clone()),
            ),
            (
                TRANSITION_PLAYHEAD_SYMBOL.to_string(),
                Variable::new_from_typed_property(playhead.clone()),
            ),
        ]));
        let playhead_binding = Some(Box::new(ValueDefinition::Identifier(PaxIdentifier::new(
            TRANSITION_PLAYHEAD_SYMBOL,
        ))));
        let enter = TimelineTrackDefinition {
            elements: vec![
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(0),
                    value: ValueDefinition::LiteralValue(PaxValue::Numeric(0.0.into())),
                    easing: Some(Token::new_without_location("Linear".to_string())),
                }),
                TimelineTrackElement::Keyframe(TimelineKeyframe {
                    marker: TimelineMarker::Frame(10),
                    value: expression("$base"),
                    easing: None,
                }),
            ],
            playhead: playhead_binding,
            duration: Some(Box::new(ValueDefinition::LiteralValue(PaxValue::Duration(
                Duration::Frames(10.into()),
            )))),
            repeat: Some(false),
            starting_value: None,
            interruption: Default::default(),
            use_local_property_scope: false,
        };
        let transition = TransitionDefinition {
            enter: Some(enter),
            ..Default::default()
        };
        let columns = columns_for(
            "opacity",
            vec![entry(ValueDefinition::Transition(transition))],
        );

        let common = create_new_common_properties_from_columns(&columns, &stack);
        assert_eq!(
            common.borrow().opacity.get(),
            Some(Opacity::Alpha(0.5.into()))
        );

        playhead.set(10.0);
        assert_eq!(
            common.borrow().opacity.get(),
            Some(Opacity::Alpha(1.0.into()))
        );
    }

    #[test]
    fn scalar_common_property_alias_applies_to_both_axes() {
        let stack = empty_stack();
        let defined = BTreeMap::from([("scale".to_string(), percent_literal(20.0))]);
        let columns = property_columns_from_defined_properties(&defined);

        let common = create_new_common_properties_from_columns(&columns, &stack);

        assert_percent(common.borrow().scale_x.get(), 20.0);
        assert_percent(common.borrow().scale_y.get(), 20.0);
    }

    #[test]
    fn list_common_property_alias_splits_across_axes() {
        let stack = empty_stack();
        let defined = BTreeMap::from([("anchor".to_string(), size_pair_literal(25.0, 75.0))]);
        let columns = property_columns_from_defined_properties(&defined);

        let common = create_new_common_properties_from_columns(&columns, &stack);

        assert_percent(common.borrow().anchor_x.get(), 25.0);
        assert_percent(common.borrow().anchor_y.get(), 75.0);
    }

    #[test]
    fn axis_specific_common_property_overrides_alias_with_base() {
        let stack = empty_stack();
        let defined = BTreeMap::from([
            ("scale".to_string(), percent_literal(20.0)),
            ("scale_y".to_string(), expression("$base + 5")),
        ]);
        let columns = property_columns_from_defined_properties(&defined);

        let common = create_new_common_properties_from_columns(&columns, &stack);

        assert_percent(common.borrow().scale_x.get(), 20.0);
        assert_percent(common.borrow().scale_y.get(), 25.0);
    }
}

/// Applies resolved common-property columns to an existing expanded node,
/// preserving layer order so `$base` can reference each prior layer.
pub fn update_existing_common_properties_from_columns(
    expanded_node: &Rc<ExpandedNode>,
    property_columns: &RuntimeResolvedPropertyColumns,
    stack_frame: &Rc<RuntimePropertiesStackFrame>,
) {
    let expanded_node = borrow!(**expanded_node);
    let outer_ref = expanded_node.common_properties.borrow();
    let rc = Rc::clone(&outer_ref);
    let inner_ref = (*rc).borrow_mut();
    let mut cp = inner_ref;

    update_common_properties(&mut cp, property_columns, stack_frame);
}

/// Applies a flattened common-property map to an existing expanded node.
pub fn update_existing_common_properties(
    expanded_node: &Rc<ExpandedNode>,
    defined_properties: &BTreeMap<String, pax_manifest::ValueDefinition>,
    stack_frame: &Rc<RuntimePropertiesStackFrame>,
) {
    update_existing_common_properties_from_columns(
        expanded_node,
        &property_columns_from_defined_properties(defined_properties),
        stack_frame,
    );
}

fn create_id_property(
    property_columns: &RuntimeResolvedPropertyColumns,
) -> Property<Option<String>> {
    let id = property_columns
        .get("id")
        .and_then(|entries| entries.last());
    Property::new(
        if let Some(RuntimeResolvedPropertyEntry {
            value: pax_manifest::ValueDefinition::Identifier(pax_identifier),
            ..
        }) = id
        {
            Some(pax_identifier.name.clone())
        } else {
            None
        },
    )
}

/// Creates common properties from resolved columns, preserving layer order so
/// `$base` can reference each prior layer.
pub fn create_new_common_properties_from_columns(
    property_columns: &RuntimeResolvedPropertyColumns,
    stack_frame: &Rc<RuntimePropertiesStackFrame>,
) -> Rc<RefCell<CommonProperties>> {
    Rc::new(RefCell::new(CommonProperties {
        id: create_id_property(property_columns),
        x: resolve_property("x", property_columns, stack_frame),
        y: resolve_property("y", property_columns, stack_frame),
        padding_x: resolve_property("padding_x", property_columns, stack_frame),
        padding_y: resolve_property("padding_y", property_columns, stack_frame),
        width: resolve_property("width", property_columns, stack_frame),
        height: resolve_property("height", property_columns, stack_frame),
        scale_x: resolve_property("scale_x", property_columns, stack_frame),
        scale_y: resolve_property("scale_y", property_columns, stack_frame),
        skew_x: resolve_property("skew_x", property_columns, stack_frame),
        skew_y: resolve_property("skew_y", property_columns, stack_frame),
        rotate: resolve_property("rotate", property_columns, stack_frame),
        transform: resolve_property("transform", property_columns, stack_frame),
        opacity: resolve_property("opacity", property_columns, stack_frame),
        layout_role: resolve_property("layout_role", property_columns, stack_frame),
        anchor_x: resolve_property("anchor_x", property_columns, stack_frame),
        anchor_y: resolve_property("anchor_y", property_columns, stack_frame),
        unclippable: resolve_property("unclippable", property_columns, stack_frame),
        _raycastable: resolve_property("_raycastable", property_columns, stack_frame),
        _suspended: resolve_property("_suspended", property_columns, stack_frame),
    }))
}

/// Creates common properties from a flattened property map.
pub fn create_new_common_properties(
    defined_properties: &BTreeMap<String, pax_manifest::ValueDefinition>,
    stack_frame: &Rc<RuntimePropertiesStackFrame>,
) -> Rc<RefCell<CommonProperties>> {
    create_new_common_properties_from_columns(
        &property_columns_from_defined_properties(defined_properties),
        stack_frame,
    )
}

fn update_common_properties(
    cp: &mut CommonProperties,
    property_columns: &RuntimeResolvedPropertyColumns,
    stack_frame: &Rc<RuntimePropertiesStackFrame>,
) {
    cp.id.replace_with(create_id_property(property_columns));
    cp.x.replace_with(resolve_property("x", property_columns, stack_frame));
    cp.y.replace_with(resolve_property("y", property_columns, stack_frame));
    cp.padding_x
        .replace_with(resolve_property("padding_x", property_columns, stack_frame));
    cp.padding_y
        .replace_with(resolve_property("padding_y", property_columns, stack_frame));
    cp.width
        .replace_with(resolve_property("width", property_columns, stack_frame));
    cp.height
        .replace_with(resolve_property("height", property_columns, stack_frame));
    cp.scale_x
        .replace_with(resolve_property("scale_x", property_columns, stack_frame));
    cp.scale_y
        .replace_with(resolve_property("scale_y", property_columns, stack_frame));
    cp.skew_x
        .replace_with(resolve_property("skew_x", property_columns, stack_frame));
    cp.skew_y
        .replace_with(resolve_property("skew_y", property_columns, stack_frame));
    cp.rotate
        .replace_with(resolve_property("rotate", property_columns, stack_frame));
    cp.transform
        .replace_with(resolve_property("transform", property_columns, stack_frame));
    cp.opacity
        .replace_with(resolve_property("opacity", property_columns, stack_frame));
    cp.layout_role.replace_with(resolve_property(
        "layout_role",
        property_columns,
        stack_frame,
    ));
    cp.anchor_x
        .replace_with(resolve_property("anchor_x", property_columns, stack_frame));
    cp.anchor_y
        .replace_with(resolve_property("anchor_y", property_columns, stack_frame));
    cp.unclippable.replace_with(resolve_property(
        "unclippable",
        property_columns,
        stack_frame,
    ));
    cp._raycastable.replace_with(resolve_property(
        "_raycastable",
        property_columns,
        stack_frame,
    ));
    cp._suspended.replace_with(resolve_property(
        "_suspended",
        property_columns,
        stack_frame,
    ));
}

#[cfg(test)]
mod runtime_settings_tests {
    use super::{add_removed_property_resets, resolve_runtime_settings_with_layers_for_node};
    use crate::{
        ExpandedNodeIdentifier, RuntimePropertiesStackFrame, RuntimeSettingsLayer,
        RuntimeSettingsSource,
    };
    use pax_manifest::{
        LiteralBlockDefinition, PaxIdentifier, SettingElement, SettingsBlockElement,
        TemplateNodeDefinition, TimelineBlockElement, TimelineDefinition,
        TimelineSelectorBlockDefinition, TimelineSelectorElement, TimelineTrackDefinition, Token,
        TypeId, ValueDefinition,
    };
    use pax_runtime_api::PaxValue;
    use std::collections::{BTreeMap, HashMap};

    fn setting(name: &str, value: i32) -> SettingElement {
        SettingElement::Setting(
            Token::new_without_location(name.to_string()),
            ValueDefinition::LiteralValue(PaxValue::Numeric(value.into())),
        )
    }

    fn selector_block(selector: &str, elements: Vec<SettingElement>) -> SettingsBlockElement {
        SettingsBlockElement::SelectorBlock(
            Token::new_without_location(selector.to_string()),
            LiteralBlockDefinition::new(elements),
        )
    }

    #[test]
    fn runtime_settings_resolve_columns_and_provenance_by_layer() {
        let mut node = TemplateNodeDefinition {
            type_id: TypeId::build_singleton("example::Text", Some("Text")),
            control_flow_settings: None,
            settings: Some(vec![
                SettingElement::Setting(
                    Token::new_without_location("id".to_string()),
                    ValueDefinition::Identifier(PaxIdentifier::new("hero")),
                ),
                setting("fill", 4),
            ]),
            selector_info: pax_manifest::TemplateNodeSelectorInfo {
                class_binding: Some(ValueDefinition::LiteralValue(PaxValue::String(
                    "headline".to_string(),
                ))),
                ..Default::default()
            },
            raw_comment_string: None,
        };
        node.normalize_selector_info();

        let component_settings = Some(vec![selector_block(
            "Text",
            vec![setting("width", 1), setting("fill", 1)],
        )]);
        let imported_layers = vec![
            RuntimeSettingsLayer {
                provider_id: ExpandedNodeIdentifier(10),
                provider_type_id: TypeId::build_singleton("example::BaseTheme", Some("BaseTheme")),
                provider_stack: RuntimePropertiesStackFrame::new(HashMap::new()),
                settings: vec![selector_block(
                    "Text",
                    vec![setting("height", 2), setting("fill", 2)],
                )],
            },
            RuntimeSettingsLayer {
                provider_id: ExpandedNodeIdentifier(11),
                provider_type_id: TypeId::build_singleton(
                    "example::AccentTheme",
                    Some("AccentTheme"),
                ),
                provider_stack: RuntimePropertiesStackFrame::new(HashMap::new()),
                settings: vec![selector_block(
                    ".headline",
                    vec![setting("opacity", 3), setting("fill", 3)],
                )],
            },
        ];

        let resolved = resolve_runtime_settings_with_layers_for_node(
            &node,
            &BTreeMap::new(),
            &component_settings,
            &imported_layers,
            &["headline".to_string()],
            None,
            &[],
        );

        assert!(matches!(
            resolved.defined_properties.get("width"),
            Some(ValueDefinition::LiteralValue(PaxValue::Numeric(value))) if *value == 1.into()
        ));
        assert!(matches!(
            resolved.defined_properties.get("height"),
            Some(ValueDefinition::LiteralValue(PaxValue::Numeric(value))) if *value == 2.into()
        ));
        assert!(matches!(
            resolved.defined_properties.get("opacity"),
            Some(ValueDefinition::LiteralValue(PaxValue::Numeric(value))) if *value == 3.into()
        ));
        assert!(matches!(
            resolved.defined_properties.get("fill"),
            Some(ValueDefinition::LiteralValue(PaxValue::Numeric(value))) if *value == 4.into()
        ));

        let fill_column = resolved
            .columns
            .get("fill")
            .expect("fill column should exist");
        assert_eq!(fill_column.len(), 4);
        assert!(matches!(
            fill_column[0].source,
            RuntimeSettingsSource::ComponentSettings
        ));
        assert!(matches!(
            fill_column[1].source,
            RuntimeSettingsSource::ImportedLayer { provider_id, .. } if provider_id == ExpandedNodeIdentifier(10)
        ));
        assert!(matches!(
            fill_column[2].source,
            RuntimeSettingsSource::ImportedLayer { provider_id, .. } if provider_id == ExpandedNodeIdentifier(11)
        ));
        assert!(matches!(
            fill_column[3].source,
            RuntimeSettingsSource::Inline
        ));

        let width_source = resolved
            .provenance
            .get("width")
            .expect("width provenance should exist");
        assert!(matches!(
            width_source.source,
            RuntimeSettingsSource::ComponentSettings
        ));
        assert!(matches!(
            width_source.selector,
            Some(pax_manifest::SelectorExpr::Type(ref selector)) if selector == "Text"
        ));

        let opacity_source = resolved
            .provenance
            .get("opacity")
            .expect("opacity provenance should exist");
        assert!(matches!(
            opacity_source.source,
            RuntimeSettingsSource::ImportedLayer { provider_id, .. } if provider_id == ExpandedNodeIdentifier(11)
        ));
        assert!(matches!(
            opacity_source.selector,
            Some(pax_manifest::SelectorExpr::Class(ref selector)) if selector == "headline"
        ));

        let fill_source = resolved
            .provenance
            .get("fill")
            .expect("fill provenance should exist");
        assert!(matches!(fill_source.source, RuntimeSettingsSource::Inline));
        assert!(fill_source.selector.is_none());
    }

    #[test]
    fn selector_cascade_is_type_then_classes_then_id_then_inline() {
        let mut node = TemplateNodeDefinition {
            type_id: TypeId::build_singleton("example::Text", Some("Text")),
            settings: Some(vec![
                SettingElement::Setting(
                    Token::new_without_location("id".to_string()),
                    ValueDefinition::Identifier(PaxIdentifier::new("hero")),
                ),
                setting("fill", 5),
            ]),
            ..Default::default()
        };
        node.normalize_selector_info();
        let component_settings = Some(vec![
            selector_block("Text", vec![setting("fill", 1)]),
            selector_block(".base", vec![setting("fill", 2)]),
            selector_block(".selected", vec![setting("fill", 3)]),
            selector_block("#hero", vec![setting("fill", 4)]),
        ]);

        let resolved = resolve_runtime_settings_with_layers_for_node(
            &node,
            &BTreeMap::new(),
            &component_settings,
            &[],
            &["base".to_string(), "selected".to_string()],
            None,
            &[],
        );
        let values = resolved.columns["fill"]
            .iter()
            .map(|entry| match &entry.value {
                ValueDefinition::LiteralValue(PaxValue::Numeric(value)) => *value,
                other => panic!("unexpected fill value: {other:?}"),
            })
            .collect::<Vec<_>>();

        assert_eq!(
            values,
            vec![1.into(), 2.into(), 3.into(), 4.into(), 5.into()]
        );
    }

    #[test]
    fn lifecycle_timeline_uses_classes_captured_at_transition_start() {
        let node = TemplateNodeDefinition {
            type_id: TypeId::build_singleton("example::Rectangle", Some("Rectangle")),
            ..Default::default()
        };
        let component_settings = Some(vec![SettingsBlockElement::Transition(
            Token::new_without_location("in".to_string()),
            Token::new_without_location("appear".to_string()),
        )]);
        let timelines = vec![TimelineDefinition {
            name: Some(Token::new_without_location("appear".to_string())),
            elements: vec![TimelineBlockElement::SelectorBlock(
                Token::new_without_location(".active".to_string()),
                TimelineSelectorBlockDefinition {
                    elements: vec![TimelineSelectorElement::Track(
                        Token::new_without_location("opacity".to_string()),
                        TimelineTrackDefinition::default(),
                    )],
                },
            )],
            ..Default::default()
        }];

        let resolved = resolve_runtime_settings_with_layers_for_node(
            &node,
            &BTreeMap::new(),
            &component_settings,
            &[],
            &[],
            Some(&["active".to_string()]),
            &timelines,
        );

        assert!(matches!(
            resolved.defined_properties.get("opacity"),
            Some(ValueDefinition::Transition(_))
        ));
    }

    #[test]
    fn removed_runtime_properties_are_explicitly_reset_only_when_requested() {
        let previous =
            BTreeMap::from([("fill".to_string(), vec![]), ("width".to_string(), vec![])]);
        let mut current = BTreeMap::from([("width".to_string(), vec![])]);

        add_removed_property_resets(&previous, &mut current);

        assert!(current.contains_key("fill"));
        assert!(current.contains_key("width"));
    }
}
