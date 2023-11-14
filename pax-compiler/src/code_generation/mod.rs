//! # Code Generation Module
//!
//! The `code_generation` module provides structures and functions for generating Pax Cartridges
//! from Pax Manifests. The `generate_and_overwrite_cartridge` function is the main entrypoint.

use crate::helpers::{HostCrateInfo, IMPORTS_BUILTINS, PKG_DIR_NAME};
use crate::manifest::{PaxManifest, Token};
use crate::parsing;
use itertools::Itertools;
use pax_runtime_api::CommonProperties;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fs;
use std::str::FromStr;

use crate::manifest::{
    ComponentDefinition, EventDefinition, ExpressionSpec, LiteralBlockDefinition,
    TemplateNodeDefinition, TypeDefinition, TypeTable, ValueDefinition,
};

use crate::errors::source_map::SourceMap;
use std::path::PathBuf;
use toml_edit::Item;

use crate::parsing::escape_identifier;

use self::templating::{
    press_template_codegen_cartridge_component_factory,
    press_template_codegen_cartridge_render_node_literal, MappedString,
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

    let mut imports: Vec<String> = manifest
        .import_paths
        .iter()
        .map(|e| host_crate_info.fully_qualify_path(e))
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

    let implicit_root = nodes[0].borrow();
    let children_literal: Vec<String> = implicit_root
        .child_ids
        .iter()
        .map(|child_id| {
            let tnd_map = rngc.active_component_definition.template.as_ref().unwrap();
            let active_tnd = &tnd_map[*child_id];
            recurse_generate_render_nodes_literal(rngc, active_tnd, host_crate_info, source_map)
        })
        .collect();

    children_literal.join(",")
}

fn generate_bound_events(
    inline_settings: Option<Vec<(Token, ValueDefinition)>>,
    source_map: &mut SourceMap,
) -> Vec<(MappedString, MappedString)> {
    let mut ret: HashMap<MappedString, MappedString> = HashMap::new();
    if let Some(ref inline) = inline_settings {
        for (key, value) in inline.iter() {
            if let ValueDefinition::EventBindingTarget(s) = value {
                let key_source_map_id = source_map.insert(key.clone());
                let key_mapped_string =
                    source_map.generate_mapped_string(key.token_value.clone(), key_source_map_id);

                let value_source_map_id = source_map.insert(s.clone());
                let value_mapped_string =
                    source_map.generate_mapped_string(s.token_value.clone(), value_source_map_id);

                ret.insert(key_mapped_string, value_mapped_string);
            };
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
    let qualified_path = host_crate_info.fully_qualify_path(&type_definition.import_path);

    // Buffer to store the string representation of the struct
    let mut struct_representation = format!("\n{{ let mut ret = {}::default();", qualified_path);

    // Iterating through each (key, value) pair in the settings_key_value_pairs
    for (key, value_definition) in block.settings_key_value_pairs.iter() {
        let type_id = &type_definition
            .property_definitions
            .iter()
            .find(|pd| &pd.name == &key.token_value)
            .expect(&format!(
                "Property {} not found on type {}",
                key.token_value, type_definition.type_id
            ))
            .type_id;
        let fully_qualified_type = host_crate_info.fully_qualify_path(type_id);

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
                    id.expect("Tried to use expression but it wasn't compiled")
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
        .map(|child_id| {
            let active_tnd =
                &rngc.active_component_definition.template.as_ref().unwrap()[*child_id];
            recurse_generate_render_nodes_literal(rngc, active_tnd, host_crate_info, source_map)
        })
        .collect();

    //pull inline event binding and store into map
    let events = generate_bound_events(tnd.settings.clone(), source_map);
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
            properties_coproduct_variant: "Repeat".to_string(),
            component_properties_struct: "RepeatProperties".to_string(),
            defined_properties: vec![
                (
                    MappedString::new("source_expression_vec".to_string()),
                    rse_vec,
                ),
                (
                    MappedString::new("source_expression_range".to_string()),
                    rse_range,
                ),
            ],
            common_properties_literal,
            children_literal,

            pascal_identifier: rngc
                .active_component_definition
                .pascal_identifier
                .to_string(),
            type_id_escaped: escape_identifier(
                rngc.active_component_definition.type_id.to_string(),
            ),
            events,
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
            properties_coproduct_variant: "Conditional".to_string(),
            component_properties_struct: "ConditionalProperties".to_string(),
            defined_properties: vec![(
                MappedString::new("boolean_expression".to_string()),
                conditional_mapped_string,
            )],
            common_properties_literal,
            children_literal,
            pascal_identifier: rngc
                .active_component_definition
                .pascal_identifier
                .to_string(),
            type_id_escaped: escape_identifier(
                rngc.active_component_definition.type_id.to_string(),
            ),
            events,
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
            properties_coproduct_variant: "Slot".to_string(),
            component_properties_struct: "SlotProperties".to_string(),
            defined_properties: vec![(MappedString::new("index".to_string()), slot_mapped_string)],
            common_properties_literal,
            children_literal,
            pascal_identifier: rngc
                .active_component_definition
                .pascal_identifier
                .to_string(),
            type_id_escaped: escape_identifier(
                rngc.active_component_definition.type_id.to_string(),
            ),
            events,
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
                                .find(|avd| avd.0.token_value == pd.name)
                            {
                                let setting_source_map_id =
                                    source_map.insert(matched_setting.0.clone());
                                let key_mapped_string = source_map.generate_mapped_string(
                                    matched_setting.0.token_value.clone(),
                                    setting_source_map_id,
                                );

                                match &matched_setting.1 {
                                    ValueDefinition::LiteralValue(lv) => {
                                        let value_source_map_id = source_map.insert(lv.clone());
                                        let value_mapped_string = source_map
                                            .generate_mapped_string(
                                                format!(
                                                    "Box::new(PropertyLiteral::new({}))",
                                                    lv.token_value
                                                ),
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
                                    "Box::new(PropertyExpression::new({}))",
                                    id.expect("Tried to use expression but it wasn't compiled")),
                                                value_source_map_id,
                                            );
                                        Some((key_mapped_string.clone(), value_mapped_string))
                                    }
                                    ValueDefinition::Block(block) => Some((
                                        key_mapped_string.clone(),
                                        MappedString::new(format!(
                                            "Box::new(PropertyLiteral::new({}))",
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
                        .find(|vd| vd.0.token_value == *identifier_and_type.0)
                    {
                        let key_source_map_id = source_map.insert(matched_setting.0.clone());
                        let key_mapped_string = source_map.generate_mapped_string(
                            matched_setting.0.token_value.clone(),
                            key_source_map_id,
                        );

                        (
                            key_mapped_string,
                            match &matched_setting.1 {
                                ValueDefinition::LiteralValue(lv) => {
                                    let value_source_map_id = source_map.insert(lv.clone());
                                    let mut literal_value = format!(
                                        "Box::new(PropertyLiteral::new(Into::<{}>::into({})))",
                                        identifier_and_type.1, lv.token_value,
                                    );
                                    if is_optional(&identifier_and_type.0) {
                                        literal_value = format!("Some({})", literal_value);
                                    }
                                    let value_mapped_string = source_map
                                        .generate_mapped_string(literal_value, value_source_map_id);

                                    value_mapped_string
                                }
                                ValueDefinition::Expression(token, id)
                                | ValueDefinition::Identifier(token, id) => {
                                    let value_source_map_id = source_map.insert(token.clone());
                                    let mut literal_value = format!(
                                        "Box::new(PropertyExpression::new({}))",
                                        id.expect("Tried to use expression but it wasn't compiled")
                                    );
                                    if is_optional(&identifier_and_type.0) {
                                        literal_value = format!("Some({})", literal_value);
                                    }
                                    let value_mapped_string = source_map
                                        .generate_mapped_string(literal_value, value_source_map_id);
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
                            MappedString::new(default_common_property_value(
                                &identifier_and_type.0,
                            )),
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
            pascal_identifier: rngc
                .active_component_definition
                .pascal_identifier
                .to_string(),
            type_id_escaped: escape_identifier(
                rngc.active_component_definition.type_id.to_string(),
            ),
            events,
        }
    };

    press_template_codegen_cartridge_render_node_literal(args)
}

struct RenderNodesGenerationContext<'a> {
    components: &'a std::collections::HashMap<String, ComponentDefinition>,
    active_component_definition: &'a ComponentDefinition,
    type_table: &'a TypeTable,
}

fn generate_events_map(
    events: Option<Vec<EventDefinition>>,
    source_map: &mut SourceMap,
) -> Vec<(MappedString, Vec<MappedString>)> {
    let mut ret = HashMap::new();
    let _ = match events {
        Some(event_list) => {
            for e in event_list.iter() {
                let event_values: Vec<MappedString> = e
                    .value
                    .clone()
                    .iter()
                    .map(|et| {
                        let et_source_map_id = source_map.insert(et.clone());
                        let et_mapped_string = source_map
                            .generate_mapped_string(et.token_value.clone(), et_source_map_id);
                        et_mapped_string
                    })
                    .collect();
                let key_source_map_id = source_map.insert(e.key.clone());
                let key_mapped_string =
                    source_map.generate_mapped_string(e.key.token_value.clone(), key_source_map_id);
                ret.insert(key_mapped_string, event_values);
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

    let fully_qualified_properties_type = host_crate_info.fully_qualify_path(&cd.type_id);

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
                    host_crate_info.fully_qualify_path(&pd.get_type_definition(&manifest.type_table).type_id),
                )
            })
            .collect(),
        events: generate_events_map(cd.events.clone(), source_map),
        render_nodes_literal: generate_cartridge_render_nodes_literal(
            &rngc,
            host_crate_info,
            source_map,
        ),
        fully_qualified_properties_type,
    };

    press_template_codegen_cartridge_component_factory(args)
}
