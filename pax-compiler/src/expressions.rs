use super::manifest::{TemplateNodeDefinition, PaxManifest, ExpressionSpec, ExpressionSpecInvocation, ComponentDefinition, ControlFlowRepeatPredicateDeclaration, AttributeValueDefinition, PropertyDefinition};
use std::collections::HashMap;
use std::ops::{Range, RangeFrom};
use futures::StreamExt;


pub fn compile_all_expressions<'a>(manifest: &'a mut PaxManifest) {

    let mut new_expression_specs : HashMap<usize, ExpressionSpec> = HashMap::new();
    let mut stack_offset = 0;
    let mut uid_gen = 0..;

    let mut component_id_map = HashMap::new();

    for cd in manifest.components.iter() {
        component_id_map.insert(&cd.source_id, &*cd);
    }

    let mut new_components = manifest.components.clone();
    new_components.iter_mut().for_each(|component_def : &mut ComponentDefinition|{

        let mut new_component_def = component_def.clone();
        let read_only_component_def = component_def.clone();


        if let Some(ref mut template) = new_component_def.template {
            template.iter_mut().for_each(|node_def| {
                let mut new_node_def = node_def.clone();
                let mut ctx = TemplateTraversalContext {
                    active_node_def: new_node_def,
                    scope_stack: vec![component_def.property_definitions.iter().map(|pd| {(pd.name.clone(), pd.clone())}).collect()],
                    uid_gen: 0..,
                    expression_specs: &mut new_expression_specs,
                    component_def: &read_only_component_def,
                    template_node_definitions: manifest.template_node_definitions.clone(),
                };

                ctx = recurse_template_and_compile_expressions(ctx);

                std::mem::swap(node_def, &mut ctx.active_node_def);
                std::mem::swap(&mut manifest.template_node_definitions, &mut ctx.template_node_definitions);
            });
        }

        std::mem::swap(component_def, &mut new_component_def);

    });
    manifest.components = new_components;
    manifest.expression_specs = Some(new_expression_specs);

    println!("{}", serde_json::to_string_pretty(&manifest).unwrap());
}


fn recurse_template_and_compile_expressions<'a>(mut ctx: TemplateTraversalContext<'a>) -> TemplateTraversalContext<'a> {
    let mut incremented = false;

    //TODO: join settings blocks here, merge with inline_attributes
    let mut cloned_inline_attributes = ctx.active_node_def.inline_attributes.clone();
    let mut cloned_control_flow_attributes = ctx.active_node_def.control_flow_attributes.clone();


    if let Some(ref mut inline_attributes) = cloned_inline_attributes {
        //Handle non-control-flow declarations
        inline_attributes.iter_mut().for_each(|attr| {
            match &mut attr.1 {
                AttributeValueDefinition::LiteralValue(_) => {
                    //no need to compile literal values
                }
                AttributeValueDefinition::EventBindingTarget(s) => {
                    //TODO: bind events here, or on a separate pass?
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

                    ctx.expression_specs.insert(id, ExpressionSpec {
                        id,
                        pascalized_return_type: (&ctx.component_def.property_definitions.iter().find(|property_def| {
                            property_def.name == attr.0
                        }).unwrap().fully_qualified_type.get(0).unwrap().pascalized_fully_qualified_type).clone(),
                        invocations: vec![
                            todo!("add unique identifiers found during PAXEL parsing; include stack offset")
                            //note that each identifier may have a different stack offset value, meaning that ids must be resolved statically
                            //(requires looking up identifiers per "compiletime stack frame," e.g. components/control flow, plus error handling if symbols aren't found.)
                        ],
                        output_statement: output_statement,
                        input_statement: identifier.clone(),
                    });
                }
                AttributeValueDefinition::Expression(input, manifest_id) => {
                    // e.g. the `self.num_clicks + 5` in `<SomeNode some_property={self.num_clicks + 5} />`
                    let id = ctx.uid_gen.next().unwrap();

                    //Write this id back to the manifest, for downstream use by RIL component tree generator
                    let mut manifest_id_insert: usize = id;
                    std::mem::swap(&mut manifest_id.take().unwrap(), &mut manifest_id_insert);

                    let output_statement = Some(compile_paxel_to_ril(&input, &ctx));

                    ctx.expression_specs.insert(id, ExpressionSpec {
                        id,
                        pascalized_return_type: (&ctx.component_def.property_definitions.iter().find(|property_def| {
                            property_def.name == attr.0
                        }).unwrap().fully_qualified_type.get(0).unwrap().pascalized_fully_qualified_type).clone(),
                        invocations: vec![
                            todo!("add unique identifiers found during PAXEL parsing; include stack offset")
                            //note that each identifier may have a different stack offset value, meaning that ids must be resolved statically
                            //(requires looking up identifiers per "compiletime stack frame," e.g. components/control flow, plus error handling if symbols aren't found.)
                        ],
                        output_statement: "".to_string(),
                        input_statement: input.clone(),
                    });


                    //Write this id back to the manifest, for downstream use by RIL component tree generator
                    let mut manifest_id_insert = Some(id);
                    std::mem::swap(manifest_id, &mut manifest_id_insert);
                }
            }
        });
    } else if let Some(ref mut cfa) = cloned_control_flow_attributes {
        //Handle control flow declarations

        if let Some(range) = cfa.repeat_source_definition.range_expression {

            todo!("Register `range` as an expression with return type usize — allow expression compiler to handle everything else")
            //
            // let id = ctx.uid_gen.next().unwrap();
            //
            // // Examples:
            // // for (elem, i) in self.elements
            // //  - must be a symbolic identifier, such as `elements` or `self.elements`
            // // for i in 0..max_elems
            // //  - may use an integer literal or symbolic identifier in either position
            // //  - may use an exclusive (..) or inclusive (...) range operator
            // //  -
            //
            // match cfa.repeat_predicate_declaration.unwrap() {
            //     ControlFlowRepeatPredicateDeclaration::ElemId(elem_id) => {
            //         ctx.scope_stack.push(HashMap::from((
            //             (elem_id.clone(), PropertyDefinition {
            //                 name: elem_id.clone(),
            //                 fully_qualified_type: cfa
            //             })
            //         )));
            //     },
            //     ControlFlowRepeatPredicateDeclaration::ElemIdIndexId(elem_id, index_id) => {
            //
            //     },
            // }
            //
            // ctx.expression_specs.insert(id, ExpressionSpec {
            //     id,
            //     pascalized_return_type: (&ctx.component_def.property_definitions.iter().find(|property_def| {
            //         property_def.name == ""
            //     }).unwrap().pascalized_fully_qualified_type).clone(),
            //     invocations: vec![
            //         todo!("add unique identifiers found during PAXEL parsing; include stack offset")
            //         //note that each identifier may have a different stack offset value, meaning that ids must be resolved statically
            //         //(requires looking up identifiers per "compiletime stack frame," e.g. components/control flow, plus error handling if symbols aren't found.)
            //     ],
            //     output_statement: "".to_string(),
            //     input_statement: expression.clone(),
            // });
        } else if let Some(symbol) = cfa.repeat_source_definition.symbolic_binding {
            //for example the `self.entries` in `for n in self.entries`

            //Do we resolve `symbol` as an expression with known return type -- or
            // can we just pass in the inner type T for Property<Vec<T>>, so that the symbolic identifier
            // can be resolved directly within the expression, using `T` for `datum_cast`
            todo!("resolve `symbol` as an expression, with return type ");
        }
    }

    std::mem::swap(&mut cloned_inline_attributes, &mut ctx.active_node_def.inline_attributes);

    for id in ctx.active_node_def.children_ids.clone().iter() {
        let mut active_node_def = ctx.template_node_definitions.remove(id).unwrap();
        ctx.active_node_def = active_node_def;

        ctx = recurse_template_and_compile_expressions(ctx);
        ctx.template_node_definitions.insert(id.to_string(), ctx.active_node_def.clone());
    };

    if incremented {
        ctx.scope_stack.pop();
    }
    ctx
}


/// From a symbol like `num_clicks` or `self.num_clicks`, populate an ExpressionSpecInvocation
fn resolve_symbol_as_invocation(sym: &str, ctx: &TemplateTraversalContext) -> ExpressionSpecInvocation {

    let identifier =  if sym.starts_with("self.") {
        sym.replacen("self.", "", 1);
    } else if sym.starts_with("this.") {
        sym.replacen("this.", "", 1);
    } else {
        sym.to_string()
    };

    let prop_def = ctx.component_def.property_definitions.iter().find(|ppd|{ppd.name}).expect(format!("Symbol not found: {}", &identifier));
    let properties_type = prop_def.fully_qualified_type.fully_qualified_type;

    let pascalized_datum_cast_type = if let Some(x) = &prop_def.datum_cast_type {
        Some(x.pascalized_fully_qualified_type)
    } else {
        None
    };

    let stack_offset = todo!("traverse stack to determine the first depth where this id occurs");

    ExpressionSpecInvocation {
        identifier,
        stack_offset,
        properties_type,
        pascalized_datum_cast_type,
        is_repeat_elem: false,
        is_repeat_index: false
    }
}




/// Returns (RIL string, list of invocation specs for any symbols used)
fn compile_paxel_to_ril<'a>(paxel: &str, ctx: &TemplateTraversalContext<'a>) -> (String, Vec<ExpressionSpecInvocation>) {
    todo!("");

    //1. run Pratt parser; generate output RIL
    let (output_string, symbolic_ids) = run_pratt_parser();

    //2. for each xo_symbol discovered during parsing, resolve that symbol through scope_stack and populate an ExpressionSpecInvocation
    let invocations = symbolic_ids.iter().map(|sym| {
        let inv = resolve_symbol_as_invocation(&sym, ctx);
        inv
    });
    //3. return tuple of (RIL string,ExpressionSpecInvocations)

    (output_string ,invocations)

}

pub struct TemplateTraversalContext<'a> {
    pub active_node_def: TemplateNodeDefinition,
    pub component_def: &'a ComponentDefinition,
    pub scope_stack: Vec<HashMap<String, PropertyDefinition>>,
    pub uid_gen: RangeFrom<usize>,
    pub expression_specs: &'a mut HashMap<usize, ExpressionSpec>,
    pub template_node_definitions: HashMap<String, TemplateNodeDefinition>,
}