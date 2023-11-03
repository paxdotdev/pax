use super::manifest::{
    ComponentDefinition, ControlFlowRepeatPredicateDefinition, ExpressionSpec,
    ExpressionSpecInvocation, PaxManifest, PropertyDefinition, SettingsSelectorBlockDefinition,
    TemplateNodeDefinition, ValueDefinition,
};
use std::collections::HashMap;
use std::ops::{IndexMut, RangeFrom};
use std::slice::IterMut;

use crate::errors::source_map::SourceMap;
use crate::errors::PaxTemplateError;
use crate::manifest::{PropertyDefinitionFlags, Token, TypeDefinition, TypeTable};
use crate::parsing::escape_identifier;
use color_eyre::eyre;
use color_eyre::eyre::Report;
use lazy_static::lazy_static;

const BUILTIN_TYPES: &'static [(&str, &str); 12] = &[
    ("transform", "Transform2D"),
    ("width", "Size"),
    ("height", "Size"),
    ("x", "Size"),
    ("y", "Size"),
    ("anchor_x", "Size"),
    ("anchor_y", "Size"),
    ("skew_x", "Numeric"),
    ("skew_y", "Numeric"),
    ("scale_x", "Size"),
    ("scale_y", "Size"),
    ("rotate", "Rotation"),
];

pub fn compile_all_expressions<'a>(
    manifest: &'a mut PaxManifest,
    source_map: &'a mut SourceMap,
) -> eyre::Result<(), Report> {
    let mut swap_expression_specs: HashMap<usize, ExpressionSpec> = HashMap::new();
    let mut all_expression_specs: HashMap<usize, ExpressionSpec> = HashMap::new();

    let mut new_components = manifest.components.clone();
    let mut uid_track = 0;

    for component_def in new_components.values_mut() {
        let mut new_component_def = component_def.clone();
        let read_only_component_def = component_def.clone();

        if let Some(ref mut template) = new_component_def.template {
            let mut active_node_def = TemplateNodeDefinition::default();
            std::mem::swap(&mut active_node_def, template.index_mut(0));

            let mut ctx = ExpressionCompilationContext {
                template,
                active_node_def,
                scope_stack: vec![component_def
                    .get_property_definitions(&manifest.type_table)
                    .iter()
                    .map(|pd| (pd.name.clone(), pd.clone()))
                    .collect()],
                uid_gen: uid_track..,
                all_components: manifest.components.clone(),
                expression_specs: &mut swap_expression_specs,
                component_def: &read_only_component_def,
                type_table: &manifest.type_table,
            };

            ctx = recurse_compile_expressions(ctx, source_map)?;
            uid_track = ctx.uid_gen.next().unwrap();
            all_expression_specs.extend(ctx.expression_specs.to_owned());
            std::mem::swap(&mut ctx.active_node_def, template.index_mut(0));
        }

        std::mem::swap(component_def, &mut new_component_def);
    }

    manifest.components = new_components;
    manifest.expression_specs = Some(swap_expression_specs);
    Ok(())
}

fn pull_matched_identifiers_from_inline(
    inline_settings: &Option<Vec<(Token, ValueDefinition)>>,
    s: String,
) -> Vec<Token> {
    let mut ret = Vec::new();
    if let Some(val) = inline_settings {
        for (_, matched) in val.iter().filter(|avd| avd.0.token_value == s.as_str()) {
            match matched {
                ValueDefinition::Identifier(s, _) => ret.push(s.clone()),
                _ => {}
            };
        }
    }
    ret
}

fn pull_settings_with_selector(
    settings: &Option<Vec<SettingsSelectorBlockDefinition>>,
    selector: String,
) -> Option<Vec<(Token, ValueDefinition)>> {
    settings.as_ref().and_then(|val| {
        let merged_settings: Vec<(Token, ValueDefinition)> = val
            .iter()
            .filter(|block| block.selector.token_value == selector)
            .flat_map(|block| block.value_block.settings_key_value_pairs.clone())
            .collect();
        (!merged_settings.is_empty()).then(|| merged_settings)
    })
}

fn merge_inline_settings_with_settings_block(
    inline_settings: &Option<Vec<(Token, ValueDefinition)>>,
    settings_block: &Option<Vec<SettingsSelectorBlockDefinition>>,
) -> Option<Vec<(Token, ValueDefinition)>> {
    // collect id settings
    let ids = pull_matched_identifiers_from_inline(&inline_settings, "id".to_string());

    let mut id_settings = Vec::new();
    if ids.len() == 1 {
        if let Some(settings) =
            pull_settings_with_selector(&settings_block, format!("#{}", ids[0].token_value))
        {
            id_settings.extend(settings.clone());
        }
    } else if ids.len() > 1 {
        panic!("Specified more than one id inline!");
    }

    // collect all class settings
    let classes = pull_matched_identifiers_from_inline(&inline_settings, "class".to_string());

    let mut class_settings = Vec::new();
    for class in classes {
        if let Some(settings) =
            pull_settings_with_selector(&settings_block, format!(".{}", class.token_value))
        {
            class_settings.extend(settings.clone());
        }
    }

    let mut map = HashMap::new();

    // Iterate in reverse order of priority (class, then id, then inline)
    for (key, value) in class_settings.into_iter() {
        map.insert(key, value);
    }

    for (key, value) in id_settings.into_iter() {
        map.insert(key, value);
    }

    if let Some(inline) = inline_settings.clone() {
        for (key, value) in inline.into_iter() {
            map.insert(key, value);
        }
    }

    let merged: Vec<(Token, ValueDefinition)> = map.into_iter().collect();
    if merged.len() > 0 {
        Some(merged)
    } else {
        None
    }
}

fn recurse_compile_literal_block<'a>(
    settings_pairs: &mut IterMut<(Token, ValueDefinition)>,
    ctx: &mut ExpressionCompilationContext,
    current_property_definitions: Vec<PropertyDefinition>,
    type_id: String,
    source_map: &mut SourceMap,
) -> Result<(), eyre::Report> {
    settings_pairs.try_for_each(|pair| {
        match &mut pair.1 {
            // LiteralValue:       no need to compile literal values
            // EventBindingTarget: event bindings are handled on a separate compiler pass; no-op here
            ValueDefinition::LiteralValue(_) | ValueDefinition::EventBindingTarget(_) => {}
            ValueDefinition::Block(block) => {
                let type_def = (current_property_definitions
                    .iter()
                    .find(|property_def| property_def.name == pair.0.token_value))
                .ok_or::<eyre::Report>(PaxTemplateError::new(
                    Some(format!(
                        "Property `{}` not found on `{}`",
                        &pair.0.token_value, type_id
                    )),
                    pair.0.clone(),
                ))?
                .get_type_definition(ctx.type_table);
                recurse_compile_literal_block(
                    &mut block.settings_key_value_pairs.iter_mut(),
                    ctx,
                    type_def.property_definitions.clone(),
                    type_def.type_id_escaped.clone(),
                    source_map,
                )?;
            }
            ValueDefinition::Expression(input, manifest_id) => {
                // e.g. the `self.num_clicks + 5` in `<SomeNode some_property={self.num_clicks + 5} />`
                let id = ctx.uid_gen.next().unwrap();

                let (output_statement, invocations) = compile_paxel_to_ril(input.clone(), &ctx)?;

                let pascalized_return_type = if let Some(type_string) = BUILTIN_TYPES
                    .iter()
                    .find(|type_str| type_str.0 == &*pair.0.token_value)
                    .map(|type_str| type_str.1)
                {
                    type_string.to_string()
                } else {
                    (current_property_definitions
                        .iter()
                        .find(|property_def| property_def.name == pair.0.token_value)
                        .ok_or::<eyre::Report>(PaxTemplateError::new(
                            Some(format!(
                                "Property `{}` not found on `{}`",
                                &pair.0.token_value, type_id
                            )),
                            pair.0.clone(),
                        ))?
                        .get_type_definition(ctx.type_table)
                        .type_id_escaped)
                        .clone()
                };

                let mut whitespace_removed_input = input.clone().token_value;
                whitespace_removed_input.retain(|c| !c.is_whitespace());

                let source_map_id = source_map.insert(input.clone());
                let input_statement =
                    source_map.generate_mapped_string(whitespace_removed_input, source_map_id);

                ctx.expression_specs.insert(
                    id,
                    ExpressionSpec {
                        id,
                        pascalized_return_type,
                        invocations,
                        output_statement,
                        input_statement,
                        is_repeat_source_iterable_expression: false,
                        repeat_source_iterable_type_id_escaped: "".to_string(),
                    },
                );

                //Write this id back to the manifest, for downstream use by RIL component tree generator
                let mut manifest_id_insert = Some(id);
                std::mem::swap(manifest_id, &mut manifest_id_insert);
            }
            ValueDefinition::Identifier(identifier, manifest_id) => {
                // e.g. the self.active_color in `bg_color=self.active_color`

                if pair.0.token_value == "id" || pair.0.token_value == "class" {
                    //No-op -- special-case `id=some_identifier` and `class=some_identifier` — we DON'T want to compile an expression {some_identifier},
                    //so we skip the case where `id` is the key
                } else {
                    let id = ctx.uid_gen.next().unwrap();

                    //Write this id back to the manifest, for downstream use by RIL component tree generator
                    let mut manifest_id_insert: Option<usize> = Some(id);
                    std::mem::swap(manifest_id, &mut manifest_id_insert);

                    //a single identifier binding is the same as an expression returning that identifier, `{self.some_identifier}`
                    //thus, we can compile it as PAXEL and make use of any shared logic, e.g. `self`/`this` handling
                    let (output_statement, invocations) =
                        compile_paxel_to_ril(identifier.clone(), &ctx)?;

                    let source_map_id = source_map.insert(identifier.clone());
                    let input_statement = source_map
                        .generate_mapped_string(identifier.token_value.clone(), source_map_id);

                    let pascalized_return_type = (&ctx
                        .component_def
                        .get_property_definitions(ctx.type_table)
                        .iter()
                        .find(|property_def| property_def.name == pair.0.token_value)
                        .unwrap()
                        .get_type_definition(ctx.type_table)
                        .type_id_escaped)
                        .clone();

                    ctx.expression_specs.insert(
                        id,
                        ExpressionSpec {
                            id,
                            pascalized_return_type,
                            invocations,
                            output_statement,
                            input_statement,
                            is_repeat_source_iterable_expression: false,
                            repeat_source_iterable_type_id_escaped: "".to_string(),
                        },
                    );
                }
            }
            _ => {
                unreachable!()
            }
        }
        Ok::<(), eyre::Report>(())
    })?;
    Ok(())
}

fn recurse_compile_expressions<'a>(
    mut ctx: ExpressionCompilationContext<'a>,
    mut source_map: &mut SourceMap,
) -> eyre::Result<ExpressionCompilationContext<'a>, Report> {
    let incremented = false;

    let cloned_settings_block = ctx.component_def.settings.clone();
    let cloned_inline_settings = ctx.active_node_def.settings.clone();
    let mut merged_settings =
        merge_inline_settings_with_settings_block(&cloned_inline_settings, &cloned_settings_block);
    let mut cloned_control_flow_settings = ctx.active_node_def.control_flow_settings.clone();

    if let Some(ref mut inline_settings) = merged_settings {
        // Handle standard key/value declarations (non-control-flow)
        let type_id = ctx.active_node_def.type_id.clone();
        let pascal_identifier;
        let property_def;

        // Scope created to limit the borrow of ctx
        {
            let active_node_component = ctx.all_components.get(&type_id)
                .expect(&format!("No known component with identifier {}.  Try importing or defining a component named {}", &type_id, &type_id));

            pascal_identifier = active_node_component.pascal_identifier.clone();
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

            let id = ctx.uid_gen.next().unwrap();
            repeat_source_definition.vtable_id = Some(id);

            // Handle the `self.some_data_source` in `for (elem, i) in self.some_data_source`
            let repeat_source_definition = cfa.repeat_source_definition.as_ref().unwrap();
            // todo!("map 'this is a source' into a flag for codegen, so we can rewrap Rc<>s");

            let is_repeat_source_range = repeat_source_definition.range_expression_paxel.is_some();
            let is_repeat_source_iterable = repeat_source_definition.symbolic_binding.is_some();

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
                    TypeDefinition::builtin_vec_rc_properties_coproduct(inner_iterable_type_id),
                )
            } else {
                unreachable!()
            };

            let repeat_source_iterable_type_id_escaped =
                if let Some(iiti) = return_type.inner_iterable_type_id {
                    escape_identifier(iiti.clone())
                } else {
                    "".to_string()
                };

            //Though we are compiling this as an arbitrary expression, we must already have validated
            //with the parser that we are only binding to a simple symbolic id, like `self.foo`.
            //This is because we are inferring the return type of this expression based on the declared-and-known
            //type of property `self.foo`
            let (output_statement, invocations) = compile_paxel_to_ril(paxel.clone(), &ctx)?;

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
                        },
                        type_id: "isize".to_string(),
                    };

                    let scope: HashMap<String, PropertyDefinition> = HashMap::from([
                        //`elem` property (by specified name)
                        (elem_id.token_value.clone(), property_definition),
                    ]);

                    ctx.scope_stack.push(scope);
                }
                ControlFlowRepeatPredicateDefinition::ElemIdIndexId(elem_id, index_id) => {
                    //if repeat_source is a range, this is simply isize
                    //if repeat_source is a symbolic binding, then we resolve that symbolic binding and use that resolved type here
                    let iterable_type =
                        if let Some(_) = &repeat_source_definition.range_expression_paxel {
                            TypeDefinition::primitive("isize")
                        } else if let Some(symbolic_binding) =
                            &repeat_source_definition.symbolic_binding
                        {
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

                    let elem_property_definition = PropertyDefinition {
                        name: format!("{}", elem_id.token_value),
                        type_id: iterable_type.type_id,
                        flags: PropertyDefinitionFlags {
                            is_binding_repeat_elem: true,
                            is_binding_repeat_i: false,
                            is_repeat_source_range,
                            is_repeat_source_iterable,
                            is_property_wrapped: true,
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
                    };

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

            ctx.expression_specs.insert(
                id,
                ExpressionSpec {
                    id,
                    pascalized_return_type: return_type.type_id_escaped,
                    invocations,
                    output_statement,
                    input_statement,
                    is_repeat_source_iterable_expression: is_repeat_source_iterable,
                    repeat_source_iterable_type_id_escaped,
                },
            );
        } else if let Some(condition_expression_paxel) = &cfa.condition_expression_paxel {
            //Handle `if` boolean expression, e.g. the `num_clicks > 5` in `if num_clicks > 5 { ... }`
            let (output_statement, invocations) =
                compile_paxel_to_ril(condition_expression_paxel.clone(), &ctx)?;
            let id = ctx.uid_gen.next().unwrap();

            cfa.condition_expression_vtable_id = Some(id);

            let mut whitespace_removed_input = condition_expression_paxel.clone().token_value;
            whitespace_removed_input.retain(|c| !c.is_whitespace());

            let source_map_id = source_map.insert(condition_expression_paxel.clone());
            let input_statement =
                source_map.generate_mapped_string(whitespace_removed_input, source_map_id);

            ctx.expression_specs.insert(
                id,
                ExpressionSpec {
                    id,
                    pascalized_return_type: "bool".to_string(),
                    invocations,
                    output_statement,
                    input_statement,
                    is_repeat_source_iterable_expression: false,
                    repeat_source_iterable_type_id_escaped: "".to_string(),
                },
            );
        } else if let Some(slot_index_expression_paxel) = &cfa.slot_index_expression_paxel {
            //Handle `if` boolean expression, e.g. the `num_clicks > 5` in `if num_clicks > 5 { ... }`
            let (output_statement, invocations) =
                compile_paxel_to_ril(slot_index_expression_paxel.clone(), &ctx)?;
            let id = ctx.uid_gen.next().unwrap();

            cfa.slot_index_expression_vtable_id = Some(id);

            let mut whitespace_removed_input = slot_index_expression_paxel.clone().token_value;
            whitespace_removed_input.retain(|c| !c.is_whitespace());

            let source_map_id = source_map.insert(slot_index_expression_paxel.clone());
            let input_statement =
                source_map.generate_mapped_string(whitespace_removed_input, source_map_id);

            ctx.expression_specs.insert(
                id,
                ExpressionSpec {
                    id,
                    pascalized_return_type: "Numeric".to_string(),
                    invocations,
                    output_statement,
                    input_statement,
                    is_repeat_source_iterable_expression: false,
                    repeat_source_iterable_type_id_escaped: "".to_string(),
                },
            );
        } else {
            unreachable!("encountered invalid control flow definition")
        }

        // Write back our modified control_flow_settings, which now contain vtable lookup ids
        std::mem::swap(
            &mut cloned_control_flow_settings,
            &mut ctx.active_node_def.control_flow_settings,
        );
    }

    std::mem::swap(&mut merged_settings, &mut ctx.active_node_def.settings);

    // Traverse descendent nodes and continue compiling expressions recursively
    for id in ctx.active_node_def.child_ids.clone().iter() {
        //Create two blanks
        let mut active_node_def = TemplateNodeDefinition::default();
        let mut old_active_node_def = TemplateNodeDefinition::default();

        //Swap the first blank for the node with specified id
        std::mem::swap(&mut active_node_def, ctx.template.get_mut(*id).unwrap());

        //Swap the second blank for the current ctx.active_node_def value, so we can pass it back
        //to caller when done
        std::mem::swap(&mut old_active_node_def, &mut ctx.active_node_def);

        //Arm ctx with the newly retrieved, mutable active_node_def
        ctx.active_node_def = active_node_def;

        //Recurse
        ctx = recurse_compile_expressions(ctx, source_map)?;

        //Pull the (presumably mutated) active_node_def back out of ctx and attach it back into `template`
        std::mem::swap(&mut ctx.active_node_def, ctx.template.get_mut(*id).unwrap());

        //Put old active_node_def back in place so we can return it to caller
        std::mem::swap(&mut old_active_node_def, &mut ctx.active_node_def);
    }

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
        let is_nested_numeric = ExpressionSpecInvocation::is_numeric(&nested_prop_def.type_id);

        let split_symbols = clean_and_split_symbols(&sym);
        let escaped_identifier = escape_identifier(split_symbols.join("."));

        let mut split_symbols = split_symbols.into_iter();
        let root_identifier = split_symbols.next().unwrap().to_string();
        let root_prop_def = prop_def_chain.first().unwrap();

        let properties_coproduct_type = ctx.component_def.type_id_escaped.clone();

        let iterable_type_id_escaped = if root_prop_def.flags.is_binding_repeat_elem {
            escape_identifier(root_prop_def.type_id.clone())
        } else if root_prop_def.flags.is_binding_repeat_i {
            "usize".to_string()
        } else {
            "".to_string()
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
        let property_properties_coproduct_type = &root_prop_def
            .get_type_definition(ctx.type_table)
            .type_id
            .split("::")
            .last()
            .unwrap();

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
            is_numeric: ExpressionSpecInvocation::is_numeric(&property_properties_coproduct_type),
            is_bool: ExpressionSpecInvocation::is_primitive_bool(
                &property_properties_coproduct_type,
            ),
            is_string: ExpressionSpecInvocation::is_primitive_string(
                &property_properties_coproduct_type,
            ),
            escaped_identifier,
            stack_offset,
            iterable_type_id_escaped,
            properties_coproduct_type,
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
    pub template: &'a mut Vec<TemplateNodeDefinition>,

    /// Static stack of addressable properties, by string
    /// Enables resolution of scope-nested symbolic identifiers, including shadowing
    pub scope_stack: Vec<HashMap<String, PropertyDefinition>>,

    /// Generator used to create monotonically increasing, compilation-unique integer IDs
    /// Used at least for expression vtable id generation
    pub uid_gen: RangeFrom<usize>,

    /// Mutable reference to a traversal-global map of ExpressionSpecs,
    /// to be appended to as expressions are compiled during traversal
    pub expression_specs: &'a mut HashMap<usize, ExpressionSpec>,

    /// The current template node whose expressions are being compiled.  For example `<SomeNode some_property={/* some expression */} />`
    pub active_node_def: TemplateNodeDefinition,

    /// All components, by ID
    pub all_components: HashMap<String, ComponentDefinition>,

    /// Type table, used for looking up property types by string type_ids
    pub type_table: &'a TypeTable,
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

    entire_symbol
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
