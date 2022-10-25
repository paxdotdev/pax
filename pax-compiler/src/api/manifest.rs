use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::{fs, env};
use std::ops::RangeFrom;
use std::path::{Components, Path, PathBuf};
use std::rc::Rc;
use pest::iterators::{Pair, Pairs};

use uuid::Uuid;
use pest::Parser;

use serde_derive::{Serialize, Deserialize};
use serde_json;
use tera::Template;

//definition container for an entire Pax cartridge
#[derive(Serialize, Deserialize)]
pub struct PaxManifest {
    pub components: Vec<ComponentDefinition>,
    pub root_component_id: String,
    pub expression_specs: Option<HashMap<usize, ExpressionSpec>>,
    pub template_node_definitions: HashMap<String, TemplateNodeDefinition>
}

#[derive(Serialize, Deserialize)]
pub struct ExpressionSpec {
    pub id: usize,
    pub properties_type: String,
    pub pascalized_return_type: String,
    pub invocations: Vec<ExpressionSpecInvocation>,
    pub output_statement: String,
    pub input_statement: String,
}

#[derive(Serialize, Deserialize)]
pub struct ExpressionSpecInvocation {
    pub identifier: String, //for example:
    pub atomic_identifier: String, //for example `some_prop` from `self.some_prop`
    pub stack_offset: usize,
    pub properties_type: String, //e.g. PropertiesCoproduct::Foo or PropertiesCoproduct::RepeatItem
}

pub struct TemplateTraversalContext<'a> {
    active_node_def: TemplateNodeDefinition,
    component_def: &'a ComponentDefinition,
    compiletime_stack: Vec<Vec<PropertyDefinition>>,
    uid_gen: RangeFrom<usize>,
    expression_specs: &'a mut HashMap<usize, ExpressionSpec>,
    template_node_definitions: HashMap<String, TemplateNodeDefinition>,
}



impl PaxManifest {
    pub fn compile_all_expressions<'a>(&mut self) {

        let mut new_expression_specs : HashMap<usize, ExpressionSpec> = HashMap::new();
        let mut stack_offset = 0;
        let mut uid_gen = 0..;

        let mut component_id_map = HashMap::new();

        for cd in self.components.iter() {
            component_id_map.insert(&cd.source_id, &*cd);
        }

        let mut new_components = self.components.clone();
        new_components.iter_mut().for_each(|component_def : &mut ComponentDefinition|{

            let mut new_component_def = component_def.clone();
            let read_only_component_def = component_def.clone();


            if let Some(ref mut template) = new_component_def.template {
                template.iter_mut().for_each(|node_def| {
                    let mut new_node_def = node_def.clone();
                    let mut ctx = TemplateTraversalContext {
                        active_node_def: new_node_def,
                        compiletime_stack: vec![],
                        uid_gen: 0..,
                        expression_specs: &mut new_expression_specs,
                        component_def: &read_only_component_def,
                        template_node_definitions: self.template_node_definitions.clone(),
                    };


                    ctx = recurse_template_and_compile_expressions(ctx);

                    std::mem::swap(node_def, &mut ctx.active_node_def);
                    std::mem::swap(&mut self.template_node_definitions, &mut ctx.template_node_definitions);
                });
            }

            std::mem::swap(component_def, &mut new_component_def);

        });
        self.components = new_components;
        self.expression_specs = Some(new_expression_specs);

        println!("{}", serde_json::to_string_pretty(&self).unwrap());
    }




}


// Returns (RIL string, list of invocation specs for any symbols used)
fn compile_paxel_to_ril<'a>(paxel: &str, ctx: &TemplateTraversalContext<'a>) -> (String, Vec<ExpressionSpecInvocation>) {
    todo!("")
}


fn recurse_template_and_compile_expressions<'a>(mut ctx: TemplateTraversalContext<'a>) -> TemplateTraversalContext<'a> {
    let mut incremented = false;
    if ctx.active_node_def.pascal_identifier == "Slot" || ctx.active_node_def.pascal_identifier == "Repeat" || ctx.active_node_def.pascal_identifier == "Conditional" {
        // ctx.compiletime_stack.push(ctx.active_node_def)
        // ctx.compiletime_stack
        todo!("instead of keeping an int counter, add a compiletimestackframe");
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
                        properties_type: ctx.active_node_def.pascal_identifier.clone(),
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
                        properties_type: ctx.active_node_def.pascal_identifier.clone(),
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
                properties_type: ctx.active_node_def.pascal_identifier.clone(),
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
        Example use of Pratt parser, from Pest repo:
        fn parse_to_str(pairs: Pairs<Rule>, pratt: &PrattParser<Rule>) -> String {
            pratt
                .map_primary(|primary| match primary.as_rule() {
                    Rule::int => primary.as_str().to_owned(),
                    Rule::expr => parse_to_str(primary.into_inner(), pratt),
                    _ => unreachable!(),
                })
                .map_prefix(|op, rhs| match op.as_rule() {
                    Rule::neg => format!("(-{})", rhs),
                    _ => unreachable!(),
                })
                .map_postfix(|lhs, op| match op.as_rule() {
                    Rule::fac => format!("({}!)", lhs),
                    _ => unreachable!(),
                })
                .map_infix(|lhs, op, rhs| match op.as_rule() {
                    Rule::add => format!("({}+{})", lhs, rhs),
                    Rule::sub => format!("({}-{})", lhs, rhs),
                    Rule::mul => format!("({}*{})", lhs, rhs),
                    Rule::div => format!("({}/{})", lhs, rhs),
                    Rule::pow => format!("({}^{})", lhs, rhs),
                    _ => unreachable!(),
                })
                .parse(pairs)
        }


 */
    if incremented {
        ctx.compiletime_stack.pop();
    }
    ctx
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComponentDefinition {
    pub source_id: String,
    pub pascal_identifier: String,
    pub module_path: String,
    pub root_template_node_id: Option<String>,
    pub template: Option<Vec<TemplateNodeDefinition>>,
    pub settings: Option<Vec<SettingsSelectorBlockDefinition>>,
    pub property_definitions: Vec<PropertyDefinition>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
//Represents an entry within a component template, e.g. a <Rectangle> declaration inside a template
pub struct TemplateNodeDefinition {
    pub id: String,
    pub component_id: String,
    pub control_flow_attributes: Option<ControlFlowAttributeValueDefinition>,
    pub inline_attributes: Option<Vec<(String, AttributeValueDefinition)>>,
    pub children_ids: Vec<String>,
    pub pascal_identifier: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PropertyDefinition {
    /// String representation of the identifier of a declared Property
    pub name: String,
    /// Type as authored, literally.  May be partially namespace-qualified or aliased.
    pub original_type: String,
    /// Vec of constituent components of a possibly-compound type, for example `Rc<String>` breaks down into the qualified identifiers {`std::rc::Rc`, `std::string::String`}
    pub fully_qualified_types: Vec<String>,
    /// Same type as `original_type`, but dynamically normalized to be fully qualified, suitable for reexporting
    pub fully_qualified_type: String,
    /// Same as fully qualified type, but Pascalized to make a suitable enum identifier
    pub pascalized_fully_qualified_type: String,
    //pub default_value ?
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AttributeValueDefinition {
    LiteralValue(String),
    //(Expression contents, vtable id binding)
    Expression(String, Option<usize>),
    //(Expression contents, vtable id binding)
    Identifier(String, Option<usize>),
    EventBindingTarget(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ControlFlowRepeatPredicateDeclaration {
    Identifier(String),
    IdentifierTuple(String, String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ControlFlowRepeatPredicateSource {
    Identifier(String),
    IdentifierTuple(String, String),
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ControlFlowAttributeValueDefinition {
    pub condition_expression: Option<String>,
    pub slot_index: Option<String>,
    pub repeat_predicate_declaration: Option<ControlFlowRepeatPredicateDeclaration>,
    pub repeat_predicate_source_expression: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SettingsSelectorBlockDefinition {
    pub selector: String,
    pub value_block: SettingsLiteralBlockDefinition,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SettingsLiteralBlockDefinition {
    pub explicit_type_pascal_identifier: Option<String>,
    pub settings_key_value_pairs: Vec<(String, SettingsValueDefinition)>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SettingsValueDefinition {
    Literal(SettingsLiteralValue),
    Expression(String),
    Enum(String),
    Block(SettingsLiteralBlockDefinition),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SettingsLiteralValue {
    LiteralNumberWithUnit(Number, Unit),
    LiteralNumber(Number),
    LiteralArray(Vec<SettingsLiteralValue>),
    String(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Number {
    Float(f64),
    Int(isize)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Unit {
    Pixels,
    Percent
}
