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
    pub components: HashMap<String, ComponentDefinition>,
    pub root_component_id: String,
    pub expression_specs: Option<HashMap<usize, ExpressionSpec>>,
    pub template_node_definitions: HashMap<String, TemplateNodeDefinition>
}

impl PaxManifest {

}

#[derive(Serialize, Deserialize, Clone)]
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

}

#[derive(Serialize, Deserialize, Clone)]
pub struct  ExpressionSpecInvocation {
    /// Identifier as authored, for example: `self.some_prop`
    pub identifier: String,

    /// Identifier escaped so that all operations (like `.` or `[...]`) are
    /// encoded as a valid single identifier — e.g. `self.foo` => `self__
    pub escaped_identifier: String,

    /// Statically known stack offset for traversing Repeat-based scopes at runtime
    pub stack_offset: usize,
    /// Type of the containing Properties struct, for unwrapping from PropertiesCoproduct.  For example, `Foo` for `PropertiesCoproduct::Foo` or `RepeatItem` for PropertiesCoproduct::RepeatItem
    pub properties_type: String,

    /// For invocations that reference repeat elements, this is the enum identifier within
    /// the TypesCoproduct that represents the appropriate `datum_cast` type
    pub pascalized_datum_cast_type: Option<String>,

    /// Flag describing whether this invocation should be bound to the `elem` in `(elem, i)`
    pub is_repeat_elem: bool,
    /// Flag describing whether this invocation should be bound to the `i` in `(elem, i)`
    pub is_repeat_index: bool,
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
    pub addressable_properties: Option<Vec<PropertyDefinition>>,
    pub children_ids: Vec<String>,
    pub pascal_identifier: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PropertyDefinition {
    /// String representation of the identifier of a declared Property
    pub name: String,
    /// Type as authored, literally.  May be partially namespace-qualified or aliased.
    pub original_type: String,
    /// Vec of constituent components of a possibly-compound type, for example `Rc<String>` breaks down into the qualified identifiers {`std::rc::Rc`, `std::string::String`}
    pub fully_qualified_constituent_types: Vec<String>,
    /// Store of fully qualified types that may be needed for expression vtable generation
    pub property_type_info: PropertyType,

    pub datum_cast_type: Option<PropertyType>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PropertyType {
    /// Same type as `PropertyDefinition#original_type`, but dynamically normalized to be fully qualified, suitable for reexporting.  For example, the original_type `Vec<SomeStruct>` would be fully qualified as `std::vec::Vec<some_crate::SomeStruct>`
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
    ElemId(String),
    ElemIdIndexId(String, String),
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
    pub repeat_source_definition: RepeatSourceDefinition,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RepeatSourceDefinition {
    pub range_expression: Option<String>,
    pub symbolic_binding: Option<String>,
    pub elem_type: PropertyDefinition,
}

// Instead of mapping out Repeat Source ontology, we can
// resolve any Source as an expression
//
// #[derive(Serialize, Deserialize, Debug, Clone, Default)]
// pub struct RepeatSourceRangeDefinition {
//     start: RepeatSourceRangeBoundary,
//     end: RepeatSourceRangeBoundary,
//     operator: RepeatSourceRangeOperator,
// }
//
// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub enum RepeatSourceRangeBoundary {
//     SymbolicIdentifier(String),
//     IntegerLiteral(usize),
// }
//
// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub enum RepeatSourceRangeOperator {
//     Inclusive,
//     Exclusive,
// }

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
