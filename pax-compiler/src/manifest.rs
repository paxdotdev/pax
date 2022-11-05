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
    /// Unique id for vtable entry — used for binding a node definition property to vtable
    pub id: usize,

    /// Used to wrap the return type in TypesCoproduct
    pub pascalized_return_type: String,

    /// Representations of symbols used in an expression, and the necessary
    /// metadata to "invoke" those symbols from the runtime
    pub invocations: Vec<ExpressionSpecInvocation>,

    /// String (RIL) representation of the compiled expression
    pub output_statement: String,

    /// String representation of the original input statement
    pub input_statement: String,

    // Note: provisionally removed because this data is
    // For data structures that Repeat can iterate over (starting with std::vec::Vec<T>),
    // this field stores a string representation of the iterable type `T`.  Note that
    // this type must be available in the PropertiesCoproduct, which can be achieved
    // by using a built-in primitive type, or by annotating a custom type with the `pax_type` macro.
    // pub iter_type: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ExpressionSpecInvocation {
    /// Identifier as authored, for example: `self.some_prop`
    pub identifier: String,
    /// Representation of the symbol to be invoked: for example `some_prop` from `self.some_prop`
    pub atomic_identifier: String,
    /// Statically known stack offset for traversing Repeat-based scopes at runtime
    pub stack_offset: usize,
    /// Type of the containing Properties struct, for unwrapping from PropertiesCoproduct.  For example, `Foo` for `PropertiesCoproduct::Foo` or `RepeatItem` for PropertiesCoproduct::RepeatItem
    pub properties_type: String,
    /// Flag describing whether this invocation should be bound to the `elem` in `(elem, i)`
    pub is_repeat_elem: bool,
    /// Flag describing whether this invocation should be bound to the `i` in `(elem, i)`
    pub is_repeat_index: bool,
}



// fn recurse_pratt_parse_to_string(pairs: Pairs<Rule>, pratt: &PrattParser<Rule>) -> String {
//     pratt
//         .map_primary(|primary| match primary.as_rule() {
//             Rule::int => primary.as_str().to_owned(),
//             Rule::expr => parse_to_str(primary.into_inner(), pratt),
//             _ => unreachable!(),
//         })
//         .map_prefix(|op, rhs| match op.as_rule() {
//             Rule::neg => format!("(-{})", rhs),
//             _ => unreachable!(),
//         })
//         .map_postfix(|lhs, op| match op.as_rule() {
//             Rule::fac => format!("({}!)", lhs),
//             _ => unreachable!(),
//         })
//         .map_infix(|lhs, op, rhs| match op.as_rule() {
//             Rule::add => format!("({}+{})", lhs, rhs),
//             Rule::sub => format!("({}-{})", lhs, rhs),
//             Rule::mul => format!("({}*{})", lhs, rhs),
//             Rule::div => format!("({}/{})", lhs, rhs),
//             Rule::pow => format!("({}^{})", lhs, rhs),
//             _ => unreachable!(),
//         })
//         .parse(pairs)
// }



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
    ///(Element ID, Index ID)
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
