use_RefCell!();
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use pax_manifest::{PaxManifest, TemplateNodeDefinition, TemplateNodeId, TypeId, ValueDefinition};
use pax_manifest::PaxType::Slot;
use pax_runtime_api::{borrow_mut, CommonProperties, Numeric, Property, use_RefCell};
use pax_runtime_api::pax_value::{PaxAny, ToFromPaxAny};
use pax_runtime_api::properties::UntypedProperty;
use crate::{ComponentInstance, ExpressionContext, ExpressionTable, HandlerRegistry, InstanceNode, InstantiationArgs, RuntimePropertiesStackFrame};
use crate::api::NodeContext;

pub trait PaxCartridge {
    fn instantiate_expression_table(&self) -> HashMap<usize, Box<dyn Fn(ExpressionContext) -> PaxAny>>;
}
pub trait DefinitionToInstanceTraverser {
    fn new(manifest: pax_manifest::PaxManifest) -> Self where Self: Sized;

    fn get_manifest(&self) ->  &pax_manifest::PaxManifest;

    #[cfg(feature = "designtime")]
    fn get_designtime_manager(&self, project_query: String) -> std::option::Option<std::rc::Rc<RefCell<pax_designtime::DesigntimeManager>>>;

    fn get_main_component(&self) -> std::rc::Rc<crate::ComponentInstance> {
        let main_component_type_id = {
            let manifest = self.get_manifest();
            manifest.main_component_type_id.clone()
        };
        let args = self.build_component_args(&main_component_type_id);
        let main_component = crate::ComponentInstance::instantiate(args);
        main_component
    }

    fn get_component(&mut self, type_id: &pax_manifest::TypeId) -> std::rc::Rc<dyn crate::rendering::InstanceNode> {
        let factory = self.get_component_factory(type_id).expect("Failed to get component factory");
        let args = self.build_component_args(type_id);
        factory.build_component(args)
    }

    fn get_component_factory(&self, type_id: &pax_manifest::TypeId) -> Option<Box<dyn crate::ComponentFactory>>;

    fn build_component_args(&self, type_id: &pax_manifest::TypeId) -> crate::rendering::InstantiationArgs {
        let manifest = self.get_manifest();
        let property_names = manifest.get_all_property_names(type_id);
        if let None = manifest.components.get(type_id) {
            panic!("Components with type_id {} not found in manifest", type_id);
        }
        let component = manifest.components.get(type_id).unwrap();
        let factory = self.get_component_factory(&type_id).expect(&format!("No component factory for type: {}", type_id));
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
                match node.type_id.get_pax_type(){
                    pax_manifest::PaxType::If | pax_manifest::PaxType::Slot | pax_manifest::PaxType::Repeat => {
                        instances.push(self.build_control_flow(type_id, &node_id));
                    },
                    pax_manifest::PaxType::Comment => continue,
                    _ => {
                        instances.push(self.build_template_node(type_id, &node_id));
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

    fn build_control_flow(&self, containing_component_type_id: &pax_manifest::TypeId, node_id: &pax_manifest::TemplateNodeId) -> std::rc::Rc<dyn crate::rendering::InstanceNode> {

        let manifest = self.get_manifest();
        let prototypical_common_properties_factory = Box::new(|_,_| std::rc::Rc::new(RefCell::new(CommonProperties::default())));

        let containing_component = manifest.components.get(containing_component_type_id).unwrap();
        let containing_template = containing_component.template.as_ref().unwrap();
        let tnd = containing_template.get_node(node_id).unwrap();
        let unique_identifier = pax_manifest::UniqueTemplateNodeIdentifier::build(containing_component_type_id.clone(), node_id.clone());

        let children = self.build_children(containing_component_type_id, &node_id);
        match tnd.type_id.get_pax_type(){
            pax_manifest::PaxType::If => {
                let expression_info = tnd
                    .control_flow_settings
                    .as_ref()
                    .unwrap()
                    .condition_expression_info
                    .as_ref()
                    .unwrap();
                let vtable_id = expression_info.vtable_id;
                let dep_symbols = expression_info.dependencies.clone();
                let prototypical_properties_factory : Box<dyn Fn(std::rc::Rc<crate::RuntimePropertiesStackFrame>, std::rc::Rc<crate::ExpressionTable>) -> std::rc::Rc<RefCell<pax_runtime_api::pax_value::PaxAny>>> = Box::new(move |stack_frame, table| std::rc::Rc::new(RefCell::new( {
                    let mut properties = crate::ConditionalProperties::default();
                    let cloned_table = table.clone();
                    let cloned_stack = stack_frame.clone();

                    let mut dependencies = Vec::new();
                    for dependency in &dep_symbols {
                        if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                            dependencies.push(p);
                        } else {
                            panic!("Failed to resolve symbol {}", dependency);
                        }
                    }

                    properties.boolean_expression =  Property::computed_with_name(move || {
                        let new_value_wrapped: pax_runtime_api::pax_value::PaxAny = cloned_table.compute_vtable_value(&cloned_stack, vtable_id);
                        let coerced = new_value_wrapped.try_coerce::<bool>().map_err(|e| format!("expr with vtable_id {} failed: {}", vtable_id, e)).unwrap();
                        coerced
                    }, &dependencies, "conditional (if) expr");
                    properties.to_pax_any()
                })));
                crate::ConditionalInstance::instantiate(crate::rendering::InstantiationArgs {
                    prototypical_common_properties_factory,
                    prototypical_properties_factory,
                    handler_registry: None,
                    component_template: None,
                    children: Some(RefCell::new(children)),
                    template_node_identifier: Some(unique_identifier),
                    properties_scope_factory: None,
                })
            },
            pax_manifest::PaxType::Slot => {
                let expression_info = tnd
                    .control_flow_settings
                    .as_ref()
                    .unwrap()
                    .slot_index_expression_info
                    .as_ref()
                    .unwrap();

                let vtable_id = expression_info.vtable_id;
                let dep_symbols = expression_info.dependencies.clone();

                let prototypical_properties_factory : Box<dyn Fn(std::rc::Rc<crate::RuntimePropertiesStackFrame>, std::rc::Rc<crate::ExpressionTable>) -> std::rc::Rc<RefCell<pax_runtime_api::pax_value::PaxAny>>>  = Box::new(move |stack_frame, table| std::rc::Rc::new(RefCell::new( {
                    let mut properties = crate::Slot::default();
                    let cloned_table = table.clone();
                    let cloned_stack = stack_frame.clone();

                    let mut dependencies = Vec::new();
                    for dependency in &dep_symbols {
                        if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                            dependencies.push(p);
                        } else {
                            panic!("Failed to resolve symbol {}", dependency);
                        }
                    }
                    properties.index = Property::computed_with_name(move || {
                        let new_value_wrapped: pax_runtime_api::pax_value::PaxAny = cloned_table.compute_vtable_value(&cloned_stack, vtable_id);
                        let coerced = new_value_wrapped.try_coerce::<Numeric>().unwrap();
                        coerced
                    }, &dependencies, "slot index");
                    properties.to_pax_any()
                })));
                crate::SlotInstance::instantiate(crate::rendering::InstantiationArgs {
                    prototypical_common_properties_factory,
                    prototypical_properties_factory,
                    handler_registry: None,
                    component_template: None,
                    children: Some(RefCell::new(children)),
                    template_node_identifier: Some(unique_identifier),
                    properties_scope_factory: None,
                })
            },
            pax_manifest::PaxType::Repeat => {
                let rsd = tnd
                    .control_flow_settings
                    .as_ref()
                    .unwrap()
                    .repeat_source_definition
                    .clone()
                    .unwrap();
                let rpd = tnd
                    .control_flow_settings
                    .as_ref()
                    .unwrap()
                    .repeat_predicate_definition
                    .clone()
                    .unwrap();
                let expression_info = rsd.expression_info.as_ref().unwrap();
                let vtable_id = expression_info.vtable_id.clone();
                let dep_symbols = expression_info.dependencies.clone();
                let prototypical_properties_factory : Box<dyn Fn(std::rc::Rc<crate::RuntimePropertiesStackFrame>, std::rc::Rc<crate::ExpressionTable>) -> std::rc::Rc<RefCell<pax_runtime_api::pax_value::PaxAny>>> = Box::new(move |stack_frame,table| std::rc::Rc::new(RefCell::new( {
                    let mut properties = crate::RepeatProperties::default();

                    let mut dependencies = Vec::new();
                    for dependency in &dep_symbols {
                        if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                            dependencies.push(p);
                        } else {
                            panic!("Failed to resolve symbol {}", dependency);
                        }
                    }

                    properties.source_expression_vec =
                        if let Some(t) = &rsd.symbolic_binding {
                            let cloned_table = table.clone();
                            let cloned_stack = stack_frame.clone();
                            Some(
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: pax_runtime_api::pax_value::PaxAny = cloned_table.compute_vtable_value(&cloned_stack, vtable_id);
                                    let coerced = new_value_wrapped.try_coerce::<Vec<std::rc::Rc<RefCell<pax_runtime_api::pax_value::PaxAny>>>>().unwrap();
                                    coerced
                                }, &dependencies, "repeat source vec")
                            )
                        } else {
                            None
                        };

                    properties.source_expression_range =
                        if let Some(t) = &rsd.range_expression_paxel {
                            let cloned_table = table.clone();
                            let cloned_stack = stack_frame.clone();
                            Some(
                                Property::computed_with_name(move || {
                                    let new_value_wrapped: pax_runtime_api::pax_value::PaxAny = cloned_table.compute_vtable_value(&cloned_stack, vtable_id);
                                    let coerced = new_value_wrapped.try_coerce::<std::ops::Range::<isize>>().unwrap();
                                    coerced
                                }, &dependencies, "repeat source range")
                            )
                        } else {
                            None
                        };

                    let (elem, index) = match &rpd {
                        pax_manifest::ControlFlowRepeatPredicateDefinition::ElemId(token) => {
                            (Some(token.raw_value.clone()), None)
                        },
                        pax_manifest::ControlFlowRepeatPredicateDefinition::ElemIdIndexId(t1,t2) => {
                            (Some(t1.raw_value.clone()), Some(t2.raw_value.clone()))
                        }
                    };
                    properties.iterator_i_symbol = index;
                    properties.iterator_elem_symbol = elem;
                    properties.to_pax_any()
                })));
                crate::RepeatInstance::instantiate(crate::rendering::InstantiationArgs {
                    prototypical_common_properties_factory,
                    prototypical_properties_factory,
                    handler_registry: None,
                    component_template: None,
                    children: Some(RefCell::new(children)),
                    template_node_identifier: Some(unique_identifier),
                    properties_scope_factory: None
                })
            },
            _ => {
                unreachable!("Unexpected control flow type {}", tnd.type_id)
            }

        }

    }

    fn build_children(&self, containing_component_type_id: &pax_manifest::TypeId, node_id: &pax_manifest::TemplateNodeId) -> Vec<std::rc::Rc<dyn crate::rendering::InstanceNode>> {
        let manifest = self.get_manifest();
        let containing_component = manifest.components.get(containing_component_type_id).unwrap();
        let containing_template = containing_component.template.as_ref().unwrap();
        let children = containing_template.get_children(node_id);

        let mut children_instances = Vec::new();
        for child_id in &children.unwrap_or_default() {
            let child = containing_template.get_node(&child_id).unwrap();
            match child.type_id.get_pax_type() {
                pax_manifest::PaxType::If | pax_manifest::PaxType::Slot | pax_manifest::PaxType::Repeat  => {
                    children_instances.push(self.build_control_flow(containing_component_type_id, &child_id));
                },
                pax_manifest::PaxType::Comment => continue,
                _ => {
                    children_instances.push(self.build_template_node(containing_component_type_id, child_id));
                }
            }
        }
        children_instances
    }

    fn build_template_node(&self, containing_component_type_id: &pax_manifest::TypeId, node_id: &pax_manifest::TemplateNodeId) -> std::rc::Rc<dyn crate::rendering::InstanceNode> {
        let manifest = self.get_manifest();

        let containing_component = manifest.components.get(containing_component_type_id).unwrap();
        let containing_template = containing_component.template.as_ref().unwrap();
        let node = containing_template.get_node(node_id).unwrap();
        let containing_component_factory = self.get_component_factory(containing_component_type_id).unwrap();

        let mut args = self.build_component_args(&node.type_id);
        let node_component_factory = self.get_component_factory(&node.type_id).unwrap();

        // update handlers from tnd
        let handlers_from_tnd = manifest.get_inline_event_handlers(node);
        let updated_registry = if let Some(registry) = args.handler_registry {
            containing_component_factory.add_inline_handlers(handlers_from_tnd, registry)
        } else {
            containing_component_factory.add_inline_handlers(handlers_from_tnd, std::rc::Rc::new(RefCell::new(crate::HandlerRegistry::default())))
        };

        args.handler_registry = Some(updated_registry);

        // update properties from tnd
        let inline_properties = manifest.get_inline_properties(containing_component_type_id, node);
        let updated_properties = node_component_factory.build_inline_properties(inline_properties.clone());
        args.prototypical_properties_factory = updated_properties;

        // update common properties from tnd
        let updated_common_properties = node_component_factory.build_inline_common_properties(inline_properties);
        args.prototypical_common_properties_factory = updated_common_properties;


        args.children = Some(RefCell::new(self.build_children(containing_component_type_id, node_id)));
        args.template_node_identifier = Some(pax_manifest::UniqueTemplateNodeIdentifier::build(containing_component_type_id.clone(), node_id.clone()));

        node_component_factory.build_component(args)
    }


    fn get_template_node_by_id(&self, id: &str) -> Option<std::rc::Rc<dyn crate::rendering::InstanceNode>> {
        let manifest = self.get_manifest();
        let main_component_type_id = manifest.main_component_type_id.clone();
        let main_component = manifest.components.get(&main_component_type_id).unwrap();
        let template = main_component.template.as_ref().unwrap();
        for node_id in template.get_ids() {
            if let Some(found) = self.recurse_get_template_node_by_id(id, &main_component_type_id, node_id) {
                return Some(self.build_template_node(&found.0, &found.1))
            }
        }
        None
    }

    fn check_for_id_in_template_node(&self, id: &str, tnd: &pax_manifest::TemplateNodeDefinition) -> bool {
        if let Some(settings) = &tnd.settings {
            for setting in settings {
                if let pax_manifest::SettingElement::Setting(token, value) = setting {
                    if &token.raw_value == "id" {
                        if let pax_manifest::ValueDefinition::LiteralValue(lv) = value {
                            if lv.raw_value == id {
                                return true;
                            }
                        }

                    }
                }
            }
        }
        false
    }

    fn recurse_get_template_node_by_id<'a>(&'a self, id: &str, containing_component_type_id: &'a pax_manifest::TypeId, node_id: &'a pax_manifest::TemplateNodeId) -> Option<(pax_manifest::TypeId, pax_manifest::TemplateNodeId)>{
        let manifest = self.get_manifest();
        let containing_component = manifest.components.get(containing_component_type_id).unwrap();
        let containing_template = containing_component.template.as_ref().unwrap();
        let tnd = containing_template.get_node(node_id).unwrap();

        if self.check_for_id_in_template_node(id, tnd) {
            return Some((containing_component_type_id.clone(), node_id.clone()));
        }

        if let Some(component) = &manifest.components.get(&tnd.type_id){
            if let Some(template) = &component.template {
                for node_id in template.get_ids() {
                    if let Some(found) = self.recurse_get_template_node_by_id(id, &tnd.type_id, node_id) {
                        return Some(found.clone());
                    }
                }
            }
        }
        None
    }
}

// Used to DRY the building of common properties for ComponentFactory#build_inline_common_properties
macro_rules! resolve_property {
    ($cp:expr, $key:expr, $value:expr, $prop_name:ident, $type:ty, $stack_frame:expr, $table:expr) => {
        if $key == stringify!($prop_name) {
            let resolved_property: Property<Option<$type>> = match $value.clone() {
                pax_manifest::ValueDefinition::LiteralValue(lv) => {
                    let val = pax_manifest::deserializer::from_pax_try_coerce::<$type>(&lv.raw_value)
                        .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                    Property::new_with_name(Some(val), &lv.raw_value)
                },
                pax_manifest::ValueDefinition::DoubleBinding(token,info) => {
                    let identifier = token.token_value.clone();
                    let untyped_property = $stack_frame.resolve_symbol_as_erased_property(&identifier).expect("failed to resolve identifier");
                    Property::new_from_untyped(untyped_property.clone())
                },
                pax_manifest::ValueDefinition::Expression(token, info) | pax_manifest::ValueDefinition::Identifier(token, info) => {
                    if let Some(info) = info {
                        let mut dependents = vec![];
                        for dependency in &info.dependencies {
                            if let Some(p) = $stack_frame.resolve_symbol_as_erased_property(dependency) {
                                dependents.push(p);
                            } else {
                                panic!("Failed to resolve symbol {}", dependency);
                            }
                        }
                        let cloned_stack = $stack_frame.clone();
                        let cloned_table = $table.clone();
                        Property::computed_with_name(move || {
                            let new_value_wrapped: pax_runtime_api::pax_value::PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                            let coerced = new_value_wrapped.try_coerce::<$type>().unwrap();
                            Some(coerced)
                        }, &dependents, &token.raw_value)
                    } else {
                        unreachable!("No info for expression")
                    }
                },
                _ => unreachable!("Invalid value definition for {}", stringify!($prop_name))
            };
            $cp.$prop_name = resolved_property;
        }
    };
}

pub trait ComponentFactory {
    /// Returns the default CommonProperties factory
    fn build_default_common_properties(&self) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<CommonProperties>>>{
        Box::new(|_,_| Rc::new(RefCell::new(CommonProperties::default())))
    }

    /// Returns the default properties factory for this component
    fn build_default_properties(&self) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>>;

    fn build_inline_common_properties(&self, defined_properties: std::collections::BTreeMap<String,pax_manifest::ValueDefinition>) -> Box<dyn Fn(std::rc::Rc<RuntimePropertiesStackFrame>, std::rc::Rc<ExpressionTable>) -> std::rc::Rc<RefCell<CommonProperties>>> {
        Box::new(move |stack_frame, table| std::rc::Rc::new(RefCell::new({
            let mut cp = CommonProperties::default();
            for (key, value) in &defined_properties {
                resolve_property!(cp, key, value, id, String, stack_frame, table);
                resolve_property!(cp, key, value, x, pax_runtime_api::Size, stack_frame, table);
                resolve_property!(cp, key, value, y, pax_runtime_api::Size, stack_frame, table);
                resolve_property!(cp, key, value, scale_x, pax_runtime_api::Size, stack_frame, table);
                resolve_property!(cp, key, value, scale_y, pax_runtime_api::Size, stack_frame, table);
                resolve_property!(cp, key, value, skew_x, pax_runtime_api::Rotation, stack_frame, table);
                resolve_property!(cp, key, value, skew_y, pax_runtime_api::Rotation, stack_frame, table);
                resolve_property!(cp, key, value, anchor_x, pax_runtime_api::Size, stack_frame, table);
                resolve_property!(cp, key, value, anchor_y, pax_runtime_api::Size, stack_frame, table);
                resolve_property!(cp, key, value, rotate, pax_runtime_api::Rotation, stack_frame, table);
                resolve_property!(cp, key, value, transform, pax_runtime_api::Transform2D, stack_frame, table);
                resolve_property!(cp, key, value, width, pax_runtime_api::Size, stack_frame, table);
                resolve_property!(cp, key, value, height, pax_runtime_api::Size, stack_frame, table);
            }
            cp.clone()
        })))
    }

    /// Returns the properties factory based on the defined properties
    fn build_inline_properties(&self, defined_properties: BTreeMap<String,ValueDefinition>) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>>;

    /// Returns the requested closure for the handler registry based on the defined handlers for this component
    /// The argument type is extrapolated based on how the handler was used in the initial compiled template
    fn build_handler(&self, fn_name: &str) -> fn(Rc<RefCell<PaxAny>>, &NodeContext, Option::<PaxAny>);

    /// Returns the handler registry based on the defined handlers for this component
    fn build_component_handlers(&self, handlers: Vec<(String, Vec<String>)>) -> Rc<RefCell<HandlerRegistry>>;

    // Takes a hander registry and adds the given inline handlers to it
    fn add_inline_handlers(&self, handlers: Vec<(String, String)>, registry: Rc<RefCell<HandlerRegistry>>) -> Rc<RefCell<HandlerRegistry>>;

    // Calls the instantion function for the component
    fn build_component(&self, args: InstantiationArgs) -> Rc<dyn InstanceNode>;

    // Returns the property scope for the component
    fn get_properties_scope_factory(&self) -> Box<dyn Fn(Rc<RefCell<PaxAny>>) -> HashMap<String, UntypedProperty>> {
        Box::new(|_| HashMap::new())
    }
}




