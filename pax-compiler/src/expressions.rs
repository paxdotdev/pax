use super::manifest::{TemplateNodeDefinition, PaxManifest, ExpressionSpec, ExpressionSpecInvocation, ComponentDefinition, ControlFlowRepeatPredicateDeclaration, AttributeValueDefinition, PropertyDefinition};
use std::collections::HashMap;
use std::ops::RangeFrom;


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

    //only need to push stack frame for Repeat, not for Conditional or Slot
    if ctx.active_node_def.pascal_identifier == "Repeat" {

        let predicate_declaration = ctx.active_node_def.control_flow_attributes.clone().unwrap().repeat_predicate_declaration.unwrap();
        match predicate_declaration {
            ControlFlowRepeatPredicateDeclaration::Identifier(elem_id) => {
                ctx.scope_stack.push(HashMap::from([(elem_id.to_string(), PropertyDefinition {
                    name: "".to_string(),
                    original_type: todo!("get inner type from Iterable -- special-case `Property<Vec>`"),

                    fully_qualified_types: vec![],
                    fully_qualified_type: "".to_string(),
                    pascalized_fully_qualified_type: "".to_string()
                })]));
            },
            ControlFlowRepeatPredicateDeclaration::IdentifierTuple(elem_id, index_id) => {
                ctx.scope_stack.push(HashMap::from([
                    (elem_id.to_string(),PropertyDefinition {
                        name: elem_id.to_string(),
                        original_type: "".to_string(),
                        fully_qualified_types: vec![],
                        fully_qualified_type: "".to_string(),
                        pascalized_fully_qualified_type: "".to_string()
                    }),
                    (index_id.to_string(),PropertyDefinition {
                        name: index_id.to_string(),
                        original_type: "usize".to_string(),
                        fully_qualified_types: vec![],
                        fully_qualified_type: "".to_string(),
                        pascalized_fully_qualified_type: "".to_string()
                    })
                ]));
            }
        };

        //TODO: turn compiletime stack into HashMap<String, PropertyDefinition>
        //   (allows us both to look up presence of a symbol (HashSet-like behavior) and to resolve the PropertiesCoproduct::xxx lookup and to standardize the TypesCoproduct::xxx return, required for vtable codegen)



        ctx.scope_stack.push(HashMap::from([
            ("foo".to_string(),
             PropertyDefinition {
                 name: "slot_index".to_string(),
                 original_type: "usize".to_string(),
                 fully_qualified_types: vec!["usize".to_string()],
                 fully_qualified_type: "usize".to_string(),
                 pascalized_fully_qualified_type: "__usize".to_string()
             })]
        ));
        // ctx.active_node_def.control_flow_attributes.unwrap().slot_index)

        // todo!("instead of keeping an int counter, add a compiletimestackframe");
        incremented = true;
    }

    //TODO: join settings blocks here, merge with inline_attributes
    let mut cloned_inline_attributes = ctx.active_node_def.inline_attributes.clone();
    let mut cloned_control_flow_attributes = ctx.active_node_def.control_flow_attributes.clone();
    if let Some(ref mut inline_attributes) = cloned_inline_attributes {
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
                        }).unwrap().pascalized_fully_qualified_type).clone(),
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
                        }).unwrap().pascalized_fully_qualified_type).clone(),
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
        if let Some(ref expression) = cfa.repeat_predicate_source_expression {
            let id = ctx.uid_gen.next().unwrap();

            ctx.expression_specs.insert(id, ExpressionSpec {
                id,
                pascalized_return_type: (&ctx.component_def.property_definitions.iter().find(|property_def| {
                    property_def.name == ""
                }).unwrap().pascalized_fully_qualified_type).clone(),
                invocations: vec![
                    todo!("add unique identifiers found during PAXEL parsing; include stack offset")
                    //note that each identifier may have a different stack offset value, meaning that ids must be resolved statically
                    //(requires looking up identifiers per "compiletime stack frame," e.g. components/control flow, plus error handling if symbols aren't found.)
                ],
                output_statement: "".to_string(),
                input_statement: expression.clone(),
            });
        }
    }

    std::mem::swap(&mut cloned_inline_attributes, &mut ctx.active_node_def.inline_attributes);

    for id in ctx.active_node_def.children_ids.clone().iter() {
        let mut active_node_def = ctx.template_node_definitions.remove(id).unwrap();
        ctx.active_node_def = active_node_def;

        ctx = recurse_template_and_compile_expressions(ctx);
        ctx.template_node_definitions.insert(id.to_string(), ctx.active_node_def.clone());
    };

    /* traverse template for a single component:
     [x] traverse slot, if, for, keeping track of compile-time stack
    for each found expression & expression-like (e.g. identifier binding):
     [x] write back to Manifest with unique usize id, as lookup ID for RIL component tree ge
     [ ] build lookup mechanism for symbols: "compiletime stack" + hashmaps
     [ ] handle control-flow
         [x] parsing & container structs
         [ ] special expression-binding for control flow:
             [ ] Conditional `boolean_expression`
             [ ] Repeat `data_source`
             [ ] Slot `index`
         [ ] special invocation + symbol redirection for Repeat (RepeatItem, datum_cast, i)
     [ ] Populate an ExpressionSpec, using same usize id as above for vtable entry id
         [ ] parse string PAXEL expression into RIL string with pest::PrattParser
            [ ] `.into`, `as` or `.custom_into` likely gets injected at this stage
         [ ] track unique identifiers from parsing step; use these to populate ExpressionSpecInvoations, along with compile-time stack info (offset)

     */
    if incremented {
        ctx.scope_stack.pop();
    }
    ctx
}

/// Returns (RIL string, list of invocation specs for any symbols used)
fn compile_paxel_to_ril<'a>(paxel: &str, ctx: &TemplateTraversalContext<'a>) -> (String, Vec<ExpressionSpecInvocation>) {
    todo!("");

    //1. run Pratt parser; generate output RIL
    //2. for each xo_symbol discovered during parsing, resolve that symbol through scope_stack and populate an ExpressionSpecInvocation
    //3. return tuple of (RIL string,ExpressionSpecInvocations)



}

pub struct TemplateTraversalContext<'a> {
    pub active_node_def: TemplateNodeDefinition,
    pub component_def: &'a ComponentDefinition,
    pub scope_stack: Vec<HashMap<String, PropertyDefinition>>,
    pub uid_gen: RangeFrom<usize>,
    pub expression_specs: &'a mut HashMap<usize, ExpressionSpec>,
    pub template_node_definitions: HashMap<String, TemplateNodeDefinition>,
}