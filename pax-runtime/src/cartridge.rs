use_RefCell!();
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use pax_manifest::{PaxManifest, TemplateNodeDefinition, TemplateNodeId, TypeId, ValueDefinition};
use pax_runtime_api::{CommonProperties, Property, use_RefCell};
use pax_runtime_api::pax_value::PaxAny;
use pax_runtime_api::properties::UntypedProperty;
use crate::{ComponentInstance, ExpressionContext, ExpressionTable, HandlerRegistry, InstanceNode, InstantiationArgs, RuntimePropertiesStackFrame};
use crate::api::NodeContext;



pub trait PaxCartridge {
    fn instantiate_expression_table(&self) -> HashMap<usize, Box<dyn Fn(ExpressionContext) -> PaxAny>>;
    // fn instantiate_main_component(&self) -> Rc<ComponentInstance>;
    // fn get_definition_to_instance_traverser(&self) -> Box<dyn DefinitionToInstanceTraverser>;
}
pub trait DefinitionToInstanceTraverser {
    fn new(manifest: PaxManifest) -> Self where Self: Sized;
    fn get_main_component(&mut self) -> Rc<ComponentInstance>;
    fn get_manifest(&self) -> &PaxManifest;
    fn get_template_node_by_id(&self, id: &str) -> Option<Rc<dyn InstanceNode>>;
    fn get_component(&mut self, type_id: &TypeId) -> Rc<dyn InstanceNode>;
    fn get_component_factory(&self, type_id: &TypeId) -> Option<Box<dyn ComponentFactory>>;
    fn build_component_args(&self, type_id: &TypeId) -> InstantiationArgs;
    fn build_control_flow(&self, containing_component_type_id: &TypeId, node_id: &TemplateNodeId) -> Rc<dyn InstanceNode>;
    fn build_children(&self, containing_component_type_id: &TypeId, node_id: &TemplateNodeId) -> Vec<Rc<dyn InstanceNode>>;
    fn build_template_node(&self, containing_component_type_id: &TypeId, node_id: &TemplateNodeId) -> Rc<dyn InstanceNode>;
    fn check_for_id_in_template_node(&self, id: &str, tnd: &TemplateNodeDefinition) -> bool;
    fn recurse_get_template_node_by_id<'a>(&'a self, id: &str, containing_component_type_id: &'a TypeId, node_id: &'a TemplateNodeId) -> Option<(TypeId, TemplateNodeId)>;
}


pub trait ComponentFactory {
    /// Returns the default CommonProperties factory
    fn build_default_common_properties(&self) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<CommonProperties>>>{
        Box::new(|_,_| Rc::new(RefCell::new(CommonProperties::default())))
    }

    /// Returns the default properties factory for this component
    fn build_default_properties(&self) -> Box<dyn Fn(Rc<RuntimePropertiesStackFrame>, Rc<ExpressionTable>) -> Rc<RefCell<PaxAny>>>;

    /// Returns the CommonProperties factory based on the defined properties
    fn build_inline_common_properties(&self, defined_properties: std::collections::BTreeMap<String,pax_manifest::ValueDefinition>) ->Box<dyn Fn(std::rc::Rc<RuntimePropertiesStackFrame>, std::rc::Rc<ExpressionTable>) -> std::rc::Rc<RefCell<CommonProperties>>> {
        Box::new(move |stack_frame, table| std::rc::Rc::new(RefCell::new({
            let mut cp = CommonProperties::default();
            for (key, value) in &defined_properties {
                match key.as_str() {
                    "id" => {
                        let resolved_property: Property<Option<String>> = match value.clone() {
                            pax_manifest::ValueDefinition::LiteralValue(lv) => {
                                let val = pax_manifest::deserializer::from_pax_try_coerce::<String>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            pax_manifest::ValueDefinition::DoubleBinding(token,info) => {
                                let identifier = token.token_value.clone();
                                let untyped_property = stack_frame.resolve_symbol_as_erased_property(&identifier).expect("failed to resolve identifier");
                                Property::new_from_untyped(untyped_property.clone())
                            },
                            pax_manifest::ValueDefinition::Expression(token, info) | pax_manifest::ValueDefinition::Identifier(token, info) =>
                                {
                                    if let Some(info) = info {
                                        let mut dependents = vec![];
                                        for dependency in &info.dependencies {
                                            if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                dependents.push(p);
                                            } else {
                                                panic!("Failed to resolve symbol {}", dependency);
                                            }
                                        }
                                        let cloned_stack = stack_frame.clone();
                                        let cloned_table = table.clone();
                                        Property::computed_with_name(move || {
                                            let new_value_wrapped: pax_runtime_api::pax_value::PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                            let coerced = new_value_wrapped.try_coerce::<String>().unwrap();
                                            Some(coerced)
                                        }, &dependents, &token.raw_value)
                                    } else {
                                        unreachable!("No info for expression")
                                    }
                                },
                            _ => unreachable!("Invalid value definition for id")
                        };
                        cp.id = resolved_property;
                    },

                    "x" => {
                        let resolved_property: Property<Option<pax_runtime_api::Size>> = match value.clone() {
                            pax_manifest::ValueDefinition::LiteralValue(lv) => {
                                let val = pax_manifest::deserializer::from_pax_try_coerce::<pax_runtime_api::Size>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            pax_manifest::ValueDefinition::Expression(token, info) | pax_manifest::ValueDefinition::Identifier(token, info) =>
                                {
                                    if let Some(info) = info {
                                        let mut dependents = vec![];
                                        for dependency in &info.dependencies {
                                            if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                dependents.push(p);
                                            } else {
                                                panic!("Failed to resolve symbol {}", dependency);
                                            }
                                        }
                                        let cloned_stack = stack_frame.clone();
                                        let cloned_table = table.clone();
                                        Property::computed_with_name(move || {
                                            let new_value_wrapped: pax_runtime_api::pax_value::PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                            let coerced = new_value_wrapped.try_coerce::<pax_runtime_api::Size>().unwrap();
                                            Some(coerced)
                                        }, &dependents, &token.raw_value)
                                    } else {
                                        unreachable!("No info for expression")
                                    }
                                },
                            _ => unreachable!("Invalid value definition for x")
                        };
                        cp.x = resolved_property;
                    },

                    "y" => {
                        let resolved_property: Property<Option<pax_runtime_api::Size>> = match value.clone() {
                            pax_manifest::ValueDefinition::LiteralValue(lv) => {
                                let val = pax_manifest::deserializer::from_pax_try_coerce::<pax_runtime_api::Size>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            pax_manifest::ValueDefinition::Expression(token, info) | pax_manifest::ValueDefinition::Identifier(token, info) =>
                                {
                                    if let Some(info) = info {
                                        let mut dependents = vec![];
                                        for dependency in &info.dependencies {
                                            if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                dependents.push(p);
                                            } else {
                                                panic!("Failed to resolve symbol {}", dependency);
                                            }
                                        }
                                        let cloned_stack = stack_frame.clone();
                                        let cloned_table = table.clone();
                                        Property::computed_with_name(move || {
                                            let new_value_wrapped: pax_runtime_api::pax_value::PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                            let coerced = new_value_wrapped.try_coerce::<pax_runtime_api::Size>().unwrap();
                                            Some(coerced)
                                        }, &dependents, &token.raw_value)
                                    } else {
                                        unreachable!("No info for expression")
                                    }
                                },
                            _ => unreachable!("Invalid value definition for y")
                        };
                        cp.y = resolved_property;
                    },

                    "scale_x" => {
                        let resolved_property: Property<Option<pax_runtime_api::Size>> = match value.clone() {
                            pax_manifest::ValueDefinition::LiteralValue(lv) => {
                                let val = pax_manifest::deserializer::from_pax_try_coerce::<pax_runtime_api::Size>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            pax_manifest::ValueDefinition::Expression(token, info) | pax_manifest::ValueDefinition::Identifier(token, info) =>
                                {
                                    if let Some(info) = info {
                                        let mut dependents = vec![];
                                        for dependency in &info.dependencies {
                                            if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                dependents.push(p);
                                            } else {
                                                panic!("Failed to resolve symbol {}", dependency);
                                            }
                                        }
                                        let cloned_stack = stack_frame.clone();
                                        let cloned_table = table.clone();
                                        Property::computed_with_name(move || {
                                            let new_value_wrapped: pax_runtime_api::pax_value::PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                            let coerced = new_value_wrapped.try_coerce::<pax_runtime_api::Size>().unwrap();
                                            Some(coerced)
                                        }, &dependents, &token.raw_value)
                                    } else {
                                        unreachable!("No info for expression")
                                    }
                                },
                            _ => unreachable!("Invalid value definition for scale_x")
                        };
                        cp.scale_x = resolved_property;
                    },

                    "scale_y" => {
                        let resolved_property: Property<Option<pax_runtime_api::Size>> = match value.clone() {
                            pax_manifest::ValueDefinition::LiteralValue(lv) => {
                                let val = pax_manifest::deserializer::from_pax_try_coerce::<pax_runtime_api::Size>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            pax_manifest::ValueDefinition::Expression(token, info) | pax_manifest::ValueDefinition::Identifier(token, info) =>
                                {
                                    if let Some(info) = info {
                                        let mut dependents = vec![];
                                        for dependency in &info.dependencies {
                                            if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                dependents.push(p);
                                            } else {
                                                panic!("Failed to resolve symbol {}", dependency);
                                            }
                                        }
                                        let cloned_stack = stack_frame.clone();
                                        let cloned_table = table.clone();
                                        Property::computed_with_name(move || {
                                            let new_value_wrapped: pax_runtime_api::pax_value::PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                            let coerced = new_value_wrapped.try_coerce::<pax_runtime_api::Size>().unwrap();
                                            Some(coerced)
                                        }, &dependents, &token.raw_value)
                                    } else {
                                        unreachable!("No info for expression")
                                    }
                                },
                            _ => unreachable!("Invalid value definition for scale_y")
                        };
                        cp.scale_y = resolved_property;
                    },

                    "skew_x" => {
                        let resolved_property: Property<Option<pax_runtime_api::Rotation>> = match value.clone() {
                            pax_manifest::ValueDefinition::LiteralValue(lv) => {
                                let val = pax_manifest::deserializer::from_pax_try_coerce::<pax_runtime_api::Rotation>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            pax_manifest::ValueDefinition::Expression(token, info) | pax_manifest::ValueDefinition::Identifier(token, info) =>
                                {
                                    if let Some(info) = info {
                                        let mut dependents = vec![];
                                        for dependency in &info.dependencies {
                                            if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                dependents.push(p);
                                            } else {
                                                panic!("Failed to resolve symbol {}", dependency);
                                            }
                                        }
                                        let cloned_stack = stack_frame.clone();
                                        let cloned_table = table.clone();
                                        Property::computed_with_name(move || {
                                            let new_value_wrapped: pax_runtime_api::pax_value::PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                            let coerced = new_value_wrapped.try_coerce::<pax_runtime_api::Rotation>().unwrap();
                                            Some(coerced)
                                        }, &dependents, &token.raw_value)
                                    } else {
                                        unreachable!("No info for expression")
                                    }
                                },
                            _ => unreachable!("Invalid value definition for skew_x")
                        };
                        cp.skew_x = resolved_property;
                    },

                    "skew_y" => {
                        let resolved_property: Property<Option<pax_runtime_api::Rotation>> = match value.clone() {
                            pax_manifest::ValueDefinition::LiteralValue(lv) => {
                                let val = pax_manifest::deserializer::from_pax_try_coerce::<pax_runtime_api::Rotation>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            pax_manifest::ValueDefinition::Expression(token, info) | pax_manifest::ValueDefinition::Identifier(token, info) =>
                                {
                                    if let Some(info) = info {
                                        let mut dependents = vec![];
                                        for dependency in &info.dependencies {
                                            if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                dependents.push(p);
                                            } else {
                                                panic!("Failed to resolve symbol {}", dependency);
                                            }
                                        }
                                        let cloned_stack = stack_frame.clone();
                                        let cloned_table = table.clone();
                                        Property::computed_with_name(move || {
                                            let new_value_wrapped: pax_runtime_api::pax_value::PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                            let coerced = new_value_wrapped.try_coerce::<pax_runtime_api::Rotation>().unwrap();
                                            Some(coerced)
                                        }, &dependents, &token.raw_value)
                                    } else {
                                        unreachable!("No info for expression")
                                    }
                                },
                            _ => unreachable!("Invalid value definition for skew_y")
                        };
                        cp.skew_y = resolved_property;
                    },

                    "anchor_x" => {
                        let resolved_property: Property<Option<pax_runtime_api::Size>> = match value.clone() {
                            pax_manifest::ValueDefinition::LiteralValue(lv) => {
                                let val = pax_manifest::deserializer::from_pax_try_coerce::<pax_runtime_api::Size>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            pax_manifest::ValueDefinition::Expression(token, info) | pax_manifest::ValueDefinition::Identifier(token, info) =>
                                {
                                    if let Some(info) = info {
                                        let mut dependents = vec![];
                                        for dependency in &info.dependencies {
                                            if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                dependents.push(p);
                                            } else {
                                                panic!("Failed to resolve symbol {}", dependency);
                                            }
                                        }
                                        let cloned_stack = stack_frame.clone();
                                        let cloned_table = table.clone();
                                        Property::computed_with_name(move || {
                                            let new_value_wrapped: pax_runtime_api::pax_value::PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                            let coerced = new_value_wrapped.try_coerce::<pax_runtime_api::Size>().unwrap();
                                            Some(coerced)
                                        }, &dependents, &token.raw_value)
                                    } else {
                                        unreachable!("No info for expression")
                                    }
                                },
                            _ => unreachable!("Invalid value definition for anchor_x")
                        };
                        cp.anchor_x = resolved_property;
                    },

                    "anchor_y" => {
                        let resolved_property: Property<Option<pax_runtime_api::Size>> = match value.clone() {
                            pax_manifest::ValueDefinition::LiteralValue(lv) => {
                                let val = pax_manifest::deserializer::from_pax_try_coerce::<pax_runtime_api::Size>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            pax_manifest::ValueDefinition::Expression(token, info) | pax_manifest::ValueDefinition::Identifier(token, info) =>
                                {
                                    if let Some(info) = info {
                                        let mut dependents = vec![];
                                        for dependency in &info.dependencies {
                                            if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                dependents.push(p);
                                            } else {
                                                panic!("Failed to resolve symbol {}", dependency);
                                            }
                                        }
                                        let cloned_stack = stack_frame.clone();
                                        let cloned_table = table.clone();
                                        Property::computed_with_name(move || {
                                            let new_value_wrapped: pax_runtime_api::pax_value::PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                            let coerced = new_value_wrapped.try_coerce::<pax_runtime_api::Size>().unwrap();
                                            Some(coerced)
                                        }, &dependents, &token.raw_value)
                                    } else {
                                        unreachable!("No info for expression")
                                    }
                                },
                            _ => unreachable!("Invalid value definition for anchor_y")
                        };
                        cp.anchor_y = resolved_property;
                    },

                    "rotate" => {
                        let resolved_property: Property<Option<pax_runtime_api::Rotation>> = match value.clone() {
                            pax_manifest::ValueDefinition::LiteralValue(lv) => {
                                let val = pax_manifest::deserializer::from_pax_try_coerce::<pax_runtime_api::Rotation>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            pax_manifest::ValueDefinition::Expression(token, info) | pax_manifest::ValueDefinition::Identifier(token, info) =>
                                {
                                    if let Some(info) = info {
                                        let mut dependents = vec![];
                                        for dependency in &info.dependencies {
                                            if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                dependents.push(p);
                                            } else {
                                                panic!("Failed to resolve symbol {}", dependency);
                                            }
                                        }
                                        let cloned_stack = stack_frame.clone();
                                        let cloned_table = table.clone();
                                        Property::computed_with_name(move || {
                                            let new_value_wrapped: pax_runtime_api::pax_value::PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                            let coerced = new_value_wrapped.try_coerce::<pax_runtime_api::Rotation>().unwrap();
                                            Some(coerced)
                                        }, &dependents, &token.raw_value)
                                    } else {
                                        unreachable!("No info for expression")
                                    }
                                },
                            _ => unreachable!("Invalid value definition for rotate")
                        };
                        cp.rotate = resolved_property;
                    },

                    "transform" => {
                        let resolved_property: Property<Option<pax_runtime_api::Transform2D>> = match value.clone() {
                            pax_manifest::ValueDefinition::LiteralValue(lv) => {
                                let val = pax_manifest::deserializer::from_pax_try_coerce::<pax_runtime_api::Transform2D>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            pax_manifest::ValueDefinition::Expression(token, info) | pax_manifest::ValueDefinition::Identifier(token, info) =>
                                {
                                    if let Some(info) = info {
                                        let mut dependents = vec![];
                                        for dependency in &info.dependencies {
                                            if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                dependents.push(p);
                                            } else {
                                                panic!("Failed to resolve symbol {}", dependency);
                                            }
                                        }
                                        let cloned_stack = stack_frame.clone();
                                        let cloned_table = table.clone();
                                        Property::computed_with_name(move || {
                                            let new_value_wrapped: pax_runtime_api::pax_value::PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                            let coerced = new_value_wrapped.try_coerce::<pax_runtime_api::Transform2D>().unwrap();
                                            Some(coerced)
                                        }, &dependents, &token.raw_value)
                                    } else {
                                        unreachable!("No info for expression")
                                    }
                                },
                            _ => unreachable!("Invalid value definition for transform")
                        };
                        cp.transform = resolved_property;
                    },

                    "width" => {
                        let resolved_property: Property<Option<pax_runtime_api::Size>> = match value.clone() {
                            pax_manifest::ValueDefinition::LiteralValue(lv) => {
                                let val = pax_manifest::deserializer::from_pax_try_coerce::<pax_runtime_api::Size>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            pax_manifest::ValueDefinition::Expression(token, info) | pax_manifest::ValueDefinition::Identifier(token, info) =>
                                {
                                    if let Some(info) = info {
                                        let mut dependents = vec![];
                                        for dependency in &info.dependencies {
                                            if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                dependents.push(p);
                                            } else {
                                                panic!("Failed to resolve symbol {}", dependency);
                                            }
                                        }
                                        let cloned_stack = stack_frame.clone();
                                        let cloned_table = table.clone();
                                        Property::computed_with_name(move || {
                                            let new_value_wrapped: pax_runtime_api::pax_value::PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                            let coerced = new_value_wrapped.try_coerce::<pax_runtime_api::Size>().unwrap();
                                            Some(coerced)
                                        }, &dependents, &token.raw_value)
                                    } else {
                                        unreachable!("No info for expression")
                                    }
                                },
                            _ => unreachable!("Invalid value definition for width")
                        };
                        cp.width = resolved_property;
                    },

                    "height" => {
                        let resolved_property: Property<Option<pax_runtime_api::Size>> = match value.clone() {
                            pax_manifest::ValueDefinition::LiteralValue(lv) => {
                                let val = pax_manifest::deserializer::from_pax_try_coerce::<pax_runtime_api::Size>(&lv.raw_value)
                                    .map_err(|e| format!("failed to read {}: {}", &lv.raw_value, e)).unwrap();
                                Property::new_with_name(Some(val), &lv.raw_value)
                            },
                            pax_manifest::ValueDefinition::Expression(token, info) | pax_manifest::ValueDefinition::Identifier(token, info) =>
                                {
                                    if let Some(info) = info {
                                        let mut dependents = vec![];
                                        for dependency in &info.dependencies {
                                            if let Some(p) = stack_frame.resolve_symbol_as_erased_property(dependency) {
                                                dependents.push(p);
                                            } else {
                                                panic!("Failed to resolve symbol {}", dependency);
                                            }
                                        }
                                        let cloned_stack = stack_frame.clone();
                                        let cloned_table = table.clone();
                                        Property::computed_with_name(move || {
                                            let new_value_wrapped: pax_runtime_api::pax_value::PaxAny = cloned_table.compute_vtable_value(&cloned_stack, info.vtable_id.clone());
                                            let coerced = new_value_wrapped.try_coerce::<pax_runtime_api::Size>().unwrap();
                                            Some(coerced)
                                        }, &dependents, &token.raw_value)
                                    } else {
                                        unreachable!("No info for expression")
                                    }
                                },
                            _ => unreachable!("Invalid value definition for height")
                        };
                        cp.height = resolved_property;
                    },

                    _ => {}
                }
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




