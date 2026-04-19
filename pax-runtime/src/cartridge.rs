use_RefCell!();
use crate::api::NodeContext;
use crate::{
    ConditionalProperties, ExpandedNode, HandlerRegistry, InstanceNode, InstantiationArgs,
    ReusableInstanceNodeArgs, RuntimePropertiesStackFrame,
};
use pax_language::Computable;
use pax_manifest::{
    ExpressionInfo, LiteralBlockDefinition, SettingElement, TimelineKeyframe, TimelineMarker,
    TimelineTrackDefinition, TimelineTrackElement, TypeId, ValueDefinition,
};
use pax_message::borrow;
use pax_runtime_api::pax_value::{CoercionRules, PaxAny, ToFromPaxAny};
use pax_runtime_api::properties::{PropertyValue, UntypedProperty};
use pax_runtime_api::{
    use_RefCell, CommonProperties, EasingCurve, Numeric, PaxValue, Property, Variable,
};
use serde::de::DeserializeOwned;
use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

pub trait PaxCartridge {}

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
        let factory = self
            .get_component_factory(type_id)
            .expect("Failed to get component factory");
        let args = self.build_component_args(type_id);
        factory.build_component(args)
    }

    fn get_component_factory(
        &self,
        type_id: &pax_manifest::TypeId,
    ) -> Option<Box<dyn crate::ComponentFactory>>;

    fn build_component_args(
        &self,
        type_id: &pax_manifest::TypeId,
    ) -> crate::rendering::InstantiationArgs {
        let manifest: std::cell::Ref<pax_manifest::PaxManifest> = self.get_manifest();
        if let None = manifest.components.get(type_id) {
            panic!("Components with type_id {} not found in manifest", type_id);
        }
        let component = manifest.components.get(type_id).unwrap();
        let factory = self
            .get_component_factory(&type_id)
            .expect(&format!("No component factory for type: {}", type_id));
        let prototypical_common_properties_factory = factory.build_default_common_properties();
        let mut prototypical_properties_factory = factory.build_default_properties();

        // pull handlers for this component
        let handlers = manifest.get_component_handlers(type_id);
        let handler_registry = Some(factory.build_component_handlers(handlers));

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
            prototypical_properties_factory =
                factory.build_inline_properties(component_self_timeline_properties);
        }

        crate::rendering::InstantiationArgs {
            prototypical_common_properties_factory,
            prototypical_properties_factory,
            handler_registry,
            component_template,
            children: None,
            template_node_identifier: None,
            properties_scope_factory: Some(factory.get_properties_scope_factory()),
        }
    }

    fn build_control_flow(
        &self,
        containing_component_type_id: &pax_manifest::TypeId,
        node_id: &pax_manifest::TemplateNodeId,
        prior_node: Option<ReusableInstanceNodeArgs>,
    ) -> std::rc::Rc<dyn crate::rendering::InstanceNode> {
        let manifest = self.get_manifest();
        let prototypical_common_properties_factory =
            Box::new(|_, _| Some(std::rc::Rc::new(RefCell::new(CommonProperties::default()))));

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
                        prototypical_common_properties_factory,
                        prototypical_properties_factory,
                        handler_registry: None,
                        component_template: None,
                        children: Some(children),
                        template_node_identifier: Some(unique_identifier),
                        properties_scope_factory: None,
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
                    prototypical_common_properties_factory,
                    prototypical_properties_factory,
                    handler_registry: None,
                    component_template: None,
                    children: Some(children),
                    template_node_identifier: Some(unique_identifier),
                    properties_scope_factory: None,
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
                        properties.to_pax_any()
                    })))
                });
                crate::RepeatInstance::instantiate(crate::rendering::InstantiationArgs {
                    prototypical_common_properties_factory,
                    prototypical_properties_factory,
                    handler_registry: None,
                    component_template: None,
                    children: Some(children),
                    template_node_identifier: Some(unique_identifier),
                    properties_scope_factory: None,
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
        let containing_component_factory = self
            .get_component_factory(containing_component_type_id)
            .unwrap();

        let mut args = self.build_component_args(&node.type_id);
        let node_component_factory = self.get_component_factory(&node.type_id).unwrap();

        if let Some(prior_node) = prior_node {
            args.handler_registry = prior_node.handler_registry;
            args.children = Some(prior_node.children);
            args.template_node_identifier = prior_node.template_node_identifier;
        } else {
            let handlers_from_tnd = manifest.get_inline_event_handlers(node);
            let updated_registry = if let Some(registry) = args.handler_registry {
                containing_component_factory.add_inline_handlers(handlers_from_tnd, registry)
            } else {
                containing_component_factory.add_inline_handlers(
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
        }

        // update properties from tnd
        let mut inline_properties =
            manifest.get_inline_properties(containing_component_type_id, node);
        manifest
            .merge_component_self_timelines_with_properties(&node.type_id, &mut inline_properties);
        let updated_properties =
            node_component_factory.build_inline_properties(inline_properties.clone());
        args.prototypical_properties_factory = updated_properties;

        // update common properties from tnd
        let updated_common_properties =
            node_component_factory.build_inline_common_properties(inline_properties);
        args.prototypical_common_properties_factory = updated_common_properties;

        node_component_factory.build_component(args)
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

fn resolve_property<T: CoercionRules + PropertyValue + DeserializeOwned>(
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
        ValueDefinition::Timeline(_) => None,
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

fn coerce_timeline_value<T: CoercionRules + PropertyValue>(
    value_definition: &ValueDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> Option<T> {
    let value = evaluate_value_definition_to_pax_value(value_definition, stack)?;
    T::try_coerce(value).ok()
}

fn loop_target_value<T: CoercionRules + PropertyValue>(
    track: &TimelineTrackDefinition,
    first_keyframe: &ResolvedTimelineKeyframe<T>,
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> T {
    if first_keyframe.frame == 0.0 {
        first_keyframe.value.clone()
    } else {
        track
            .starting_value
            .as_ref()
            .and_then(|value| coerce_timeline_value(value, stack))
            .unwrap_or_else(|| first_keyframe.value.clone())
    }
}

struct ResolvedTimelineKeyframe<T> {
    frame: f64,
    value: T,
    easing: Option<String>,
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

fn sample_timeline_track<T: CoercionRules + PropertyValue>(
    track: &TimelineTrackDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> Option<T> {
    let total_frames = timeline_total_frames(track);
    let repeat = track.repeat.unwrap_or(true);
    let sample_frame = sample_timeline_playhead(track, stack, total_frames);

    let mut resolved_keyframes: Vec<ResolvedTimelineKeyframe<T>> = track
        .keyframes()
        .filter_map(|keyframe: &TimelineKeyframe| {
            Some(ResolvedTimelineKeyframe {
                frame: timeline_marker_to_frame(&keyframe.marker, total_frames),
                value: coerce_timeline_value(&keyframe.value, stack)?,
                easing: keyframe
                    .easing
                    .as_ref()
                    .map(|token| token.token_value.clone()),
            })
        })
        .collect();

    resolved_keyframes.sort_by(|lhs, rhs| {
        lhs.frame
            .partial_cmp(&rhs.frame)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let first_keyframe = resolved_keyframes.first()?;

    if sample_frame < first_keyframe.frame {
        return track
            .starting_value
            .as_ref()
            .and_then(|value| coerce_timeline_value(value, stack))
            .or_else(|| Some(first_keyframe.value.clone()));
    }

    for keyframes in resolved_keyframes.windows(2) {
        let current = &keyframes[0];
        let next = &keyframes[1];
        if sample_frame <= next.frame {
            if sample_frame <= current.frame {
                return Some(current.value.clone());
            }
            let span = next.frame - current.frame;
            if span <= f64::EPSILON {
                return Some(next.value.clone());
            }
            let progress = (sample_frame - current.frame) / span;
            let curve = easing_curve_from_name(current.easing.as_deref());
            return Some(curve.interpolate(&current.value, &next.value, progress));
        }
    }

    let last_keyframe = resolved_keyframes.last()?;
    if sample_frame <= last_keyframe.frame || !repeat || total_frames <= last_keyframe.frame {
        return Some(last_keyframe.value.clone());
    }

    let loop_target = loop_target_value(track, first_keyframe, stack);
    let span = total_frames - last_keyframe.frame;
    if span <= f64::EPSILON {
        return Some(loop_target);
    }
    let progress = (sample_frame - last_keyframe.frame) / span;
    let curve = easing_curve_from_name(last_keyframe.easing.as_deref());
    Some(curve.interpolate(&last_keyframe.value, &loop_target, progress))
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

#[cfg(test)]
mod timeline_tests {
    use super::build_timeline_property;
    use crate::RuntimePropertiesStackFrame;
    use pax_manifest::{
        TimelineKeyframe, TimelineMarker, TimelineTrackDefinition, TimelineTrackElement, Token,
        ValueDefinition,
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
}

pub trait ComponentFactory {
    /// Returns the default CommonProperties factory
    fn build_default_common_properties(
        &self,
    ) -> Box<
        dyn Fn(
            Rc<RuntimePropertiesStackFrame>,
            Option<Rc<ExpandedNode>>,
        ) -> Option<Rc<RefCell<CommonProperties>>>,
    > {
        Box::new(|_, _| Some(Rc::new(RefCell::new(CommonProperties::default()))))
    }

    /// Returns the default properties factory for this component
    fn build_default_properties(
        &self,
    ) -> Box<
        dyn Fn(
            Rc<RuntimePropertiesStackFrame>,
            Option<Rc<ExpandedNode>>,
        ) -> Option<Rc<RefCell<PaxAny>>>,
    >;

    fn build_inline_common_properties(
        &self,
        defined_properties: BTreeMap<String, pax_manifest::ValueDefinition>,
    ) -> Box<
        dyn Fn(
            Rc<RuntimePropertiesStackFrame>,
            Option<Rc<ExpandedNode>>,
        ) -> Option<Rc<RefCell<CommonProperties>>>,
    > {
        Box::new(move |stack_frame, expanded_node| {
            if let Some(expanded_node) = &expanded_node {
                update_existing_common_properties(expanded_node, &defined_properties, &stack_frame);
                None
            } else {
                Some(create_new_common_properties(
                    &defined_properties,
                    &stack_frame,
                ))
            }
        })
    }

    /// Returns the properties factory based on the defined properties
    fn build_inline_properties(
        &self,
        defined_properties: BTreeMap<String, ValueDefinition>,
    ) -> Box<
        dyn Fn(
            Rc<RuntimePropertiesStackFrame>,
            Option<Rc<ExpandedNode>>,
        ) -> Option<Rc<RefCell<PaxAny>>>,
    >;

    /// Returns the requested closure for the handler registry based on the defined handlers for this component
    /// The argument type is extrapolated based on how the handler was used in the initial compiled template
    fn build_handler(&self, fn_name: &str)
        -> fn(Rc<RefCell<PaxAny>>, &NodeContext, Option<PaxAny>);

    /// Returns the handler registry based on the defined handlers for this component
    fn build_component_handlers(
        &self,
        handlers: Vec<(String, Vec<String>)>,
    ) -> Rc<RefCell<HandlerRegistry>>;

    // Takes a handler registry and adds the given inline handlers to it
    fn add_inline_handlers(
        &self,
        handlers: Vec<(String, String)>,
        registry: Rc<RefCell<HandlerRegistry>>,
    ) -> Rc<RefCell<HandlerRegistry>>;

    // Calls the instantiation function for the component
    fn build_component(&self, args: InstantiationArgs) -> Rc<dyn InstanceNode>;

    // Returns the property scope for the component
    fn get_properties_scope_factory(
        &self,
    ) -> Box<dyn Fn(Rc<RefCell<PaxAny>>) -> HashMap<String, Variable>> {
        Box::new(|_| HashMap::new())
    }
}

fn update_existing_common_properties(
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

fn create_new_common_properties(
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
