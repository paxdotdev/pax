use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::{fs, env};
use std::cmp::Ordering;
use std::ops::RangeFrom;
use std::path::{Components, Path, PathBuf};
use std::rc::Rc;
use pest::iterators::{Pair, Pairs};

use uuid::Uuid;
use pest::Parser;

use serde_derive::{Serialize, Deserialize};
use serde_json;
use tera::Template;
use wasm_bindgen::UnwrapThrowExt;

/// Definition container for an entire Pax cartridge
#[derive(Serialize, Deserialize)]
pub struct PaxManifest {
    pub components: HashMap<String, ComponentDefinition>,
    pub root_component_id: String,
    pub expression_specs: Option<HashMap<usize, ExpressionSpec>>,
}


impl Eq for ExpressionSpec {}

impl PartialEq<Self> for ExpressionSpec {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialOrd<Self> for ExpressionSpec {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl Ord for ExpressionSpec {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.partial_cmp(&other.id).unwrap()
    }
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
    pub pascalized_iterable_type: Option<String>,

    /// Flag describing whether this invocation should be bound to the `elem` in `(elem, i)`
    pub is_repeat_elem: bool,

    /// Flag describing whether this invocation should be bound to the `i` in `(elem, i)`
    pub is_repeat_index: bool,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComponentDefinition {
    pub source_id: String,
    pub is_root: bool,
    pub is_primitive: bool,
    pub pascal_identifier: String,
    pub module_path: String,

    /// For primitives like Rectangle or Group, a separate import
    /// path is required for the Instance (render context) struct
    /// and the Definition struct.  For primitives, then, we need
    /// to store an additional import path to use when instantiating.
    pub primitive_instance_import_path: Option<String>,
    pub template: Option<Vec<TemplateNodeDefinition>>,
    pub settings: Option<Vec<SettingsSelectorBlockDefinition>>,
    pub property_definitions: Vec<PropertyDefinition>,
}

impl ComponentDefinition {
    pub fn get_snake_case_id(&self) -> String {
        self.source_id.replace("::", "_")
    }
}

impl ComponentDefinition {
    pub fn get_property_definition_by_name(&self, name: &str) -> PropertyDefinition {
        self.property_definitions.iter().find(|pd| { pd.name.eq(name) }).expect(&format!("Property not found with name {}", &name)).clone()
    }
}


#[derive(Serialize, Deserialize, Default, Debug, Clone)]
//Represents an entry within a component template, e.g. a <Rectangle> declaration inside a template
pub struct TemplateNodeDefinition {
    /// Component-unique int ID.  Conventionally, id 0 will be the root node for a component's template
    pub id: usize,
    /// Vec of int IDs representing the child TemplateNodeDefinitions of this TemplateNodeDefinition
    pub child_ids: Vec<usize>,
    /// Reference to the unique string ID for a component, e.g. `primitive::Frame` or `component::Stacker`
    pub component_id: String,
    /// Iff this TND is a control-flow node: parsed control flow attributes (slot/if/for)
    pub control_flow_attributes: Option<ControlFlowAttributeValueDefinition>,
    /// IFF this TND is NOT a control-flow node: parsed key-value store of attribute definitions (like `some_key="some_value"`)
    pub inline_attributes: Option<Vec<(String, AttributeValueDefinition)>>,
    /// e.g. the `SomeName` in `<SomeName some_key="some_value" />`
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
    /// If present, the type `T` in a `Property<Vec<T>>` — i.e. that which can be traversed with `for`
    pub iterable_type: Option<PropertyType>,
}

impl PropertyDefinition {
    /// Shorthand factory / constructor
    pub fn primitive_with_name(type_name: &str, symbol_name: &str) -> Self {
        PropertyDefinition {
            name: symbol_name.to_string(),
            original_type: type_name.to_string(),
            fully_qualified_constituent_types: vec![],
            property_type_info: PropertyType {
                fully_qualified_type: type_name.to_string(),
                pascalized_fully_qualified_type: type_name.to_string()
            },
            iterable_type: None
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PropertyType {
    /// Same type as `PropertyDefinition#original_type`, but dynamically normalized to be fully qualified, suitable for reexporting.  For example, the original_type `Vec<SomeStruct>` would be fully qualified as `std::vec::Vec<some_crate::SomeStruct>`
    pub fully_qualified_type: String,

    /// Same as fully qualified type, but Pascalized to make a suitable enum identifier
    pub pascalized_fully_qualified_type: String,
}

impl PropertyType {
    pub fn primitive(name: &str) -> Self {
        PropertyType {
            pascalized_fully_qualified_type: name.to_string(),
            fully_qualified_type: name.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub enum AttributeValueDefinition {
    #[default]
    Undefined, //Used for `Default`
    LiteralValue(String),
    /// (Expression contents, vtable id binding)
    Expression(String, Option<usize>),
    /// (Expression contents, vtable id binding)
    Identifier(String, Option<usize>),
    EventBindingTarget(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ControlFlowRepeatPredicateDefinition {
    ElemId(String),
    ElemIdIndexId(String, String),
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ControlFlowAttributeValueDefinition {
    pub condition_expression_paxel: Option<String>,
    pub slot_index_expression_paxel: Option<String>,
    pub repeat_predicate_definition: Option<ControlFlowRepeatPredicateDefinition>,
    pub repeat_source_definition: Option<ControlFlowRepeatSourceDefinition>
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ControlFlowRepeatSourceDefinition {
    pub range_expression: Option<String>,
    pub symbolic_binding: Option<String>,
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
