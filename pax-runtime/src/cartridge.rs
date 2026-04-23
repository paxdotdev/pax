use_RefCell!();
use crate::api::NodeContext;
use crate::{
    ConditionalProperties, ExpandedNode, Handler, HandlerRegistry, InstanceNode, InstantiationArgs,
    ReusableInstanceNodeArgs, RuntimePropertiesStackFrame,
};
use pax_language::Computable;
use pax_manifest::cartridge_generation::{
    TRANSITION_PHASE_ENTER, TRANSITION_PHASE_EXIT, TRANSITION_PHASE_SYMBOL,
    TRANSITION_PLAYHEAD_SYMBOL,
};
use pax_manifest::{
    ExpressionInfo, LiteralBlockDefinition, SettingElement, SettingsBlockElement,
    TemplateNodeDefinition, TimelineKeyframe, TimelineMarker, TimelineTrackDefinition,
    TimelineTrackElement, TransitionDefinition, TypeId, ValueDefinition,
};
use pax_message::{borrow, borrow_mut};
use pax_runtime_api::pax_value::{CoercionRules, PaxAny, ToFromPaxAny};
use pax_runtime_api::properties::{PropertyValue, UntypedProperty};
use pax_runtime_api::{
    use_RefCell, CommonProperties, EasingCurve, Numeric, PaxValue, Property, Variable,
};
use std::any::Any;
use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;
use std::rc::Rc;

pub trait PaxCartridge {}

fn settings_layers_for_node(
    expanded_node: Option<&Rc<ExpandedNode>>,
    containing_component_settings: &Option<Vec<SettingsBlockElement>>,
) -> Vec<Option<Vec<SettingsBlockElement>>> {
    let mut layers = vec![containing_component_settings.clone()];
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
            .cloned()
            .map(Some),
    );
    layers
}

fn overlay_static_runtime_properties(
    map: &mut BTreeMap<String, ValueDefinition>,
    base_defined_properties: &BTreeMap<String, ValueDefinition>,
) {
    for (key, value) in base_defined_properties {
        if matches!(
            value,
            ValueDefinition::Timeline(_) | ValueDefinition::Transition(_)
        ) {
            map.insert(key.clone(), value.clone());
        }
    }
}

fn resolve_defined_properties_for_node(
    tnd: &TemplateNodeDefinition,
    base_defined_properties: &BTreeMap<String, ValueDefinition>,
    containing_component_settings: &Option<Vec<SettingsBlockElement>>,
    expanded_node: Option<&Rc<ExpandedNode>>,
) -> BTreeMap<String, ValueDefinition> {
    let settings_layers = settings_layers_for_node(expanded_node, containing_component_settings);
    if settings_layers.len() == 1 {
        return base_defined_properties.clone();
    }

    let merged_settings =
        pax_manifest::PaxManifest::merge_inline_settings_with_settings_layers(tnd, &settings_layers);
    let mut map = BTreeMap::new();
    if let Some(settings) = merged_settings {
        for setting in settings {
            if let SettingElement::Setting(key, value) = setting {
                match value {
                    ValueDefinition::LiteralValue(_)
                    | ValueDefinition::Block(_)
                    | ValueDefinition::Timeline(_)
                    | ValueDefinition::Transition(_)
                    | ValueDefinition::Expression(_)
                    | ValueDefinition::Identifier(_)
                    | ValueDefinition::DoubleBinding(_) => {
                        map.insert(key.token_value.clone(), value.clone());
                    }
                    ValueDefinition::EventBindingTarget(_) | ValueDefinition::Undefined => {}
                }
            }
        }
    }
    overlay_static_runtime_properties(&mut map, base_defined_properties);
    map
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

pub struct ComponentPropertyDescriptor<T> {
    pub name: &'static str,
    pub apply: fn(
        &mut T,
        &ValueDefinition,
        &Rc<RuntimePropertiesStackFrame>,
        Rc<RuntimePropertiesStackFrame>,
    ),
    marker: PhantomData<fn(&T)>,
}

impl<T> ComponentPropertyDescriptor<T> {
    pub const fn new(
        name: &'static str,
        apply: fn(
            &mut T,
            &ValueDefinition,
            &Rc<RuntimePropertiesStackFrame>,
            Rc<RuntimePropertiesStackFrame>,
        ),
    ) -> Self {
        Self {
            name,
            apply,
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
        &BTreeMap<String, ValueDefinition>,
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
            &BTreeMap<String, ValueDefinition>,
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
    defined_properties: &BTreeMap<String, ValueDefinition>,
    stack_frame: &Rc<RuntimePropertiesStackFrame>,
) {
    for property_descriptor in descriptor.property_descriptors {
        if let Some(value_definition) = defined_properties.get(property_descriptor.name) {
            let timeline_stack = if let ValueDefinition::Timeline(track) = value_definition {
                if track.use_local_property_scope {
                    stack_frame.push(build_property_scope(
                        properties,
                        descriptor.property_scope_descriptors,
                    ))
                } else {
                    stack_frame.clone()
                }
            } else {
                stack_frame.clone()
            };
            (property_descriptor.apply)(properties, value_definition, stack_frame, timeline_stack);
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
    defined_properties: &BTreeMap<String, ValueDefinition>,
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
    apply_component_descriptor_properties(properties, descriptor, defined_properties, stack_frame);
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
            pax_manifest::PaxType::Slot => {
                let expr_info = tnd
                    .control_flow_settings
                    .as_ref()
                    .unwrap()
                    .slot_index_expression
                    .as_ref()
                    .unwrap()
                    .clone();

                let prototypical_properties_factory: Box<
                    dyn Fn(
                        std::rc::Rc<crate::RuntimePropertiesStackFrame>,
                        Option<std::rc::Rc<ExpandedNode>>,
                    )
                        -> Option<std::rc::Rc<RefCell<pax_runtime_api::pax_value::PaxAny>>>,
                > = Box::new(move |stack_frame, expanded_node| {
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
            manifest.get_inline_properties(containing_component_type_id, &node);
        manifest
            .merge_component_self_timelines_with_properties(&node.type_id, &mut inline_properties);
        let base_defined_properties = inline_properties.clone();
        let base_defined_properties_for_common = base_defined_properties.clone();
        let properties_tnd = node.clone();
        let properties_component_settings = containing_component_settings.clone();
        args.prototypical_properties = crate::PropertiesInit::Factory(Box::new(
            move |stack_frame, expanded_node| {
                let defined_properties = resolve_defined_properties_for_node(
                    &properties_tnd,
                    &base_defined_properties,
                    &properties_component_settings,
                    expanded_node.as_ref(),
                );
                if let Some(expanded_node) = expanded_node {
                    let outer_ref = expanded_node.properties.borrow();
                    let rc = Rc::clone(&outer_ref);
                    let mut inner_ref = (*rc).borrow_mut();
                    (node_component_descriptor.apply_defined_properties)(
                        node_component_descriptor.typed_descriptor,
                        &mut inner_ref,
                        &defined_properties,
                        &stack_frame,
                    );
                    return None;
                }

                let mut properties = (node_component_descriptor.create_properties)();
                (node_component_descriptor.apply_defined_properties)(
                    node_component_descriptor.typed_descriptor,
                    &mut properties,
                    &defined_properties,
                    &stack_frame,
                );
                Some(Rc::new(RefCell::new(properties)))
            },
        ));

        // update common properties from tnd
        let common_tnd = node.clone();
        let common_component_settings = containing_component_settings;
        args.prototypical_common_properties = crate::CommonPropertiesInit::Factory(Box::new(
            move |stack_frame, expanded_node| {
                let defined_properties = resolve_defined_properties_for_node(
                    &common_tnd,
                    &base_defined_properties_for_common,
                    &common_component_settings,
                    expanded_node.as_ref(),
                );
                if let Some(expanded_node) = expanded_node {
                    update_existing_common_properties(
                        &expanded_node,
                        &defined_properties,
                        &stack_frame,
                    );
                    return None;
                }
                Some(create_new_common_properties(
                    &defined_properties,
                    &stack_frame,
                ))
            },
        ));

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

fn resolve_property<T: CoercionRules + PropertyValue>(
    name: &str,
    defined_properties: &BTreeMap<String, ValueDefinition>,
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> Property<Option<T>> {
    let Some(value_def) = defined_properties.get(name) else {
        return Property::default();
    };
    let cloned_stack = stack.clone();
    let resolved_property: Property<Option<T>> = match value_def.clone() {
        pax_manifest::ValueDefinition::LiteralValue(lv) => {
            let val = T::try_coerce(lv).unwrap_or_else(|err| {
                log::warn!("Failed to coerce new value for property. Error: {:?}", err);
                Default::default()
            });
            Property::new_with_name(Some(val), name)
        }
        pax_manifest::ValueDefinition::Timeline(track) => {
            build_timeline_property(name, &track, cloned_stack.clone())
        }
        pax_manifest::ValueDefinition::Transition(transition) => {
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
            let name = &info.expression.to_string();
            Property::computed_with_name(
                move || {
                    let new_value = info
                        .expression
                        .compute(cloned_stack.clone())
                        .unwrap_or_else(|err| {
                            log::warn!("Failed to compute expression: {:?}", err);
                            Default::default()
                        });
                    let coerced = T::try_coerce(new_value.clone()).unwrap_or_else(|err| {
                        log::warn!("Failed to coerce new value for property. Error: {:?}", err);
                        Default::default()
                    });
                    Some(coerced)
                },
                &dependents,
                name,
            )
        }
        pax_manifest::ValueDefinition::Identifier(ident) => {
            let property = if let Some(p) = stack.resolve_symbol_as_erased_property(&ident.name) {
                Property::new_from_untyped(p.clone())
            } else {
                log::warn!("Failed to resolve symbol {}", ident.name);
                return Default::default();
            };
            let untyped = property.untyped();
            Property::computed_with_name(
                move || {
                    let new_value = property.get();
                    Some(new_value)
                },
                &[untyped],
                &ident.name,
            )
        }
        _ => unreachable!("Invalid value definition for {}", stringify!($prop_name)),
    };
    resolved_property
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
        ValueDefinition::Timeline(track) => {
            if let Some(playhead) = &track.playhead {
                collect_value_definition_dependencies(playhead, stack, dependents);
            } else if let Some(property) =
                stack.resolve_symbol_as_erased_property("$frames_elapsed")
            {
                dependents.push(property);
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
            if let Some(starting_value) = &transition.starting_value {
                collect_value_definition_dependencies(starting_value, stack, dependents);
            }
            for track in [&transition.enter, &transition.exit].into_iter().flatten() {
                if let Some(playhead) = &track.playhead {
                    collect_value_definition_dependencies(playhead, stack, dependents);
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

fn evaluate_value_definition_to_pax_value(
    value_definition: &ValueDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> Option<PaxValue> {
    match value_definition {
        ValueDefinition::LiteralValue(value) => Some(value.clone()),
        ValueDefinition::Block(block) => evaluate_literal_block_to_pax_value(block, stack),
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

fn timeline_total_frames(track: &TimelineTrackDefinition) -> f64 {
    let mut max_frame = track.frames.unwrap_or_default() as f64;
    let mut uses_percent_markers = false;

    for keyframe in track.keyframes() {
        match keyframe.marker {
            TimelineMarker::Frame(frame) => max_frame = max_frame.max(frame as f64),
            TimelineMarker::Percent(_) => uses_percent_markers = true,
        }
    }

    if uses_percent_markers {
        max_frame.max(track.frames.unwrap_or(100) as f64)
    } else {
        max_frame
    }
}

fn timeline_marker_to_frame(marker: &TimelineMarker, total_frames: f64) -> f64 {
    match marker {
        TimelineMarker::Frame(frame) => *frame as f64,
        TimelineMarker::Percent(percent) => total_frames * (*percent / 100.0),
    }
}

fn timeline_value_passes_coercion<T: CoercionRules>(value: &PaxValue) -> bool {
    T::try_coerce(value.clone()).is_ok()
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
    Value(PaxValue),
    Interpolated {
        current: PaxValue,
        next: PaxValue,
        easing: Option<String>,
        progress: f64,
    },
}

fn sample_timeline_playhead(
    track: &TimelineTrackDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
    total_frames: f64,
) -> f64 {
    let raw_playhead = track
        .playhead
        .as_ref()
        .and_then(|playhead| evaluate_value_definition_to_pax_value(playhead, stack))
        .and_then(|value| Numeric::try_coerce(value).ok())
        .map(|value| value.to_float())
        .or_else(|| {
            stack
                .resolve_symbol_as_variable("$frames_elapsed")
                .and_then(|variable| {
                    Numeric::try_coerce(variable.get_as_pax_value())
                        .ok()
                        .map(|value| value.to_float())
                })
        })
        .unwrap_or_default();

    let repeat = track.repeat.unwrap_or(true);
    if !repeat {
        return raw_playhead.clamp(0.0, total_frames.max(0.0));
    }

    let cycle_len = (total_frames + 1.0).max(1.0);
    raw_playhead.rem_euclid(cycle_len)
}

fn resolve_timeline_sample(
    track: &TimelineTrackDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
    is_valid_value: fn(&PaxValue) -> bool,
) -> Option<TimelineSample> {
    let total_frames = timeline_total_frames(track);
    let repeat = track.repeat.unwrap_or(true);
    let sample_frame = sample_timeline_playhead(track, stack, total_frames);

    let mut resolved_keyframes: Vec<ResolvedTimelineKeyframe> = track
        .keyframes()
        .enumerate()
        .filter_map(|(source_order, keyframe): (usize, &TimelineKeyframe)| {
            let value = evaluate_valid_timeline_value(&keyframe.value, stack, is_valid_value)?;
            Some(ResolvedTimelineKeyframe {
                frame: timeline_marker_to_frame(&keyframe.marker, total_frames),
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
        return Some(TimelineSample::Value(
            track
                .starting_value
                .as_ref()
                .and_then(|value| evaluate_valid_timeline_value(value, stack, is_valid_value))
                .unwrap_or_else(|| first_keyframe.value.clone()),
        ));
    }

    for keyframes in resolved_keyframes.windows(2) {
        let current = &keyframes[0];
        let next = &keyframes[1];
        if sample_frame <= next.frame {
            if sample_frame <= current.frame {
                return Some(TimelineSample::Value(current.value.clone()));
            }
            let span = next.frame - current.frame;
            if span <= f64::EPSILON {
                return Some(TimelineSample::Value(next.value.clone()));
            }
            let progress = (sample_frame - current.frame) / span;
            return Some(TimelineSample::Interpolated {
                current: current.value.clone(),
                next: next.value.clone(),
                easing: current.easing.clone(),
                progress,
            });
        }
    }

    let last_keyframe = resolved_keyframes.last()?;
    if sample_frame <= last_keyframe.frame || !repeat || total_frames <= last_keyframe.frame {
        return Some(TimelineSample::Value(last_keyframe.value.clone()));
    }

    let loop_target = loop_target_value(track, first_keyframe, stack, is_valid_value);
    let span = total_frames - last_keyframe.frame;
    if span <= f64::EPSILON {
        return Some(TimelineSample::Value(loop_target));
    }
    let progress = (sample_frame - last_keyframe.frame) / span;
    Some(TimelineSample::Interpolated {
        current: last_keyframe.value.clone(),
        next: loop_target,
        easing: last_keyframe.easing.clone(),
        progress,
    })
}

fn sample_timeline_track<T: CoercionRules + PropertyValue>(
    track: &TimelineTrackDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> Option<T> {
    match resolve_timeline_sample(track, stack, timeline_value_passes_coercion::<T>)? {
        TimelineSample::Value(value) => T::try_coerce(value).ok(),
        TimelineSample::Interpolated {
            current,
            next,
            easing,
            progress,
        } => {
            let current = T::try_coerce(current).ok()?;
            let next = T::try_coerce(next).ok()?;
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
    } else if let Some(property) = stack.resolve_symbol_as_erased_property("$frames_elapsed") {
        dependents.push(property);
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
        move || sample_timeline_track(&cloned_track, &cloned_stack).unwrap_or_default(),
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
) -> Option<T> {
    let mut track = track?.clone();
    if track.starting_value.is_none() {
        track.starting_value = transition.starting_value.clone();
    }
    sample_timeline_track(&track, stack)
}

fn sample_transition_property<T: CoercionRules + PropertyValue>(
    transition: &TransitionDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> Option<T> {
    let phase = stack
        .resolve_symbol_as_variable(TRANSITION_PHASE_SYMBOL)
        .and_then(|variable| Numeric::try_coerce(variable.get_as_pax_value()).ok())
        .map(|value| value.to_int() as u64)
        .unwrap_or_default();

    match phase {
        TRANSITION_PHASE_ENTER => {
            sample_transition_track(transition, transition.enter.as_ref(), stack)
                .or_else(|| coerce_transition_starting_value(transition, stack))
        }
        TRANSITION_PHASE_EXIT => {
            sample_transition_track(transition, transition.exit.as_ref(), stack)
                .or_else(|| sample_transition_track(transition, transition.enter.as_ref(), stack))
                .or_else(|| coerce_transition_starting_value(transition, stack))
        }
        _ => coerce_transition_starting_value(transition, stack),
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
    Property::computed_with_name(
        move || sample_transition_property(&cloned_transition, &cloned_stack).unwrap_or_default(),
        &dependents,
        name,
    )
}

pub fn apply_component_property<T>(
    property: &mut Property<T>,
    name: &str,
    value_definition: &ValueDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
    timeline_stack: Rc<RuntimePropertiesStackFrame>,
    build_block: fn(&LiteralBlockDefinition, Rc<RuntimePropertiesStackFrame>) -> Property<T>,
) where
    T: CoercionRules + PropertyValue,
{
    match value_definition {
        ValueDefinition::LiteralValue(lv) => {
            let value = T::try_coerce(lv.clone()).unwrap_or_else(|err| {
                log::warn!("Failed to coerce new value for property. Error: {:?}", err);
                Default::default()
            });
            property.replace_with(Property::new_with_name(value, name));
        }
        ValueDefinition::DoubleBinding(identifier) => {
            if let Some(untyped_property) =
                stack.resolve_symbol_as_erased_property(&identifier.name)
            {
                *property = Property::new_from_untyped(untyped_property.clone());
            } else {
                log::warn!("Failed to resolve identifier: {}", &identifier.name);
            }
        }
        ValueDefinition::Identifier(ident) => {
            if let Some(variable) = stack.resolve_symbol_as_variable(&ident.name) {
                let name = ident.name.clone();
                let untyped = variable.get_untyped_property().clone();
                let cloned_variable = variable.clone();
                *property = Property::computed_with_name(
                    move || {
                        let new_value = cloned_variable.get_as_pax_value();
                        T::try_coerce(new_value).unwrap_or_else(|err| {
                            log::warn!("Failed to coerce new value for property. Error: {:?}", err);
                            Default::default()
                        })
                    },
                    &[untyped],
                    &name,
                );
            } else {
                log::warn!("Failed to resolve symbol {}", ident.name);
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
            *property = Property::computed_with_name(
                move || {
                    let new_value = cloned_ast
                        .compute(cloned_stack.clone())
                        .unwrap_or_else(|_| {
                            log::warn!("Failed to compute expr: {}", expression_label);
                            Default::default()
                        });
                    T::try_coerce(new_value.clone()).unwrap_or_else(|err| {
                        log::warn!("Failed to coerce new value for property. Error: {:?}", err);
                        Default::default()
                    })
                },
                &dependents,
                name,
            );
        }
        ValueDefinition::Timeline(track) => {
            *property = build_timeline_property(name, track, timeline_stack);
        }
        ValueDefinition::Block(block) => {
            property.replace_with(build_block(block, stack.clone()));
        }
        _ => unreachable!("Invalid value definition for {name}"),
    }
}

#[cfg(test)]
mod timeline_tests {
    use super::build_timeline_property;
    use super::build_transition_property;
    use crate::RuntimePropertiesStackFrame;
    use pax_manifest::cartridge_generation::{
        TRANSITION_PHASE_ENTER, TRANSITION_PHASE_EXIT, TRANSITION_PHASE_SYMBOL,
        TRANSITION_PLAYHEAD_SYMBOL,
    };
    use pax_manifest::{
        PaxIdentifier, TimelineKeyframe, TimelineMarker, TimelineTrackDefinition,
        TimelineTrackElement, Token, TransitionDefinition, ValueDefinition,
    };
    use pax_runtime_api::{PaxValue, Property, Variable};
    use std::collections::HashMap;
    use std::rc::Rc;

    fn build_stack(frames_elapsed: &Property<u64>) -> Rc<RuntimePropertiesStackFrame> {
        let scope: HashMap<String, Variable> = vec![(
            "$frames_elapsed".to_string(),
            Variable::new_from_typed_property(frames_elapsed.clone()),
        )]
        .into_iter()
        .collect();
        RuntimePropertiesStackFrame::new(scope)
    }

    #[test]
    fn timeline_property_loops_over_declared_frame_range() {
        let frames_elapsed = Property::new(0_u64);
        let stack = build_stack(&frames_elapsed);
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
            frames: Some(100),
            repeat: Some(true),
            starting_value: None,
            use_local_property_scope: false,
        };
        let property = build_timeline_property::<f64>("progress", &track, stack);

        assert_eq!(property.get(), 0.0);
        frames_elapsed.set(50);
        assert_eq!(property.get(), 50.0);
        frames_elapsed.set(100);
        assert_eq!(property.get(), 100.0);
        frames_elapsed.set(101);
        assert_eq!(property.get(), 0.0);
    }

    #[test]
    fn timeline_property_holds_starting_value_before_first_keyframe() {
        let frames_elapsed = Property::new(0_u64);
        let stack = build_stack(&frames_elapsed);
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
            frames: Some(100),
            repeat: Some(false),
            starting_value: Some(Box::new(ValueDefinition::LiteralValue(PaxValue::Numeric(
                5.0.into(),
            )))),
            use_local_property_scope: false,
        };
        let property = build_timeline_property::<f64>("progress", &track, stack);

        assert_eq!(property.get(), 5.0);
        frames_elapsed.set(25);
        assert_eq!(property.get(), 5.0);
        frames_elapsed.set(75);
        assert_eq!(property.get(), 5.0);
        frames_elapsed.set(100);
        assert_eq!(property.get(), 0.0);
    }

    #[test]
    fn timeline_property_interpolates_rotation_tracks() {
        let frames_elapsed = Property::new(0_u64);
        let stack = build_stack(&frames_elapsed);
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
            frames: Some(100),
            repeat: Some(true),
            starting_value: None,
            use_local_property_scope: false,
        };
        let property =
            build_timeline_property::<pax_runtime_api::Rotation>("rotate", &track, stack);

        assert!((property.get().get_as_degrees() - (-4.0)).abs() < 0.0001);
        frames_elapsed.set(50);
        assert!((property.get().get_as_degrees() - 2.0).abs() < 0.0001);
        frames_elapsed.set(100);
        assert!((property.get().get_as_degrees() - 8.0).abs() < 0.0001);
    }

    #[test]
    fn timeline_property_samples_from_bound_playhead_property() {
        let frames_elapsed = Property::new(0_u64);
        let playhead = Property::new(0.0_f64);
        let scope: HashMap<String, Variable> = vec![
            (
                "$frames_elapsed".to_string(),
                Variable::new_from_typed_property(frames_elapsed.clone()),
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
            frames: Some(10),
            repeat: Some(false),
            starting_value: None,
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
            frames: Some(10),
            repeat: Some(false),
            starting_value: None,
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
            frames: Some(10),
            repeat: Some(false),
            starting_value: None,
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
    fn timeline_property_skips_keyframes_that_fail_typed_coercion() {
        let frames_elapsed = Property::new(0_u64);
        let stack = build_stack(&frames_elapsed);
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
            frames: Some(100),
            repeat: Some(false),
            starting_value: None,
            use_local_property_scope: false,
        };
        let property = build_timeline_property::<f64>("progress", &track, stack);

        frames_elapsed.set(75);
        assert!((property.get() - 75.0).abs() < 0.0001);
    }
}

pub fn update_existing_common_properties(
    expanded_node: &Rc<ExpandedNode>,
    defined_properties: &BTreeMap<String, pax_manifest::ValueDefinition>,
    stack_frame: &Rc<RuntimePropertiesStackFrame>,
) {
    let expanded_node = borrow!(**expanded_node);
    let outer_ref = expanded_node.common_properties.borrow();
    let rc = Rc::clone(&outer_ref);
    let inner_ref = (*rc).borrow_mut();
    let mut cp = inner_ref;

    update_common_properties(&mut cp, defined_properties, stack_frame);
}

fn create_id_property(
    defined_properties: &BTreeMap<String, pax_manifest::ValueDefinition>,
) -> Property<Option<String>> {
    let id = defined_properties.get("id");
    Property::new(
        if let Some(pax_manifest::ValueDefinition::Identifier(pax_identifier)) = id {
            Some(pax_identifier.name.clone())
        } else {
            None
        },
    )
}

pub fn create_new_common_properties(
    defined_properties: &BTreeMap<String, pax_manifest::ValueDefinition>,
    stack_frame: &Rc<RuntimePropertiesStackFrame>,
) -> Rc<RefCell<CommonProperties>> {
    Rc::new(RefCell::new(CommonProperties {
        id: create_id_property(defined_properties),
        x: resolve_property("x", defined_properties, stack_frame),
        y: resolve_property("y", defined_properties, stack_frame),
        width: resolve_property("width", defined_properties, stack_frame),
        height: resolve_property("height", defined_properties, stack_frame),
        scale_x: resolve_property("scale_x", defined_properties, stack_frame),
        scale_y: resolve_property("scale_y", defined_properties, stack_frame),
        skew_x: resolve_property("skew_x", defined_properties, stack_frame),
        skew_y: resolve_property("skew_y", defined_properties, stack_frame),
        rotate: resolve_property("rotate", defined_properties, stack_frame),
        transform: resolve_property("transform", defined_properties, stack_frame),
        opacity: resolve_property("opacity", defined_properties, stack_frame),
        layout_role: resolve_property("layout_role", defined_properties, stack_frame),
        anchor_x: resolve_property("anchor_x", defined_properties, stack_frame),
        anchor_y: resolve_property("anchor_y", defined_properties, stack_frame),
        unclippable: resolve_property("unclippable", defined_properties, stack_frame),
        _raycastable: resolve_property("_raycastable", defined_properties, stack_frame),
        _suspended: resolve_property("_suspended", defined_properties, stack_frame),
    }))
}

fn update_common_properties(
    cp: &mut CommonProperties,
    defined_properties: &BTreeMap<String, pax_manifest::ValueDefinition>,
    stack_frame: &Rc<RuntimePropertiesStackFrame>,
) {
    cp.id.replace_with(create_id_property(defined_properties));
    cp.x.replace_with(resolve_property("x", defined_properties, stack_frame));
    cp.y.replace_with(resolve_property("y", defined_properties, stack_frame));
    cp.width
        .replace_with(resolve_property("width", defined_properties, stack_frame));
    cp.height
        .replace_with(resolve_property("height", defined_properties, stack_frame));
    cp.scale_x
        .replace_with(resolve_property("scale_x", defined_properties, stack_frame));
    cp.scale_y
        .replace_with(resolve_property("scale_y", defined_properties, stack_frame));
    cp.skew_x
        .replace_with(resolve_property("skew_x", defined_properties, stack_frame));
    cp.skew_y
        .replace_with(resolve_property("skew_y", defined_properties, stack_frame));
    cp.rotate
        .replace_with(resolve_property("rotate", defined_properties, stack_frame));
    cp.transform.replace_with(resolve_property(
        "transform",
        defined_properties,
        stack_frame,
    ));
    cp.opacity
        .replace_with(resolve_property("opacity", defined_properties, stack_frame));
    cp.layout_role.replace_with(resolve_property(
        "layout_role",
        defined_properties,
        stack_frame,
    ));
    cp.anchor_x.replace_with(resolve_property(
        "anchor_x",
        defined_properties,
        stack_frame,
    ));
    cp.anchor_y.replace_with(resolve_property(
        "anchor_y",
        defined_properties,
        stack_frame,
    ));
    cp.unclippable.replace_with(resolve_property(
        "unclippable",
        defined_properties,
        stack_frame,
    ));
    cp._raycastable.replace_with(resolve_property(
        "_raycastable",
        defined_properties,
        stack_frame,
    ));
    cp._suspended.replace_with(resolve_property(
        "_suspended",
        defined_properties,
        stack_frame,
    ));
}
