
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::{fs, env};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use pest::iterators::{Pair, Pairs};

use uuid::Uuid;
use pest::Parser;

use serde_derive::{Serialize, Deserialize};
use serde_json;


//definition container for an entire Pax cartridge
#[derive(Serialize, Deserialize)]
pub struct PaxManifest {
    pub components: Vec<ComponentDefinition>,
    pub root_component_id: String,
    pub expression_specs: Option<HashMap<usize, ExpressionSpec>>,
}


#[derive(Serialize, Deserialize)]
pub struct ExpressionSpec {
    pub id: u32,
    pub properties_type: String,
    pub return_type: String,
    pub invocations: String,
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

impl PaxManifest {
    pub fn compile_all_expressions(&mut self) {
        //traverse each component definition; keep track of "compile-time stack"
        //for each property that is an Expression, build a spec:
        //  - transpile expression to Rust (PrecClimb ?)
        //  - populate ExpressionSpecInvocation
        //  - handle usize ids

        let new_expression_specs : HashMap<usize, ExpressionSpec> = HashMap::new();

        let mut stack_offset = 0;

        let recurse_template_fn = |node_def|{


            //traverse template, but no need to recurse into other component defs
            // - yes need to traverse slot, if, for, keeping track of compile-time stack
            //for each found expression & expression-like (e.g. identifier binding):
            // - write back to Manifest with unique usize id, as lookup ID for RIL component tree gen
            // - use same usize id to populate an ExpressionSpec, for entry into vtable as ID
            // - parse RIL string expression with pest::PrattParser
            // - track unique identifiers from parsing step; use these to populate ExpressionSpecInvoations, along with compile-time stack info
            /* Example use of Pratt parser, from Pest repo:
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
        };
        self.components.iter_mut().for_each(|component_def|{
            component_def.template.as_ref().unwrap().iter().for_each(recurse_template_fn);
        });

        self.expression_specs = Some(new_expression_specs);
    }

}



#[derive(Serialize, Deserialize, Debug)]
pub struct ComponentDefinition {
    pub source_id: String,
    pub pascal_identifier: String,
    pub module_path: String,
    //optional not because it cannot exist, but because
    //there are times in this data structure's lifecycle when it
    //is not yet known
    pub root_template_node_id: Option<String>,
    pub template: Option<Vec<TemplateNodeDefinition>>,
    //can be hydrated as a tree via child_ids/parent_id
    pub settings: Option<Vec<SettingsSelectorBlockDefinition>>,
    pub property_definitions: Vec<PropertyDefinition>,
}

#[derive(Serialize, Deserialize, Debug)]
//Represents an entry within a component template, e.g. a <Rectangle> declaration inside a template
pub struct TemplateNodeDefinition {
    pub id: String,
    pub component_id: String,
    pub inline_attributes: Option<Vec<(String, AttributeValueDefinition)>>,
    pub children_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PropertyDefinition {
    /// String representation of the identifier of a declared Property
    pub name: String,
    /// Type as authored, literally.  May be partially namespace-qualified or aliased.
    pub original_type: String,
    /// Vec of constituent components of a type, for example `Rc<String>` would have the dependencies [`std::rc::Rc` and `std::string::String`]
    pub fully_qualified_dependencies: Vec<String>,
    /// Same type as `original_type`, but dynamically normalized to be fully qualified, suitable for reexporting
    pub fully_qualified_type: String,

    /// Same as fully qualified type, but Pascalized to make a suitable enum identifier
    pub pascalized_fully_qualified_type: String,
    //pub default_value ?
}

#[derive(Serialize, Deserialize, Debug)]
pub enum AttributeValueDefinition {
    LiteralValue(String),
    Expression(String),
    Identifier(String),
    EventBindingTarget(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SettingsSelectorBlockDefinition {
    pub selector: String,
    pub value_block: SettingsLiteralBlockDefinition,
    //TODO: think through this recursive data structure and de/serialization.
    //      might need to normalize it, keeping a tree of `SettingsLiteralBlockDefinition`s
    //      where nodes are flattened into a list.
    //     First: DO we need to normalize it?  Will something like Serde magically fix this?
    //     It's possible that it will.  Revisit only if we have trouble serializing this data.
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SettingsLiteralBlockDefinition {
    pub explicit_type_pascal_identifier: Option<String>,
    pub settings_key_value_pairs: Vec<(String, SettingsValueDefinition)>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum SettingsValueDefinition {
    Literal(SettingsLiteralValue),
    Expression(String),
    Enum(String),
    Block(SettingsLiteralBlockDefinition),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum SettingsLiteralValue {
    LiteralNumberWithUnit(Number, Unit),
    LiteralNumber(Number),
    LiteralArray(Vec<SettingsLiteralValue>),
    String(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Number {
    Float(f64),
    Int(isize)
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Unit {
    Pixels,
    Percent
}
