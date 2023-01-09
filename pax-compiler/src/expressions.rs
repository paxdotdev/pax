use std::borrow::Borrow;
use super::manifest::{TemplateNodeDefinition, PaxManifest, ExpressionSpec, ExpressionSpecInvocation, ComponentDefinition, ControlFlowRepeatPredicateDefinition, AttributeValueDefinition, PropertyDefinition};
use std::collections::HashMap;
use std::ops::{Range, RangeFrom};
use futures::StreamExt;
use crate::manifest::PropertyType;

pub fn compile_all_expressions<'a>(manifest: &'a mut PaxManifest) {

    //From a fully populated manifest, build

    let mut new_expression_specs : HashMap<usize, ExpressionSpec> = HashMap::new();
    let mut stack_offset = 0;
    let mut uid_gen = 0..;


    let mut new_components = manifest.components.clone();
    new_components.values_mut().for_each(|component_def : &mut ComponentDefinition|{

        let mut new_component_def = component_def.clone();
        let read_only_component_def = component_def.clone();

        if let Some(ref mut template) = new_component_def.template {

            // todo!"must walk template following uuid linked list -- NOT iter_mut";


            let root_template_node_id = new_component_def.root_template_node_id.as_ref().expect("cannot compile template without root node");

            let mut new_template_node_definitions = manifest.template_node_definitions.clone();

            let mut active_node_def = new_template_node_definitions.remove(root_template_node_id).unwrap();

            let mut ctx = ExpressionCompilationContext {
                active_node_def,
                scope_stack: vec![component_def.property_definitions.iter().map(|pd| {(pd.name.clone(), pd.clone())}).collect()],
                uid_gen: 0..,
                all_components: manifest.components.clone(),
                expression_specs: &mut new_expression_specs,
                component_def: &read_only_component_def,
                new_template_node_definitions,
            };

            ctx = recurse_compile_expressions(ctx);

            manifest.template_node_definitions.extend(ctx.new_template_node_definitions);

            // template.iter_mut().for_each(|node_def| {
            //     let mut new_node_def = node_def.clone();
            //
            //
            //     ctx = recurse_compile_expressions(ctx);
            //
            //     std::mem::swap(node_def, &mut ctx.active_node_def);
            //
            //     manifest.template_node_definitions.extend(ctx.new_template_node_definitions);
            //
            // });
        }

        std::mem::swap(component_def, &mut new_component_def);

    });
    manifest.components = new_components;
    manifest.expression_specs = Some(new_expression_specs);

    println!("{}", serde_json::to_string_pretty(&manifest).unwrap());
}

fn recurse_compile_expressions<'a>(mut ctx: ExpressionCompilationContext<'a>) -> ExpressionCompilationContext<'a> {
    let mut incremented = false;

    //FUTURE: join settings blocks here, merge with inline_attributes
    let mut cloned_inline_attributes = ctx.active_node_def.inline_attributes.clone();
    let mut cloned_control_flow_attributes = ctx.active_node_def.control_flow_attributes.clone();

    if let Some(ref mut inline_attributes) = cloned_inline_attributes {
        //Handle standard key/value declarations (non-control-flow)
        inline_attributes.iter_mut().for_each(|attr| {
            match &mut attr.1 {
                AttributeValueDefinition::LiteralValue(_) => {
                    //no need to compile literal values
                }
                AttributeValueDefinition::EventBindingTarget(s) => {
                    //TODO: bind events here
                    // e.g. the self.foo in `@click=self.foo`
                }
                AttributeValueDefinition::Identifier(identifier, manifest_id) => {
                    // e.g. the self.active_color in `bg_color=self.active_color`

                    let id = ctx.uid_gen.next().unwrap();

                    //Write this id back to the manifest, for downstream use by RIL component tree generator
                    let mut manifest_id_insert: usize = id;
                    std::mem::swap(&mut manifest_id.take().unwrap(), &mut manifest_id_insert);

                    //a single identifier binding is the same as an expression returning that identifier, `{self.some_identifier}`
                    //thus, we can compile it as PAXEL and make use of any shared logic, e.g. `self`/`this` handling
                    let (output_statement, invocations) = compile_paxel_to_ril(&identifier, &ctx);

                    let pascalized_return_type = (&ctx.component_def.property_definitions.iter().find(
                        |property_def| {
                            property_def.name == attr.0
                        }
                    ).unwrap().property_type_info.pascalized_fully_qualified_type).clone();

                    ctx.expression_specs.insert(id, ExpressionSpec {
                        id,
                        pascalized_return_type,
                        invocations,
                        output_statement: output_statement,
                        input_statement: identifier.clone(),
                    });
                }
                AttributeValueDefinition::Expression(input, manifest_id) => {
                    // e.g. the `self.num_clicks + 5` in `<SomeNode some_property={self.num_clicks + 5} />`
                    let id = ctx.uid_gen.next().unwrap();

                    //Write this id back to the manifest, for downstream use by RIL component tree generator
                    let mut manifest_id_insert: usize = id;
                    std::mem::swap(&mut manifest_id.take(), &mut Some(manifest_id_insert));

                    //TODO! We don't have the appropriate runtime stack at this point
                    let (output_statement, invocations) = compile_paxel_to_ril(&input, &ctx);

                    let active_node_component = (&ctx.all_components.get(&ctx.active_node_def.component_id)).expect(&format!("No known component with identifier {}.  Try importing or defining a component named {}", &ctx.active_node_def.component_id, &ctx.active_node_def.component_id));

                    let pascalized_return_type = if attr.0 == "transform" {
                        //FUTURE: DRY & robustify
                        "Transform2D".to_string()
                    } else if attr.0 == "size" {
                        //FUTURE: DRY & robustify
                        "Size2D".to_string()
                    } else {
                        (active_node_component.property_definitions.iter().find(|property_def| {
                            property_def.name == attr.0
                        }).expect(
                        &format!("Property `{}` not found on component `{}`", &attr.0, &ctx.component_def.pascal_identifier)
                        ).property_type_info.pascalized_fully_qualified_type).clone()
                    };
                    ctx.expression_specs.insert(id, ExpressionSpec {
                        id,
                        pascalized_return_type,
                        invocations,
                        output_statement,
                        input_statement: input.clone(),
                    });

                    //Write this id back to the manifest, for downstream use by RIL component tree generator
                    let mut manifest_id_insert = Some(id);
                    std::mem::swap(manifest_id, &mut manifest_id_insert);
                }
            }
        });
    } else if let Some(ref mut cfa) = cloned_control_flow_attributes {
        //Handle attributes for control flow

        // Definitions are stored modally as `Option<T>`s in ControlFlowAttributeValueDefinition,
        // so: iff `repeat_source_definition` is present, then we can assume this is a Repeat element
        if let Some(repeat_source_definition) = &cfa.repeat_source_definition {
            // Examples:
            // for (elem, i) in self.elements
            //  - must be a symbolic identifier, such as `elements` or `self.elements`
            // for i in 0..max_elems
            //  - may use an integer literal or symbolic identifier in either position
            //  - may use an exclusive (..) or inclusive (...) range operator

            let id = ctx.uid_gen.next().unwrap();

            // Attach shadowed property symbols to the scope_stack, so e.g. `elem` can be
            // referred to with the symbol `elem` in PAXEL
            match cfa.repeat_predicate_definition.as_ref().unwrap() {
                ControlFlowRepeatPredicateDefinition::ElemId(elem_id) => {
                    let mut property_definition = ctx.component_def.property_definitions.iter().find(|pd|{pd.name.eq(elem_id)}).expect(&format!("Property not found with name {}", &elem_id)).clone();
                    let fqt = property_definition.property_type_info.fully_qualified_type.clone();
                    property_definition.property_type_info = property_definition.iterable_type.clone().expect(&format!("Cannot use type Property<{}> with `for` -- can only use `for` with a `Property<Vec<T>>`", &fqt));

                    ctx.scope_stack.push(HashMap::from([
                        //`elem` property (by specified name)
                        (elem_id.clone(),
                        property_definition)
                    ]));
                },
                ControlFlowRepeatPredicateDefinition::ElemIdIndexId(elem_id, index_id) => {
                    let mut elem_property_definition = ctx.component_def.property_definitions.iter().find(|pd|{pd.name == *elem_id}).expect(&format!("Property not found with name {}", &elem_id)).clone();
                    elem_property_definition.property_type_info = elem_property_definition.iterable_type.clone().expect(&format!("Cannot use type Property<{}> with `for` -- can only use `for` with a `Property<Vec<T>>`", &elem_property_definition.property_type_info.fully_qualified_type));

                    ctx.scope_stack.push(HashMap::from([
                        //`elem` property (by specified name)
                        (elem_id.clone(),
                         elem_property_definition),
                        //`i` property (by specified name)
                        (index_id.clone(),
                            PropertyDefinition::primitive_with_name("usize", index_id)
                         )
                    ]));
                },
            };

            //handle the `self.some_data_source` in `for (elem, i) in self.some_data_source`

            let repeat_source_definition = cfa.repeat_source_definition.as_ref().unwrap();

            let (paxel, return_type) = if let Some(range_expression) = &repeat_source_definition.range_expression {
                (range_expression.to_string(), PropertyType::primitive("usize"))
            } else if let Some(symbolic_binding) = &repeat_source_definition.symbolic_binding {
                let mut property_definition = ctx.component_def.get_property_definition_by_name(symbolic_binding);
                let fqt = property_definition.property_type_info.fully_qualified_type.clone();
                let property_type = property_definition.iterable_type.clone().expect(&format!("Cannot use type Property<{}> with `for` -- can only use `for` with a `Property<Vec<T>>`", &fqt));

                (symbolic_binding.to_string(), property_type)
            } else {unreachable!()};

            //Though we are compiling this as an arbitrary expression, we must already have validated
            //that we are only binding to a simple symbolic id, like `self.foo`.  This is because we
            //are inferring the return type of this expression based on the declared-and-known
            //type of property `self.foo`
            let (output_statement, invocations) = compile_paxel_to_ril(&paxel, &ctx);

            // The return type for a repeat source expression will either be:
            //   1. usize, for tuples (including tuples with direct symbolic references as either operand, like `self.x..10`)
            //   2. T for a direct symbolic reference to `self.x` for x : Property<Vec<T>>
            // Presumably, we could also support arbitrary expressions as a #3, but
            // we need some way to infer the return type, statically.  This may mean requiring
            // an explicit type declaration by the end-user, or perhaps we can hack something
            // with further compiletime "reflection" magic

            ctx.expression_specs.insert(ctx.uid_gen.next().unwrap(), ExpressionSpec {
                id,
                pascalized_return_type: return_type.pascalized_fully_qualified_type,
                invocations,
                output_statement,
                input_statement: paxel,
            });

            //recurse through descendents to continue compiling expressions

        } else if let Some(condition_expression_paxel) = &cfa.condition_expression_paxel {
            //Handle `if` boolean expression, e.g. the `num_clicks > 5` in `if num_clicks > 5 { ... }`
            let (output_statement, invocations) = compile_paxel_to_ril(&condition_expression_paxel, &ctx);
            let id = ctx.uid_gen.next().unwrap();

            ctx.expression_specs.insert(ctx.uid_gen.next().unwrap(), ExpressionSpec {
                id,
                pascalized_return_type: "bool".to_string(),
                invocations,
                output_statement,
                input_statement: condition_expression_paxel.clone(),
            });
        }
    }

    std::mem::swap(&mut cloned_inline_attributes, &mut ctx.active_node_def.inline_attributes);

    // Traverse descendent nodes and continue compiling expressions recursively
    for id in ctx.active_node_def.children_ids.clone().iter() {
        let mut active_node_def = ctx.new_template_node_definitions.remove(id).unwrap();
        ctx.active_node_def = active_node_def;

        ctx = recurse_compile_expressions(ctx);
        ctx.new_template_node_definitions.insert(id.to_string(), ctx.active_node_def.clone());
    };

    if incremented {
        ctx.scope_stack.pop();
    }
    ctx
}


/// From a symbol like `num_clicks` or `self.num_clicks`, populate an ExpressionSpecInvocation
fn resolve_symbol_as_invocation(sym: &str, ctx: &ExpressionCompilationContext) -> ExpressionSpecInvocation {

    let identifier =  if sym.starts_with("self.") {
        sym.replacen("self.", "", 1)
    } else if sym.starts_with("this.") {
        sym.replacen("this.", "", 1)
    } else {
        sym.to_string()
    };

    let prop_def = ctx.resolve_symbol_through_stack(&identifier).expect(&format!("Symbol not found: {}", &identifier));

    let properties_type = prop_def.property_type_info.fully_qualified_type.clone();

    let pascalized_iterable_type = if let Some(x) = &prop_def.iterable_type {
        Some(x.pascalized_fully_qualified_type.clone())
    } else {
        None
    };

    let mut found_depth : Option<usize> = None;
    let mut current_depth = 0;
    while let None = found_depth {
        let map = ctx.scope_stack.get((ctx.scope_stack.len() - 1) - current_depth).expect(&format!("Symbol not found: {}", &identifier));
        if let Some(val) = map.get(&identifier) {
            found_depth = Some(current_depth);
        } else {
            current_depth += 1;
        }
    }

    let stack_offset = found_depth.unwrap();

    let escaped_identifier = crate::reflection::escape_identifier(identifier.clone());

    ExpressionSpecInvocation {
        identifier,
        escaped_identifier,
        stack_offset,
        properties_type,
        pascalized_iterable_type,
        is_repeat_elem: false,
        is_repeat_index: false
    }
}

/// Returns (RIL string, list of invocation specs for any symbols used)
fn compile_paxel_to_ril<'a>(paxel: &str, ctx: &ExpressionCompilationContext<'a>) -> (String, Vec<ExpressionSpecInvocation>) {

    //1. run Pratt parser; generate output RIL and collected symbolic_ids
    let (output_string,  symbolic_ids) = crate::parsing::run_pratt_parser(paxel);

    //2. for each symbolic id discovered during parsing, resolve that id through scope_stack and populate an ExpressionSpecInvocation
    let invocations = symbolic_ids.iter().map(|sym| {
        resolve_symbol_as_invocation(&sym, ctx)
    }).collect();

    //3. return tuple of (RIL string,ExpressionSpecInvocations)
    (output_string, invocations)

}

pub struct ExpressionCompilationContext<'a> {

    /// Current component definition, i.e. the `Component` that houses
    /// any compiled expressions and related property definitions
    pub component_def: &'a ComponentDefinition,

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

    /// Structure to store newly added TemplateNodeDefintions during traversal
    pub new_template_node_definitions: HashMap<String, TemplateNodeDefinition>,

    /// All components, by ID
    pub all_components: HashMap<String, ComponentDefinition>,

}

impl<'a> ExpressionCompilationContext<'a> {

    /// for an input symbol like `i` or `num_clicks` (already pruned of `self.` or `this.`)
    /// traverse the self-attached `scope_stack`
    /// and return a copy of the related `PropertyDefinition`, if found
    pub fn resolve_symbol_through_stack(&self, symbol: &str) -> Option<PropertyDefinition> {

        let mut found=false;
        let mut exhausted =false;
        let mut iter = self.scope_stack.iter();
        let mut current_frame = iter.next();
        let mut ret : Option<PropertyDefinition> = None;
        while !found && !exhausted {
            if let Some(frame) = current_frame {
                if let Some(pv) = frame.get(symbol) {
                    ret = Some(pv.clone());
                    found = true;
                }
                current_frame = iter.next();
            } else {
                exhausted = true;
            }
        }
        ret
    }
}
