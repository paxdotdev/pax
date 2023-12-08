//! # Code Generation Module
//!
//! The `code_generation` module provides structures and functions for generating Pax Cartridges
//! from Pax Manifests. The `generate_and_overwrite_cartridge` function is the main entrypoint.

use crate::helpers::{HostCrateInfo, PKG_DIR_NAME};
use crate::parsing;
use itertools::Itertools;
use pax_runtime_api::CommonProperties;
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::str::FromStr;

use pax_manifest::{
    escape_identifier, ComponentDefinition, ExpressionSpec, HandlersBlockElement,
    LiteralBlockDefinition, MappedString, PaxManifest, SettingElement, TemplateNodeDefinition,
    Token, TypeDefinition, TypeTable, ValueDefinition,
};

use crate::errors::source_map::SourceMap;
use std::path::PathBuf;
use toml_edit::Item;

use self::templating::{
    press_template_codegen_cartridge_component_factory,
    press_template_codegen_cartridge_render_node_literal,
    TemplateArgsCodegenCartridgeComponentFactory, TemplateArgsCodegenCartridgeRenderNodeLiteral,
};

pub mod templating;

pub fn generate_and_overwrite_cartridge(
    pax_dir: &PathBuf,
    manifest: &PaxManifest,
    host_crate_info: &HostCrateInfo,
    source_map: &mut SourceMap,
) -> PathBuf {
    let target_dir = pax_dir.join(PKG_DIR_NAME).join("pax-cartridge");

    let target_cargo_full_path = fs::canonicalize(target_dir.join("Cargo.toml")).unwrap();
    let mut target_cargo_toml_contents =
        toml_edit::Document::from_str(&fs::read_to_string(&target_cargo_full_path).unwrap())
            .unwrap();

    //insert new entry pointing to userland crate, where `pax_app` is defined
    std::mem::swap(
        target_cargo_toml_contents["dependencies"]
            .get_mut(&host_crate_info.name)
            .unwrap(),
        &mut Item::from_str("{ path=\"../../..\" }").unwrap(),
    );

    //write patched Cargo.toml
    fs::write(
        &target_cargo_full_path,
        &target_cargo_toml_contents.to_string(),
    )
    .unwrap();

    const IMPORTS_BUILTINS: [&str; 26] = [
        "std::cell::RefCell",
        "std::collections::HashMap",
        "std::collections::VecDeque",
        "std::ops::Deref",
        "std::rc::Rc",
        "pax_runtime_api::PropertyInstance",
        "pax_runtime_api::PropertyLiteral",
        "pax_runtime_api::CommonProperties",
        "pax_core::ComponentInstance",
        "pax_core::RenderNodePtr",
        "pax_core::PropertyExpression",
        "pax_core::RenderNodePtrList",
        "pax_core::RenderTreeContext",
        "pax_core::ExpressionContext",
        "pax_core::PaxEngine",
        "pax_core::RenderNode",
        "pax_core::InstanceRegistry",
        "pax_core::HandlerRegistry",
        "pax_core::InstantiationArgs",
        "pax_core::ConditionalInstance",
        "pax_core::SlotInstance",
        "pax_core::StackFrame",
        "pax_core::pax_properties_coproduct::PropertiesCoproduct",
        "pax_core::pax_properties_coproduct::TypesCoproduct",
        "pax_core::repeat::RepeatInstance",
        "piet_common::RenderContext",
    ];

    let imports_builtins_set: HashSet<&str> = IMPORTS_BUILTINS.into_iter().collect();

    #[allow(non_snake_case)]
    let IMPORT_PREFIX = format!("{}::pax_reexports::", host_crate_info.identifier);

    let mut imports: Vec<String> = manifest
        .import_paths
        .iter()
        .map(|path| {
            if !imports_builtins_set.contains(&**path) {
                IMPORT_PREFIX.clone() + &path.replace("crate::", "")
            } else {
                "".to_string()
            }
        })
        .collect();

    imports.append(
        &mut IMPORTS_BUILTINS
            .into_iter()
            .map(|ib| ib.to_string())
            .collect::<Vec<String>>(),
    );

    let consts = vec![]; //TODO!

    let mut expression_specs: Vec<ExpressionSpec> = manifest
        .expression_specs
        .as_ref()
        .unwrap()
        .values()
        .map(|es: &ExpressionSpec| es.clone())
        .collect();
    expression_specs = expression_specs.iter().sorted().cloned().collect();

    let component_factories_literal = manifest
        .components
        .values()
        .into_iter()
        .filter(|cd| !cd.is_primitive && !cd.is_struct_only_component)
        .map(|cd| {
            generate_cartridge_component_factory_literal(manifest, cd, host_crate_info, source_map)
        })
        .collect();

    //press template into String
    let generated_lib_rs = templating::press_template_codegen_cartridge_lib(
        templating::TemplateArgsCodegenCartridgeLib {
            imports,
            consts,
            expression_specs,
            component_factories_literal,
        },
    );

    // Re: formatting the generated Rust code, see prior art at `_format_generated_lib_rs`
    let path = target_dir.join("src/lib.rs");
    fs::write(path.clone(), generated_lib_rs).unwrap();
    path
}

fn generate_cartridge_render_nodes_literal(
    rngc: &RenderNodesGenerationContext,
    host_crate_info: &HostCrateInfo,
    source_map: &mut SourceMap,
) -> String {
    let nodes =
        rngc.active_component_definition.template.as_ref().expect(
            "tried to generate render nodes literal for component, but template was undefined",
        );

    let implicit_root = nodes[&0].borrow();
    let children_literal: Vec<String> = implicit_root
        .child_ids
        .iter()
        .filter(|child_id| {
            let tnd_map = rngc.active_component_definition.template.as_ref().unwrap();
            let active_tnd = &tnd_map[*child_id];
            active_tnd.type_id != parsing::TYPE_ID_COMMENT
        })
        .map(|child_id| {
            let tnd_map = rngc.active_component_definition.template.as_ref().unwrap();
            let active_tnd = &tnd_map[child_id];
            recurse_generate_render_nodes_literal(rngc, active_tnd, host_crate_info, source_map)
        })
        .collect();

    children_literal.join(",")
}

fn generate_bound_handlers(
    inline_settings: Option<Vec<SettingElement>>,
    source_map: &mut SourceMap,
) -> Vec<(MappedString, MappedString)> {
    let mut ret: HashMap<MappedString, MappedString> = HashMap::new();
    if let Some(ref inline) = inline_settings {
        for e in inline.iter() {
            if let SettingElement::Setting(key, value) = e {
                if let ValueDefinition::EventBindingTarget(s) = value {
                    let key_source_map_id = source_map.insert(key.clone());
                    let key_mapped_string = source_map
                        .generate_mapped_string(key.token_value.clone(), key_source_map_id);

                    let value_source_map_id = source_map.insert(s.clone());
                    let value_mapped_string = source_map
                        .generate_mapped_string(s.token_value.clone(), value_source_map_id);

                    ret.insert(key_mapped_string, value_mapped_string);
                }
            }
        }
    };
    ret.into_iter().collect()
}

fn recurse_literal_block(
    block: LiteralBlockDefinition,
    type_definition: &TypeDefinition,
    host_crate_info: &HostCrateInfo,
    source_map: &mut SourceMap,
) -> String {
    let qualified_path = host_crate_info.import_prefix.to_string()
        + &type_definition.import_path.clone().replace("crate::", "");

    // Buffer to store the string representation of the struct
    let mut struct_representation = format!("\n{{ let mut ret = {}::default();", qualified_path);

    // Iterating through each (key, value) pair in the settings_key_value_pairs
    for (key, value_definition) in block.get_all_settings().iter() {
        let fully_qualified_type = host_crate_info.import_prefix.to_string()
            + &type_definition
                .property_definitions
                .iter()
                .find(|pd| &pd.name == &key.token_value)
                .expect(&format!(
                    "Property {} not found on type {}",
                    key.token_value, type_definition.type_id
                ))
                .type_id;

        let mut source_map_start_marker: Option<String> = None;
        let mut source_map_end_marker: Option<String> = None;

        let value_string = match value_definition {
            ValueDefinition::LiteralValue(value) => {
                let value_source_map_id = source_map.insert(value.clone());
                let value_mapped_string = source_map
                    .generate_mapped_string(value.token_value.clone(), value_source_map_id);
                source_map_start_marker = value_mapped_string.source_map_start_marker;
                source_map_end_marker = value_mapped_string.source_map_end_marker;
                format!(
                    "ret.{} = Box::new(PropertyLiteral::new(Into::<{}>::into({})));",
                    key.token_value, fully_qualified_type, value.token_value
                )
            }
            ValueDefinition::Expression(token, id) | ValueDefinition::Identifier(token, id) => {
                let value_source_map_id = source_map.insert(token.clone());
                let value_mapped_string = source_map
                    .generate_mapped_string(token.token_value.clone(), value_source_map_id);
                source_map_start_marker = value_mapped_string.source_map_start_marker;
                source_map_end_marker = value_mapped_string.source_map_end_marker;
                format!(
                    "ret.{} = Box::new(PropertyExpression::new({}));",
                    key.token_value,
                    id.expect(
                        format!(
                            "Tried to use expression but it wasn't compiled: {:?} with id: {:?}",
                            token.token_location.clone(),
                            id.clone()
                        )
                        .as_str()
                    )
                )
            }
            ValueDefinition::Block(inner_block) => format!(
                "ret.{} = Box::new(PropertyLiteral::new(Into::<{}>::into({})));",
                key.token_value,
                fully_qualified_type,
                recurse_literal_block(
                    inner_block.clone(),
                    type_definition,
                    host_crate_info,
                    source_map
                ),
            ),
            _ => {
                panic!("Incorrect value bound to inline setting")
            }
        };
        if let Some(source_map_start_marker) = source_map_start_marker {
            struct_representation.push_str(&format!("\n{}", source_map_start_marker));
        }

        struct_representation.push_str(&format!("\n{}", value_string));

        if let Some(source_map_end_marker) = source_map_end_marker {
            struct_representation.push_str(&format!("\n{}", source_map_end_marker));
        }
    }

    struct_representation.push_str("\n ret }");

    struct_representation
}

fn recurse_generate_render_nodes_literal(
    rngc: &RenderNodesGenerationContext,
    tnd: &TemplateNodeDefinition,
    host_crate_info: &HostCrateInfo,
    source_map: &mut SourceMap,
) -> String {
    //first recurse, populating children_literal : Vec<String>
    let children_literal: Vec<String> = tnd
        .child_ids
        .iter()
        .filter(|child_id| {
            let tnd_map = rngc.active_component_definition.template.as_ref().unwrap();
            let active_tnd = &tnd_map[*child_id];
            active_tnd.type_id != parsing::TYPE_ID_COMMENT
        })
        .map(|child_id| {
            let active_tnd = &rngc.active_component_definition.template.as_ref().unwrap()[child_id];
            recurse_generate_render_nodes_literal(rngc, active_tnd, host_crate_info, source_map)
        })
        .collect();

    //pull inline event binding and store into map
    let events = generate_bound_handlers(tnd.settings.clone(), source_map);
    let args = if tnd.type_id == parsing::TYPE_ID_REPEAT {
        // Repeat
        let rsd = tnd
            .control_flow_settings
            .as_ref()
            .unwrap()
            .repeat_source_definition
            .as_ref()
            .unwrap();
        let id = rsd.vtable_id.unwrap();

        let rse_vec = if let Some(t) = &rsd.symbolic_binding {
            let vec_source_id = source_map.insert(t.clone());
            source_map.generate_mapped_string(
                format!("Some(Box::new(PropertyExpression::new({})))", id),
                vec_source_id,
            )
        } else {
            MappedString::none()
        };

        let rse_range = if let Some(t) = &rsd.range_expression_paxel {
            let range_source_id = source_map.insert(t.clone());
            source_map.generate_mapped_string(
                format!("Some(Box::new(PropertyExpression::new({})))", id),
                range_source_id,
            )
        } else {
            MappedString::none()
        };

        let common_properties_literal = CommonProperties::get_default_properties_literal()
            .iter()
            .map(|(id, value)| {
                (
                    MappedString::new(id.clone()),
                    MappedString::new(value.clone()),
                )
            })
            .collect();

        TemplateArgsCodegenCartridgeRenderNodeLiteral {
            is_primitive: true,
            snake_case_type_id: "UNREACHABLE".into(),
            primitive_instance_import_path: Some("RepeatInstance".into()),
            properties_coproduct_variant: "None".to_string(),
            component_properties_struct: "None".to_string(),
            defined_properties: vec![],
            common_properties_literal,
            children_literal,
            slot_index_literal: MappedString::none(),
            conditional_boolean_expression_literal: MappedString::none(),
            pascal_identifier: rngc
                .active_component_definition
                .pascal_identifier
                .to_string(),
            type_id_escaped: escape_identifier(
                rngc.active_component_definition.type_id.to_string(),
            ),
            handlers: events,
            repeat_source_expression_literal_vec: rse_vec,
            repeat_source_expression_literal_range: rse_range,
        }
    } else if tnd.type_id == parsing::TYPE_ID_IF {
        // If
        let id = tnd
            .control_flow_settings
            .as_ref()
            .unwrap()
            .condition_expression_vtable_id
            .unwrap();

        let conditional_expression_paxel = tnd
            .control_flow_settings
            .as_ref()
            .unwrap()
            .condition_expression_paxel
            .as_ref()
            .unwrap();

        let common_properties_literal = CommonProperties::get_default_properties_literal()
            .iter()
            .map(|(id, value)| {
                (
                    MappedString::new(id.clone()),
                    MappedString::new(value.clone()),
                )
            })
            .collect();

        let conditional_source_map_id = source_map.insert(conditional_expression_paxel.clone());
        let conditional_mapped_string = source_map.generate_mapped_string(
            format!("Some(Box::new(PropertyExpression::new({})))", id),
            conditional_source_map_id,
        );

        TemplateArgsCodegenCartridgeRenderNodeLiteral {
            is_primitive: true,
            snake_case_type_id: "UNREACHABLE".into(),
            primitive_instance_import_path: Some("ConditionalInstance".into()),
            properties_coproduct_variant: "None".to_string(),
            component_properties_struct: "None".to_string(),
            defined_properties: vec![],
            common_properties_literal,
            children_literal,
            slot_index_literal: MappedString::none(),
            repeat_source_expression_literal_vec: MappedString::none(),
            repeat_source_expression_literal_range: MappedString::none(),
            conditional_boolean_expression_literal: conditional_mapped_string,
            pascal_identifier: rngc
                .active_component_definition
                .pascal_identifier
                .to_string(),
            type_id_escaped: escape_identifier(
                rngc.active_component_definition.type_id.to_string(),
            ),
            handlers: events,
        }
    } else if tnd.type_id == parsing::TYPE_ID_SLOT {
        // Slot
        let id = tnd
            .control_flow_settings
            .as_ref()
            .unwrap()
            .slot_index_expression_vtable_id
            .unwrap();

        let slot_expression = tnd
            .control_flow_settings
            .as_ref()
            .unwrap()
            .slot_index_expression_paxel
            .as_ref()
            .unwrap();

        let slot_source_map_id = source_map.insert(slot_expression.clone());
        let slot_mapped_string = source_map.generate_mapped_string(
            format!("Some(Box::new(PropertyExpression::new({})))", id),
            slot_source_map_id,
        );

        let common_properties_literal = CommonProperties::get_default_properties_literal()
            .iter()
            .map(|(id, value)| {
                (
                    MappedString::new(id.clone()),
                    MappedString::new(value.clone()),
                )
            })
            .collect();

        TemplateArgsCodegenCartridgeRenderNodeLiteral {
            is_primitive: true,
            snake_case_type_id: "UNREACHABLE".into(),
            primitive_instance_import_path: Some("SlotInstance".into()),
            properties_coproduct_variant: "None".to_string(),
            component_properties_struct: "None".to_string(),
            defined_properties: vec![],
            common_properties_literal,
            children_literal,
            slot_index_literal: slot_mapped_string,
            repeat_source_expression_literal_vec: MappedString::none(),
            repeat_source_expression_literal_range: MappedString::none(),
            conditional_boolean_expression_literal: MappedString::none(),
            pascal_identifier: rngc
                .active_component_definition
                .pascal_identifier
                .to_string(),
            type_id_escaped: escape_identifier(
                rngc.active_component_definition.type_id.to_string(),
            ),
            handlers: events,
        }
    } else {
        //Handle anything that's not a built-in

        let component_for_current_node = rngc.components.get(&tnd.type_id).unwrap();

        //Properties:
        //  - for each property on cfcn, there will either be:
        //     - an explicit, provided value, or
        //     - an implicit, default value
        //  - an explicit value is present IFF an AttributeValueDefinition
        //    for that property is present on the TemplateNodeDefinition.
        //    That AttributeValueDefinition may be an Expression or Literal (we can throw at this
        //    stage for any `Properties` that are bound to something other than an expression / literal)

        // Tuple of property_id, RIL literal string (e.g. `PropertyLiteral::new(...`_
        let property_ril_tuples: Vec<Option<(MappedString, MappedString)>> =
            component_for_current_node
                .get_property_definitions(rngc.type_table)
                .iter()
                .map(|pd| {
                    let ril_literal_string = {
                        if let Some(merged_settings) = &tnd.settings {
                            if let Some(matched_setting) = merged_settings
                                .iter()
                                .find(|avd| {
                                    match avd {
                                        SettingElement::Setting(key, _) => key.token_value == pd.name,
                                        _ => false
                                    }
                                })
                            {
                                if let SettingElement::Setting(key, value) = matched_setting {
                                    let setting_source_map_id =
                                        source_map.insert(key.clone());
                                    let key_mapped_string = source_map.generate_mapped_string(
                                        key.token_value.clone(),
                                        setting_source_map_id,
                                    );

                                    match &value{
                                        ValueDefinition::LiteralValue(lv) => {
                                            let value_source_map_id = source_map.insert(lv.clone());
                                            let value_mapped_string = source_map
                                                .generate_mapped_string(
                                                    format!("PropertyLiteral::new({})", lv.token_value),
                                                    value_source_map_id,
                                                );
                                            Some((key_mapped_string.clone(), value_mapped_string))
                                        }
                                        ValueDefinition::Expression(t, id)
                                        | ValueDefinition::Identifier(t, id) => {
                                            let value_source_map_id = source_map.insert(t.clone());
                                            let value_mapped_string = source_map
                                                .generate_mapped_string(
                                                    format!(
                                        "PropertyExpression::new({})",
                                        id.expect("Tried to use expression but it wasn't compiled")),
                                                    value_source_map_id,
                                                );
                                            Some((key_mapped_string.clone(), value_mapped_string))
                                        }
                                        ValueDefinition::Block(block) => Some((
                                            key_mapped_string.clone(),
                                            MappedString::new(format!(
                                                "PropertyLiteral::new({})",
                                                recurse_literal_block(
                                                    block.clone(),
                                                    pd.get_type_definition(&rngc.type_table),
                                                    host_crate_info,
                                                    source_map
                                                )
                                            )),
                                        )),
                                        _ => {
                                            panic!("Incorrect value bound to inline setting")
                                        }
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            //no inline attributes at all; everything will be default
                            None
                        }
                    };

                    if let Some((key, value)) = ril_literal_string {
                        Some((key, value))
                    } else {
                        None
                    }
                })
                .collect();

        let defined_properties: Vec<(MappedString, MappedString)> = property_ril_tuples
            .iter()
            .filter_map(|p| p.as_ref())
            .cloned()
            .collect();

        let identifiers_and_types = pax_runtime_api::CommonProperties::get_property_identifiers();

        fn default_common_property_value(identifier: &str) -> String {
            if identifier == "transform" {
                "Transform2D::default_wrapped()".to_string()
            } else if identifier == "width" || identifier == "height" {
                "Rc::new(RefCell::new(PropertyLiteral::new(Size::default())))".to_string()
            } else {
                "Default::default()".to_string()
            }
        }

        fn is_optional(identifier: &str) -> bool {
            identifier != "transform" && identifier != "width" && identifier != "height"
        }

        let common_properties_literal: Vec<(MappedString, MappedString)> = identifiers_and_types
            .iter()
            .map(|identifier_and_type| {
                if let Some(inline_settings) = &tnd.settings {
                    if let Some(matched_setting) = inline_settings
                        .iter()
                        .find(|e|
                             {
                                if let SettingElement::Setting(key, _) = e {
                                    key.token_value == *identifier_and_type.0
                                } else {
                                    false
                                }
                            }
                            )
                    {
                        if let SettingElement::Setting(key, value) = matched_setting {
                        let key_source_map_id = source_map.insert(key.clone());
                        let key_mapped_string = source_map.generate_mapped_string(key.token_value.clone(), key_source_map_id);

                        (
                            key_mapped_string,
                            match &value {
                                ValueDefinition::LiteralValue(lv) => {
                                    let value_source_map_id = source_map.insert(lv.clone());
                                    let mut literal_value = format!(
                                        "Rc::new(RefCell::new(PropertyLiteral::new(Into::<{}>::into({}))))",
                                        identifier_and_type.1,
                                        lv.token_value,
                                    );
                                    if is_optional(&identifier_and_type.0) {
                                        literal_value = format!("Some({})", literal_value);
                                    }
                                    let value_mapped_string = source_map.generate_mapped_string(literal_value,
                                         value_source_map_id);

                                    value_mapped_string
                                }
                                ValueDefinition::Expression(token, id)
                                | ValueDefinition::Identifier(token, id) => {
                                    let value_source_map_id = source_map.insert(token.clone());
                                    let mut literal_value = format!(
                                        "Rc::new(RefCell::new(PropertyExpression::new({})))",
                                        id.expect("Tried to use expression but it wasn't compiled")
                                    );
                                    if is_optional(&identifier_and_type.0) {
                                        literal_value = format!("Some({})", literal_value);
                                    }
                                    let value_mapped_string = source_map.generate_mapped_string(literal_value,
                                        value_source_map_id);
                                    value_mapped_string
                                }
                                _ => {
                                    panic!("Incorrect value bound to attribute")
                                }
                            },
                        )
                    } else {
                        (
                            MappedString::new(identifier_and_type.0.to_string()),
                            MappedString::new(default_common_property_value(&identifier_and_type.0)),
                        )
                    }
                } else {
                        (
                            MappedString::new(identifier_and_type.0.to_string()),
                            MappedString::new(default_common_property_value(&identifier_and_type.0)),
                        )
                    }
                } else {
                    (
                        MappedString::new(identifier_and_type.0.to_string()),
                        MappedString::new(default_common_property_value(&identifier_and_type.0)),
                    )
                }
            })
            .collect();
        //then, on the post-order traversal, press template string and return
        TemplateArgsCodegenCartridgeRenderNodeLiteral {
            is_primitive: component_for_current_node.is_primitive,
            snake_case_type_id: component_for_current_node.get_snake_case_id(),
            primitive_instance_import_path: component_for_current_node
                .primitive_instance_import_path
                .clone(),
            properties_coproduct_variant: component_for_current_node.type_id_escaped.to_string(),
            component_properties_struct: component_for_current_node.pascal_identifier.to_string(),
            defined_properties,
            common_properties_literal,
            children_literal,
            slot_index_literal: MappedString::none(),
            repeat_source_expression_literal_vec: MappedString::none(),
            repeat_source_expression_literal_range: MappedString::none(),
            conditional_boolean_expression_literal: MappedString::none(),
            pascal_identifier: rngc
                .active_component_definition
                .pascal_identifier
                .to_string(),
            type_id_escaped: escape_identifier(
                rngc.active_component_definition.type_id.to_string(),
            ),
            handlers: events,
        }
    };

    press_template_codegen_cartridge_render_node_literal(args)
}

struct RenderNodesGenerationContext<'a> {
    components: &'a std::collections::HashMap<String, ComponentDefinition>,
    active_component_definition: &'a ComponentDefinition,
    type_table: &'a TypeTable,
}

fn generate_handlers_map(
    handlers: Option<Vec<HandlersBlockElement>>,
    source_map: &mut SourceMap,
) -> Vec<(MappedString, Vec<MappedString>)> {
    let mut ret = HashMap::new();
    let _ = match handlers {
        Some(handler_elements) => {
            let handler_pairs: Vec<(Token, Vec<Token>)> = handler_elements
                .iter()
                .filter(|he| match he {
                    HandlersBlockElement::Handler(_, _) => true,
                    _ => false,
                })
                .map(|he| match he {
                    HandlersBlockElement::Handler(key, value) => (key.clone(), value.clone()),
                    _ => unreachable!("Non-handler elements should have been filtered out"),
                })
                .collect();
            for e in handler_pairs.iter() {
                let handler_values: Vec<MappedString> =
                    e.1.clone()
                        .iter()
                        .map(|et| {
                            let et_source_map_id = source_map.insert(et.clone());
                            let et_mapped_string = source_map
                                .generate_mapped_string(et.token_value.clone(), et_source_map_id);
                            et_mapped_string
                        })
                        .collect();
                let key_source_map_id = source_map.insert(e.0.clone());
                let key_mapped_string =
                    source_map.generate_mapped_string(e.0.token_value.clone(), key_source_map_id);
                ret.insert(key_mapped_string, handler_values);
            }
        }
        _ => {}
    };
    ret.into_iter().collect()
}

fn generate_cartridge_component_factory_literal(
    manifest: &PaxManifest,
    cd: &ComponentDefinition,
    host_crate_info: &HostCrateInfo,
    source_map: &mut SourceMap,
) -> String {
    let rngc = RenderNodesGenerationContext {
        components: &manifest.components,
        active_component_definition: cd,
        type_table: &manifest.type_table,
    };

    let args = TemplateArgsCodegenCartridgeComponentFactory {
        is_main_component: cd.is_main_component,
        snake_case_type_id: cd.get_snake_case_id(),
        component_properties_struct: cd.pascal_identifier.to_string(),
        properties: cd
            .get_property_definitions(&manifest.type_table)
            .iter()
            .map(|pd| {
                (
                    pd.clone(),
                    pd.get_type_definition(&manifest.type_table)
                        .type_id_escaped
                        .clone(),
                )
            })
            .collect(),
        handlers: generate_handlers_map(cd.handlers.clone(), source_map),
        render_nodes_literal: generate_cartridge_render_nodes_literal(
            &rngc,
            host_crate_info,
            source_map,
        ),
        properties_coproduct_variant: cd.type_id_escaped.to_string(),
    };

    press_template_codegen_cartridge_component_factory(args)
}

pub fn generate_and_overwrite_properties_coproduct(
    pax_dir: &PathBuf,
    manifest: &PaxManifest,
    host_crate_info: &HostCrateInfo,
) {
    let target_dir = pax_dir.join(PKG_DIR_NAME).join("pax-properties-coproduct");

    let target_cargo_full_path = fs::canonicalize(target_dir.join("Cargo.toml")).unwrap();
    let mut target_cargo_toml_contents =
        toml_edit::Document::from_str(&fs::read_to_string(&target_cargo_full_path).unwrap())
            .unwrap();

    //insert new entry pointing to userland crate, where `pax_app` is defined
    std::mem::swap(
        target_cargo_toml_contents["dependencies"]
            .get_mut(&host_crate_info.name)
            .unwrap(),
        &mut Item::from_str("{ path=\"../../..\" }").unwrap(),
    );

    //write patched Cargo.toml
    fs::write(
        &target_cargo_full_path,
        &target_cargo_toml_contents.to_string(),
    )
    .unwrap();

    //build tuples for PropertiesCoproduct
    let mut properties_coproduct_tuples: Vec<(String, String)> = manifest
        .components
        .iter()
        .map(|comp_def| {
            let mod_path = if &comp_def.1.module_path == "crate" {
                "".to_string()
            } else {
                comp_def.1.module_path.replace("crate::", "") + "::"
            };
            (
                comp_def.1.type_id_escaped.clone(),
                format!(
                    "{}{}{}",
                    &host_crate_info.import_prefix, &mod_path, &comp_def.1.pascal_identifier
                ),
            )
        })
        .collect();
    let set: HashSet<(String, String)> = properties_coproduct_tuples.drain(..).collect();
    properties_coproduct_tuples.extend(set.into_iter());
    properties_coproduct_tuples.sort();

    //build tuples for TypesCoproduct
    // - include all Property types, representing all possible return types for Expressions
    // - include all T such that T is the iterator type for some Property<Vec<T>>
    let mut types_coproduct_tuples: Vec<(String, String)> = manifest
        .components
        .iter()
        .map(|cd| {
            cd.1.get_property_definitions(&manifest.type_table)
                .iter()
                .map(|pm| {
                    let td = pm.get_type_definition(&manifest.type_table);

                    (
                        td.type_id_escaped.clone(),
                        host_crate_info.import_prefix.to_string()
                            + &td.type_id.clone().replace("crate::", ""),
                    )
                })
                .collect::<Vec<_>>()
        })
        .flatten()
        .collect::<Vec<_>>();

    let mut set: HashSet<_> = types_coproduct_tuples.drain(..).collect();

    #[allow(non_snake_case)]
    let TYPES_COPRODUCT_BUILT_INS = vec![
        ("f64", "f64"),
        ("bool", "bool"),
        ("isize", "isize"),
        ("usize", "usize"),
        ("String", "String"),
        (
            "stdCOCOvecCOCOVecLABRstdCOCOrcCOCORcLABRPropertiesCoproductRABRRABR",
            "std::vec::Vec<std::rc::Rc<PropertiesCoproduct>>",
        ),
        ("Transform2D", "pax_runtime_api::Transform2D"),
        ("stdCOCOopsCOCORangeLABRisizeRABR", "std::ops::Range<isize>"),
        ("Size", "pax_runtime_api::Size"),
        ("Rotation", "pax_runtime_api::Rotation"),
        ("SizePixels", "pax_runtime_api::SizePixels"),
        ("Numeric", "pax_runtime_api::Numeric"),
        ("StringBox", "pax_runtime_api::StringBox"),
    ];

    TYPES_COPRODUCT_BUILT_INS.iter().for_each(|builtin| {
        set.insert((builtin.0.to_string(), builtin.1.to_string()));
    });
    types_coproduct_tuples.extend(set.into_iter());
    types_coproduct_tuples.sort();

    types_coproduct_tuples = types_coproduct_tuples
        .into_iter()
        .unique_by(|elem| elem.0.to_string())
        .collect::<Vec<(String, String)>>();

    //press template into String
    let generated_lib_rs = templating::press_template_codegen_properties_coproduct_lib(
        templating::TemplateArgsCodegenPropertiesCoproductLib {
            properties_coproduct_tuples,
            types_coproduct_tuples,
        },
    );

    //write String to file
    fs::write(target_dir.join("src/lib.rs"), generated_lib_rs).unwrap();
}
