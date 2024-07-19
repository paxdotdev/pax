use pax_manifest::{
    escape_identifier, ComponentDefinition, ComponentTemplate,
    ControlFlowRepeatPredicateDefinition, ExpressionCompilationInfo, ExpressionSpec,
    ExpressionSpecInvocation, HostCrateInfo, PaxManifest, PropertyDefinition,
    PropertyDefinitionFlags, SettingElement, TemplateNodeId, Token, TypeDefinition, TypeId,
    TypeTable, ValueDefinition,
};
use std::collections::{BTreeMap, HashMap};
use std::ops::RangeFrom;
use std::slice::IterMut;

use crate::errors::source_map::SourceMap;
use crate::errors::PaxTemplateError;
use color_eyre::eyre;
use color_eyre::eyre::Report;
use lazy_static::lazy_static;
use pax_manifest::constants::COMMON_PROPERTIES_TYPE;

pub fn compile_all_expressions<'a>(
    manifest: &'a mut PaxManifest,
    source_map: &'a mut SourceMap,
    host_crate_info: &'a HostCrateInfo,
) -> eyre::Result<(), Report> {
    let mut swap_expression_specs: HashMap<usize, ExpressionSpec> = HashMap::new();
    let mut all_expression_specs: HashMap<usize, ExpressionSpec> = HashMap::new();

    let mut new_components = manifest.components.clone();
    let mut vtable_uid_track = 0;

    for component_def in new_components.values_mut() {
        let mut new_component_def = component_def.clone();
        let read_only_component_def = component_def.clone();

        if let Some(ref mut template) = new_component_def.template {
            let root = template.get_root().clone();

            let mut ctx = ExpressionCompilationContext {
                template,
                active_node_id: None,
                scope_stack: vec![component_def
                    .get_property_definitions(&manifest.type_table)
                    .iter()
                    .map(|pd| (pd.name.clone(), pd.clone()))
                    .collect()],
                vtable_uid_gen: vtable_uid_track..,
                all_components: manifest.components.clone(),
                expression_specs: &mut swap_expression_specs,
                component_def: &read_only_component_def,
                type_table: &manifest.type_table,
                host_crate_info,
            };

            for id in root {
                ctx.active_node_id = Some(id.clone());
                ctx = recurse_compile_expressions(ctx, source_map)?;
            }

            vtable_uid_track = ctx.vtable_uid_gen.next().unwrap();
            all_expression_specs.extend(ctx.expression_specs.to_owned());
        }

        std::mem::swap(component_def, &mut new_component_def);
    }

    manifest.components = new_components;
    manifest.expression_specs = Some(swap_expression_specs);
    Ok(())
}

fn get_output_type_by_property_identifier(
    _ctx: &ExpressionCompilationContext,
    prop_defs: &Vec<PropertyDefinition>,
    property_identifier: &str,
    token: &Token,
) -> Result<String, eyre::Report> {
    let output_type = if let Some(common_match) = COMMON_PROPERTIES_TYPE
        .iter()
        .find(|cpt| cpt.0 == property_identifier)
    {
        Ok((*common_match).1.to_string())
    } else if let Some(local_match) = prop_defs
        .iter()
        .find(|property_def| property_def.name == property_identifier)
    {
        Ok(local_match.type_id.to_string())
    } else {
        return Err(PaxTemplateError::new(
            Some(format!("failed to resolve symbol {}", property_identifier)),
            token.clone(),
        ));
    };

    output_type
}

fn recurse_compile_literal_block<'a>(
    settings_pairs: &mut IterMut<SettingElement>,
    ctx: &mut ExpressionCompilationContext,
    current_property_definitions: Vec<PropertyDefinition>,
    type_id: TypeId,
    source_map: &mut SourceMap,
) -> Result<(), eyre::Report> {
    settings_pairs.try_for_each(|e| {
        if let SettingElement::Setting(token, value) = e {
            match value {
                // LiteralValue:       no need to compile literal values
                // EventBindingTarget: event bindings are handled on a separate compiler pass; no-op here
                ValueDefinition::LiteralValue(_)
                | ValueDefinition::EventBindingTarget(_)
                | ValueDefinition::DoubleBinding(_, _) => {}
                ValueDefinition::Block(block) => {
                    let type_def = (current_property_definitions
                        .iter()
                        .find(|property_def| property_def.name == token.token_value))
                    .ok_or::<eyre::Report>(PaxTemplateError::new(
                        Some(format!(
                            "Property `{}` not found on `{}`",
                            &token.token_value,
                            type_id.get_unique_identifier()
                        )),
                        token.clone(),
                    ))?
                    .get_type_definition(ctx.type_table);
                    recurse_compile_literal_block(
                        &mut block.elements.iter_mut(),
                        ctx,
                        type_def.property_definitions.clone(),
                        type_def.type_id.clone(),
                        source_map,
                    )?;
                }
                ValueDefinition::Expression(input, expression_compilation_info) => {
                    // e.g. the `self.num_clicks + 5` in `<SomeNode some_property={self.num_clicks + 5} />`

                    let output_type = get_output_type_by_property_identifier(
                        ctx,
                        &current_property_definitions,
                        &token.token_value,
                        input,
                    )?;

                    let id = ctx.vtable_uid_gen.next().unwrap();
                    let (output_statement, invocations) =
                        compile_paxel_to_ril(input.clone(), &ctx)?;

                    let mut whitespace_removed_input = input.clone().token_value;
                    whitespace_removed_input.retain(|c| !c.is_whitespace());

                    let source_map_id = source_map.insert(input.clone());
                    let input_statement =
                        source_map.generate_mapped_string(whitespace_removed_input, source_map_id);
                    ctx.expression_specs.insert(
                        id,
                        ExpressionSpec {
                            id,
                            invocations: invocations.clone(),
                            output_type,
                            output_statement,
                            input_statement,
                            is_repeat_source_iterable_expression: false,
                        },
                    );

                    //Write this expression compilation info back to the manifest, for downstream use by RIL component tree generator
                    let dependencies = invocations
                        .iter()
                        .map(|i| i.root_identifier.clone())
                        .collect::<Vec<String>>();
                    let mut expression_compilation_insert = Some(ExpressionCompilationInfo {
                        vtable_id: id,
                        dependencies,
                    });
                    std::mem::swap(
                        expression_compilation_info,
                        &mut expression_compilation_insert,
                    );
                }
                ValueDefinition::Identifier(identifier, expression_compilation_info) => {
                    // e.g. the self.active_color in `bg_color=self.active_color`

                    if token.token_value == "id" || token.token_value == "class" {
                        //No-op -- special-case `id=some_identifier` and `class=some_identifier` — we DON'T want to compile an expression {some_identifier},
                        //so we skip the case where `id` is the key
                    } else {
                        let id = ctx.vtable_uid_gen.next().unwrap();

                        let type_def = (current_property_definitions
                            .iter()
                            .find(|property_def| property_def.name == token.token_value))
                        .ok_or::<eyre::Report>(PaxTemplateError::new(
                            Some(format!(
                                "Property `{}` not found on `{}`",
                                &token.token_value, type_id
                            )),
                            token.clone(),
                        ))?
                        .get_type_definition(ctx.type_table);
                        let output_type = type_def.type_id.clone();

                        //a single identifier binding is the same as an expression returning that identifier, `{self.some_identifier}`
                        //thus, we can compile it as PAXEL and make use of any shared logic, e.g. `self`/`this` handling
                        let (output_statement, invocations) =
                            compile_paxel_to_ril(identifier.clone(), &ctx)?;

                        //Write this expression compilation info back to the manifest, for downstream use by RIL component tree generator
                        let dependencies = invocations
                            .iter()
                            .map(|i| i.root_identifier.clone())
                            .collect::<Vec<String>>();
                        let mut expression_compilation_insert = Some(ExpressionCompilationInfo {
                            vtable_id: id,
                            dependencies,
                        });
                        std::mem::swap(
                            expression_compilation_info,
                            &mut expression_compilation_insert,
                        );

                        let source_map_id = source_map.insert(identifier.clone());
                        let input_statement = source_map
                            .generate_mapped_string(identifier.token_value.clone(), source_map_id);

                        let output_type = output_type.to_string();
                        ctx.expression_specs.insert(
                            id,
                            ExpressionSpec {
                                id,
                                invocations,
                                output_type,
                                output_statement,
                                input_statement,
                                is_repeat_source_iterable_expression: false,
                            },
                        );
                    }
                }
                _ => {
                    unreachable!()
                }
            }
        };
        Ok::<(), eyre::Report>(())
    })
}

fn recurse_compile_expressions<'a>(
    mut ctx: ExpressionCompilationContext<'a>,
    mut source_map: &mut SourceMap,
) -> eyre::Result<ExpressionCompilationContext<'a>, Report> {
    let mut incremented = false;

    let cloned_settings_block = ctx.component_def.settings.clone();
    let mut active_node_def = ctx
        .template
        .get_node(&ctx.active_node_id.clone().unwrap())
        .unwrap()
        .clone();
    let cloned_inline_settings = active_node_def.settings.clone();
    let mut merged_settings = PaxManifest::merge_inline_settings_with_settings_block(
        &cloned_inline_settings,
        &cloned_settings_block,
    );
    let mut cloned_control_flow_settings = active_node_def.control_flow_settings.clone();

    if let Some(ref mut inline_settings) = merged_settings {
        // Handle standard key/value declarations (non-control-flow)
        let type_id = active_node_def.type_id.clone();
        let pascal_identifier;
        let property_def;

        // Scope created to limit the borrow of ctx
        {
            let active_node_component = ctx.all_components.get(&type_id)
                .expect(&format!("No known component with identifier {}.  Try importing or defining a component named {}", &type_id, &type_id));

            pascal_identifier = type_id.clone();
            property_def = active_node_component.get_property_definitions(&mut ctx.type_table);
        }

        recurse_compile_literal_block(
            &mut inline_settings.iter_mut(),
            &mut ctx,
            property_def.clone(),
            pascal_identifier,
            &mut source_map,
        )?;
    } else if let Some(ref mut cfa) = cloned_control_flow_settings {
        //Handle attributes for control flow
        //Our purpose here is broadly twofold:
        //  1. attach repeat-created symbols / properties to the stack, so they may be resolved in PAXEL
        //  2. compile & create an expression vtable entry for `source`

        // Definitions are stored modally as `Option<T>`s in ControlFlowAttributeValueDefinition,
        // so: iff `repeat_source_definition` is present, then we can assume this is a Repeat element
        if let Some(ref mut repeat_source_definition) = &mut cfa.repeat_source_definition {
            // Examples:
            // for (elem, i) in self.elements
            //  - must be a symbolic identifier, such as `elements` or `self.elements`
            // for i in 0..max_elems
            //  - may use an integer literal or symbolic identifier in either position
            //  - must use an exclusive (..) range operator (inclusive could be supported; effort required)

            let id = ctx.vtable_uid_gen.next().unwrap();
            let mut deps = vec![];
            if let Some(iterable) = &repeat_source_definition.symbolic_binding {
                deps.push(iterable.token_value.trim().to_string());
            } else if repeat_source_definition.range_symbolic_bindings.len() != 0 {
                deps.extend(
                    repeat_source_definition
                        .range_symbolic_bindings
                        .iter()
                        .map(|s| s.token_value.trim().to_string()),
                );
            }
            repeat_source_definition.expression_info = Some(ExpressionCompilationInfo {
                vtable_id: id,
                dependencies: deps,
            });

            // Handle the `self.some_data_source` in `for (elem, i) in self.some_data_source`
            let repeat_source_definition = cfa.repeat_source_definition.as_ref().unwrap();

            let is_repeat_source_range = repeat_source_definition.range_expression_paxel.is_some();
            let is_repeat_source_iterable = repeat_source_definition.symbolic_binding.is_some();

            #[allow(unused)]
            let (paxel, return_type) = if let Some(range_expression_paxel) =
                &repeat_source_definition.range_expression_paxel
            {
                (
                    range_expression_paxel.clone(),
                    TypeDefinition::builtin_range_isize(),
                )
            } else if let Some(symbolic_binding) = &repeat_source_definition.symbolic_binding {
                let inner_iterable_type_id = ctx
                    .resolve_symbol_as_prop_def(
                        &symbolic_binding.token_value,
                        symbolic_binding.clone(),
                    )?
                    .unwrap()
                    .last()
                    .unwrap()
                    .get_inner_iterable_type_definition(ctx.type_table)
                    .unwrap()
                    .type_id
                    .clone();
                (
                    symbolic_binding.clone(),
                    TypeDefinition::builtin_vec_rc_ref_cell_any_properties(inner_iterable_type_id),
                )
            } else {
                unreachable!()
            };

            //Though we are compiling this as an arbitrary expression, we must already have validated
            //with the parser that we are only binding to a simple symbolic id, like `self.foo`.
            //This is because we are inferring the return type of this expression based on the declared-and-known
            //type of property `self.foo`
            let (output_statement, invocations) = compile_paxel_to_ril(paxel.clone(), &ctx)?;

            //Figure out the return type for our datum — either `T` for `Property<Vec<T>>`, or `isize` for some range `j..k`
            //if repeat_source is a range, this is simply isize
            //if repeat_source is a symbolic binding, then we resolve that symbolic binding and use that resolved type here
            let iterable_type = if let Some(_) = &repeat_source_definition.range_expression_paxel {
                TypeDefinition::primitive("isize")
            } else if let Some(symbolic_binding) = &repeat_source_definition.symbolic_binding {
                let pd = ctx
                    .resolve_symbol_as_prop_def(
                        &symbolic_binding.token_value,
                        symbolic_binding.clone(),
                    )?
                    .ok_or::<eyre::Report>(PaxTemplateError::new(
                        Some(format!(
                            "Property not found: {}",
                            symbolic_binding.token_value
                        )),
                        symbolic_binding.clone(),
                    ))?
                    .last()
                    .unwrap()
                    .clone();
                pd.get_inner_iterable_type_definition(ctx.type_table)
                    .unwrap()
                    .clone()
            } else {
                unreachable!()
            };

            // Attach shadowed property symbols to the scope_stack, so e.g. `elem` can be
            // referred to with the symbol `elem` in PAXEL
            match cfa.repeat_predicate_definition.as_ref().unwrap() {
                ControlFlowRepeatPredicateDefinition::ElemId(elem_id) => {
                    //if repeat_source is a range, elem is bound to the element within the range
                    //if repeat_source is a symbolic binding,
                    //for i in 0..5
                    // i describes the element (not the index!), which in this case is a `isize`
                    // property definition: called `i`
                    // property_type:isize (the iterable_type)

                    let property_definition = PropertyDefinition {
                        name: format!("{}", elem_id.token_value),

                        flags: PropertyDefinitionFlags {
                            is_binding_repeat_i: false,
                            is_binding_repeat_elem: true,
                            is_repeat_source_range,
                            is_repeat_source_iterable,
                            is_property_wrapped: true,
                            is_enum: false,
                        },
                        type_id: iterable_type.type_id.clone(),
                    };

                    let scope: HashMap<String, PropertyDefinition> = HashMap::from([
                        //`elem` property (by specified name)
                        (elem_id.token_value.clone(), property_definition),
                    ]);

                    incremented = true;
                    ctx.scope_stack.push(scope);
                }
                ControlFlowRepeatPredicateDefinition::ElemIdIndexId(elem_id, index_id) => {
                    let elem_property_definition = PropertyDefinition {
                        name: format!("{}", elem_id.token_value),
                        type_id: iterable_type.type_id.clone(),
                        flags: PropertyDefinitionFlags {
                            is_binding_repeat_elem: true,
                            is_binding_repeat_i: false,
                            is_repeat_source_range: is_repeat_source_range.clone(),
                            is_repeat_source_iterable: is_repeat_source_iterable.clone(),
                            is_property_wrapped: true,
                            is_enum: false,
                        },
                    };

                    let mut i_property_definition =
                        PropertyDefinition::primitive_with_name("usize", &index_id.token_value);
                    i_property_definition.flags = PropertyDefinitionFlags {
                        is_binding_repeat_i: true,
                        is_binding_repeat_elem: false,
                        is_repeat_source_range,
                        is_repeat_source_iterable,
                        is_property_wrapped: true,
                        is_enum: false,
                    };

                    incremented = true;
                    ctx.scope_stack.push(HashMap::from([
                        //`elem` property (by specified name)
                        (elem_id.clone().token_value, elem_property_definition),
                        //`i` property (by specified name)
                        (index_id.clone().token_value, i_property_definition),
                    ]));
                }
            };

            // The return type for a repeat source expression will either be:
            //   1. isize, for ranges (including ranges with direct symbolic references as either operand, like `self.x..10`)
            //   2. T for a direct symbolic reference to `self.x` for x : Property<Vec<T>>
            // Presumably, we could also support arbitrary expressions as a #3, but
            // we need some way to infer the return type, statically.  This may mean requiring
            // an explicit type declaration by the end-user, or perhaps we can hack something
            // with further compiletime "reflection" magic

            let mut whitespace_removed_input = paxel.clone().token_value;
            whitespace_removed_input.retain(|c| !c.is_whitespace());

            let source_map_id = source_map.insert(paxel.clone());
            let input_statement =
                source_map.generate_mapped_string(whitespace_removed_input, source_map_id);

            let output_type = return_type.type_id.clone();

            let output_type = output_type.to_string();
            ctx.expression_specs.insert(
                id,
                ExpressionSpec {
                    id,
                    invocations,
                    output_type,
                    output_statement,
                    input_statement,
                    is_repeat_source_iterable_expression: is_repeat_source_iterable,
                },
            );
        } else if let Some(condition_expression_paxel) = &cfa.condition_expression_paxel {
            //Handle `if` boolean expression, e.g. the `num_clicks > 5` in `if num_clicks > 5 { ... }`
            let (output_statement, invocations) =
                compile_paxel_to_ril(condition_expression_paxel.clone(), &ctx)?;
            let id = ctx.vtable_uid_gen.next().unwrap();

            let deps = invocations
                .iter()
                .map(|i| i.root_identifier.clone())
                .collect::<Vec<String>>();

            cfa.condition_expression_info = Some(ExpressionCompilationInfo {
                vtable_id: id,
                dependencies: deps,
            });

            let mut whitespace_removed_input = condition_expression_paxel.clone().token_value;
            whitespace_removed_input.retain(|c| !c.is_whitespace());

            let source_map_id = source_map.insert(condition_expression_paxel.clone());
            let input_statement =
                source_map.generate_mapped_string(whitespace_removed_input, source_map_id);

            ctx.expression_specs.insert(
                id,
                ExpressionSpec {
                    id,
                    invocations,
                    output_type: "bool".to_string(),
                    output_statement,
                    input_statement,
                    is_repeat_source_iterable_expression: false,
                },
            );
        } else if let Some(slot_index_expression_paxel) = &cfa.slot_index_expression_paxel {
            //Handle `slot` index expression, e.g. the `i` in `slot(i)`
            let (output_statement, invocations) =
                compile_paxel_to_ril(slot_index_expression_paxel.clone(), &ctx)?;
            let id = ctx.vtable_uid_gen.next().unwrap();

            let deps = invocations
                .iter()
                .map(|i| i.root_identifier.clone())
                .collect::<Vec<String>>();

            cfa.slot_index_expression_info = Some(ExpressionCompilationInfo {
                vtable_id: id,
                dependencies: deps,
            });

            let mut whitespace_removed_input = slot_index_expression_paxel.clone().token_value;
            whitespace_removed_input.retain(|c| !c.is_whitespace());

            let source_map_id = source_map.insert(slot_index_expression_paxel.clone());
            let input_statement =
                source_map.generate_mapped_string(whitespace_removed_input, source_map_id);

            ctx.expression_specs.insert(
                id,
                ExpressionSpec {
                    id,
                    invocations,
                    output_type: "Numeric".to_string(),
                    output_statement,
                    input_statement,
                    is_repeat_source_iterable_expression: false,
                },
            );
        } else {
            unreachable!("encountered invalid control flow definition")
        }

        // Write back our modified control_flow_settings, which now contain vtable lookup ids
        std::mem::swap(
            &mut cloned_control_flow_settings,
            &mut active_node_def.control_flow_settings,
        );
    }

    std::mem::swap(&mut merged_settings, &mut active_node_def.settings);

    // Traverse descendent nodes and continue compiling expressions recursively
    for id in ctx
        .template
        .get_children(&ctx.active_node_id.clone().unwrap())
        .clone()
        .unwrap_or_default()
        .iter()
    {
        // update active id to child for next level of recursion into tree
        let parent_id = ctx.active_node_id;
        ctx.active_node_id = Some(id.clone());

        ctx = recurse_compile_expressions(ctx, source_map)?;

        ctx.active_node_id = parent_id;
    }

    ctx.template
        .set_node(ctx.active_node_id.clone().unwrap(), active_node_def);

    if incremented {
        ctx.scope_stack.pop();
    }
    Ok(ctx)
}

/// From a symbol like `num_clicks` or `self.num_clicks`, populate an ExpressionSpecInvocation
fn resolve_symbol_as_invocation(
    sym: &str,
    ctx: &ExpressionCompilationContext,
    token: Token,
) -> Result<ExpressionSpecInvocation, eyre::Report> {
    //Handle built-ins, like $container
    if BUILTIN_MAP.contains_key(sym) {
        unimplemented!("Built-ins like $bounds are not yet supported")
    } else {
        let prop_def_chain = ctx
            .resolve_symbol_as_prop_def(&sym, token.clone())?
            .ok_or::<eyre::Report>(PaxTemplateError::new(
                Some(format!("symbol not found: {}", &sym)),
                token.clone(),
            ))?;
        let nested_prop_def = prop_def_chain.last().unwrap();

        let split_symbols = clean_and_split_symbols(&sym);
        #[allow(unused)]
        let is_nested_numeric = split_symbols.len() > 1
            && ExpressionSpecInvocation::is_numeric(&nested_prop_def.type_id);

        #[allow(unused)]
        let escaped_identifier = escape_identifier(split_symbols.join("."));
        let nested_prop_def = prop_def_chain.last().unwrap();
        let is_nested_numeric = ExpressionSpecInvocation::is_numeric(&nested_prop_def.type_id);

        let split_symbols = clean_and_split_symbols(&sym);
        let escaped_identifier = escape_identifier(split_symbols.join("."));

        let mut split_symbols = split_symbols.into_iter();
        let root_identifier = split_symbols.next().unwrap().trim().to_string();
        let root_prop_def = prop_def_chain.first().unwrap();

        let fully_qualified_properties_struct_type =
            ctx.component_def.type_id.import_path().unwrap();

        let fully_qualified_iterable_type = if root_prop_def.flags.is_binding_repeat_i {
            "usize".to_string()
        } else {
            root_prop_def.type_id.get_unique_identifier()
        };

        let mut found_depth: Option<usize> = None;
        let mut current_depth = 0;
        let mut found_val: Option<PropertyDefinition> = None;
        while let None = found_depth {
            let map = ctx
                .scope_stack
                .get((ctx.scope_stack.len() - 1) - current_depth)
                .unwrap();
            if let Some(val) = map.get(&root_identifier) {
                found_depth = Some(current_depth);
                found_val = Some(val.clone());
            } else {
                current_depth += 1;
            }
        }

        let stack_offset = found_depth.unwrap();

        let found_val = found_val.ok_or::<eyre::Report>(PaxTemplateError::new(
            Some(format!("Property not found {}", sym)),
            token.clone(),
        ))?;
        let property_flags = found_val.flags;
        let property_type = &root_prop_def.type_id;

        let mut nested_symbol_tail_literal = "".to_string();
        prop_def_chain.iter().enumerate().for_each(|(i, elem)| {
            if i > 0 && i < prop_def_chain.len() {
                nested_symbol_tail_literal += &if elem.flags.is_property_wrapped {
                    format!(".{}.get()", elem.name)
                } else {
                    format!(".{}", elem.name)
                };
            }
        });
        if nested_symbol_tail_literal != "" {
            nested_symbol_tail_literal += ".clone()"
        }

        Ok(ExpressionSpecInvocation {
            root_identifier,
            is_numeric: ExpressionSpecInvocation::is_numeric(&property_type),
            is_bool: ExpressionSpecInvocation::is_primitive_bool(&property_type),
            is_string: ExpressionSpecInvocation::is_primitive_string(&property_type),
            escaped_identifier,
            stack_offset,
            fully_qualified_iterable_type,
            fully_qualified_properties_struct_type,
            property_flags,
            nested_symbol_tail_literal,
            is_nested_numeric,
        })
    }
}

/// Returns (RIL string, list of invocation specs for any symbols used)
fn compile_paxel_to_ril<'a>(
    paxel: Token,
    ctx: &ExpressionCompilationContext<'a>,
) -> Result<(String, Vec<ExpressionSpecInvocation>), eyre::Report> {
    //1. run Pratt parser; generate output RIL and collected symbolic_ids
    let (output_string, symbolic_ids) = crate::parsing::run_pratt_parser(&paxel.token_value);

    //2. for each symbolic id discovered during parsing, resolve that id through scope_stack and populate an ExpressionSpecInvocation
    let invocations_result: Result<Vec<_>, _> = symbolic_ids
        .iter()
        .map(|sym| resolve_symbol_as_invocation(&sym.trim(), ctx, paxel.clone()))
        .collect();

    let invocations = match invocations_result {
        Ok(mut invocations) => {
            invocations.sort_by(|esi0, esi1| esi0.escaped_identifier.cmp(&esi1.escaped_identifier));
            invocations.dedup_by(|esi0, esi1| esi0.escaped_identifier == esi1.escaped_identifier);
            invocations
        }
        Err(e) => return Err(e),
    };

    //3. return tuple of (RIL string,ExpressionSpecInvocations)
    Ok((output_string, invocations))
}

pub struct ExpressionCompilationContext<'a> {
    /// Current component definition, i.e. the `Component` that houses
    /// any compiled expressions and related property definitions
    pub component_def: &'a ComponentDefinition,

    /// Container for mutable list of TemplateNodeDefinitions,
    pub template: &'a mut ComponentTemplate,

    /// Static stack of addressable properties, by string
    /// Enables resolution of scope-nested symbolic identifiers, including shadowing
    pub scope_stack: Vec<HashMap<String, PropertyDefinition>>,

    /// Generator used to create monotonically increasing, compilation-unique integer IDs
    /// Used at least for expression vtable id generation
    pub vtable_uid_gen: RangeFrom<usize>,

    /// Mutable reference to a traversal-global map of ExpressionSpecs,
    /// to be appended to as expressions are compiled during traversal
    pub expression_specs: &'a mut HashMap<usize, ExpressionSpec>,

    /// The current template node whose expressions are being compiled.  For example `<SomeNode some_property={/* some expression */} />`
    pub active_node_id: Option<TemplateNodeId>,

    /// All components, by ID
    pub all_components: BTreeMap<TypeId, ComponentDefinition>,

    /// Type table, used for looking up property types by string type_ids
    pub type_table: &'a TypeTable,

    pub host_crate_info: &'a HostCrateInfo,
}

lazy_static! {
    static ref BUILTIN_MAP : HashMap<&'static str, ()> = HashMap::from([
        //TODO! hook into real runtime logic here instead of PropertyDefinition::default.
        //      this probably requires referring to event handlers instead of directly to PropertyDefinition via HashMap<String, PropertyDefinition>
        ("$container",())
    ]);
}

pub fn clean_and_split_symbols(possibly_nested_symbols: &str) -> Vec<String> {
    let entire_symbol = if possibly_nested_symbols.starts_with("self.") {
        possibly_nested_symbols.replacen("self.", "", 1)
    } else if possibly_nested_symbols.starts_with("this.") {
        possibly_nested_symbols.replacen("this.", "", 1)
    } else {
        possibly_nested_symbols.to_string()
    };

    let trimmed_symbol = entire_symbol.trim();

    trimmed_symbol
        .split(".")
        .map(|atomic_symbol| atomic_symbol.to_string())
        .collect::<Vec<_>>()
}

impl<'a> ExpressionCompilationContext<'a> {
    /// for an input symbol like `i` or `self.num_clicks`
    /// traverse the self-attached `scope_stack`
    /// and return a copy of the related `PropertyDefinition`, if found.
    /// For
    pub fn resolve_symbol_as_prop_def(
        &self,
        symbol: &str,
        token: Token,
    ) -> Result<Option<Vec<PropertyDefinition>>, eyre::Report> {
        let split_symbols = clean_and_split_symbols(symbol);
        let mut split_symbols = split_symbols.iter();

        let root_symbol = split_symbols.next().unwrap();

        let root_symbol_pd = if BUILTIN_MAP.contains_key(root_symbol.as_str()) {
            // resolve root symbol through builtin map
            None //FUTURE: support built-ins
        } else {
            // resolve through scope stack
            let mut found = false;
            let mut exhausted = false;
            let mut iter = self.scope_stack.iter();
            let mut current_frame = iter.next();
            let mut ret: Option<PropertyDefinition> = None;
            while !found && !exhausted {
                if let Some(frame) = current_frame {
                    if let Some(pv) = frame.get(root_symbol) {
                        ret = Some(pv.clone());
                        found = true;
                    }
                    current_frame = iter.next();
                } else {
                    exhausted = true;
                }
            }
            ret
        };

        // handle nested symbols like `foo.bar`.
        if let Some(root_symbol_pd) = root_symbol_pd {
            let mut ret = vec![root_symbol_pd];
            for atomic_symbol in split_symbols {
                let td = ret.last().unwrap().get_type_definition(self.type_table);
                // return terminal nested symbol's PropertyDefinition, or root's if there are no nested symbols
                let next_pd = td
                    .property_definitions
                    .iter()
                    .find(|pd| pd.name == *atomic_symbol)
                    .ok_or::<eyre::Report>(PaxTemplateError::new(
                        Some(format!(
                            "Unable to resolve nested symbol `{}` while evaluating `{}`.",
                            atomic_symbol, symbol
                        )),
                        token.clone(),
                    ))?
                    .clone();
                ret.push(next_pd);
            }
            Ok(Some(ret))
        } else {
            Ok(None)
        }
    }
}
