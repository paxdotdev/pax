use_RefCell!();
use crate::api::NodeContext;
use crate::{
    ConditionalProperties, ExpandedNode, HandlerRegistry, InstanceNode, InstantiationArgs,
    ReusableInstanceNodeArgs, RuntimePropertiesStackFrame,
};
use pax_lang::Computable;
use pax_manifest::{TypeId, ValueDefinition};
use pax_message::borrow;
use pax_runtime_api::pax_value::{CoercionRules, PaxAny, ToFromPaxAny};
use pax_runtime_api::properties::PropertyValue;
use pax_runtime_api::{use_RefCell, CommonProperties, Numeric, Property, Variable};
use serde::de::DeserializeOwned;
use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

pub trait PaxCartridge {}
pub trait DefinitionToInstanceTraverser {
    fn new(manifest: pax_manifest::PaxManifest) -> Self
    where
        Self: Sized;

    fn get_manifest(&self) -> std::cell::Ref<pax_manifest::PaxManifest>;

    #[cfg(any(feature = "designer", feature = "designtime"))]
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
        let prototypical_properties_factory = factory.build_default_properties();

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
                let expr_info = tnd
                    .control_flow_settings
                    .as_ref()
                    .unwrap()
                    .condition_expression
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
                        let cp = ConditionalProperties::mut_from_pax_any(&mut inner_ref).unwrap();
                        cp.boolean_expression
                            .replace_with(Property::computed_with_name(
                                move || {
                                    let new_value = expr_ast
                                        .compute(cloned_stack.clone())
                                        .unwrap_or_else(|err| {
                                            log::warn!("Failed to compute expression: {:?}", err);
                                            Default::default()
                                        });
                                    let coerced =
                                        bool::try_coerce(new_value).unwrap_or_else(|_e| {
                                            log::warn!(
                                                "Failed to parse boolean expression: {}",
                                                expr_ast
                                            );
                                            Default::default()
                                        });
                                    coerced
                                },
                                &dependencies,
                                "conditional (if) expr",
                            ));
                        return None;
                    }

                    Some(std::rc::Rc::new(RefCell::new({
                        let mut properties = crate::ConditionalProperties::default();
                        properties.boolean_expression = Property::computed_with_name(
                            move || {
                                let new_value = expr_ast.compute(cloned_stack.clone()).unwrap();
                                let coerced = bool::try_coerce(new_value).unwrap_or_else(|_e| {
                                    log::warn!("Failed to parse boolean expression: {}", expr_ast);
                                    Default::default()
                                });
                                coerced
                            },
                            &dependencies,
                            "conditional (if) expr",
                        );
                        properties.to_pax_any()
                    })))
                });
                crate::ConditionalInstance::instantiate(crate::rendering::InstantiationArgs {
                    prototypical_common_properties_factory,
                    prototypical_properties_factory,
                    handler_registry: None,
                    component_template: None,
                    children: Some(children),
                    template_node_identifier: Some(unique_identifier),
                    properties_scope_factory: None,
                })
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
        let inline_properties = manifest.get_inline_properties(containing_component_type_id, node);
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
