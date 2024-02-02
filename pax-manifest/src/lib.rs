use std::collections::{HashMap, HashSet};
use std::hash::Hasher;
use std::{cmp::Ordering, hash::Hash};

use pax_message::serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use serde_json;

#[cfg(feature = "parsing")]
pub mod utils;

#[cfg(feature = "parsing")]
pub mod cartridge_generation;

pub static TYPE_ID_IF: &str = "IF";
pub static TYPE_ID_REPEAT: &str = "REPEAT";
pub static TYPE_ID_SLOT: &str = "SLOT";
pub static TYPE_ID_COMMENT: &str = "COMMENT";

/// Definition container for an entire Pax cartridge
#[derive(Serialize, Deserialize)]
#[serde(crate = "pax_message::serde")]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct PaxManifest {
    pub components: HashMap<String, ComponentDefinition>,
    pub main_component_type_id: String,
    pub expression_specs: Option<HashMap<usize, ExpressionSpec>>,
    pub type_table: TypeTable,
    pub import_paths: std::collections::HashSet<String>,
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
#[serde(crate = "pax_message::serde")]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct ExpressionSpec {
    /// Unique id for vtable entry — used for binding a node definition property to vtable
    pub id: usize,

    /// Representations of symbols used in an expression, and the necessary
    /// metadata to "invoke" those symbols from the runtime
    pub invocations: Vec<ExpressionSpecInvocation>,

    /// String (RIL) representation of the compiled expression
    pub output_statement: String,

    /// String representation of the original input statement
    pub input_statement: MappedString,

    /// Special-handling for Repeat codegen
    pub is_repeat_source_iterable_expression: bool,
}

/// The spec of an expression `invocation`, the necessary configuration
/// for initializing a pointer to (or copy of, in some cases) the data behind a symbol.
/// For example, if an expression uses `i`, that `i` needs to be "invoked," bound dynamically
/// to some data on the other side of `i` for the context of a particular expression.  `ExpressionSpecInvocation`
/// holds the recipe for such an `invocation`, populated as a part of expression compilation.
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "pax_message::serde")]
pub struct ExpressionSpecInvocation {
    /// Identifier of the top-level symbol (stripped of `this` or `self`) for nested symbols (`foo` for `foo.bar`) or the
    /// identifier itself for non-nested symbols (`foo` for `foo`)
    pub root_identifier: String,

    /// Identifier escaped so that all operations (like `.` or `[...]`) are
    /// encoded as a valid single identifier
    pub escaped_identifier: String,

    /// Statically known stack offset for traversing Repeat-based scopes at runtime
    pub stack_offset: usize,

    /// Type of the containing Properties struct, for downcasting from dyn Any.
    pub fully_qualified_properties_struct_type: String,

    /// For symbolic invocations that refer to repeat elements, this is the fully qualified type of each such repeated element
    pub fully_qualified_iterable_type: String,

    /// Flags used for particular corner cases of `Repeat` codegen
    pub is_numeric: bool,
    pub is_bool: bool,
    pub is_string: bool,

    /// Flags describing attributes of properties
    pub property_flags: PropertyDefinitionFlags,

    /// Metadata used for nested symbol invocation, like `foo.bar.baz`
    /// Holds an RIL "tail" string for appending to invocation literal bodies,
    /// like `.bar.get().baz.get()` for the nested symbol invocation `foo.bar.baz`.
    pub nested_symbol_tail_literal: String,
    /// Flag describing whether the nested symbolic invocation, e.g. `foo.bar`, ultimately
    /// resolves to a numeric type (as opposed to `is_numeric`, which represents the root of a nested type)
    pub is_nested_numeric: bool,
}

pub const SUPPORTED_NUMERIC_PRIMITIVES: [&str; 13] = [
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "f64",
];

pub const SUPPORTED_NONNUMERIC_PRIMITIVES: [&str; 2] = ["String", "bool"];

impl ExpressionSpecInvocation {
    pub fn is_primitive_string(property_type: &str) -> bool {
        &SUPPORTED_NONNUMERIC_PRIMITIVES[0] == &property_type
    }

    pub fn is_primitive_bool(property_type: &str) -> bool {
        &SUPPORTED_NONNUMERIC_PRIMITIVES[1] == &property_type
    }

    pub fn is_numeric(property_type: &str) -> bool {
        SUPPORTED_NUMERIC_PRIMITIVES.contains(&property_type)
    }
}

/// Container for an entire component definition — includes template, settings,
/// event bindings, property definitions, and compiler + reflection metadata
#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "pax_message::serde")]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct ComponentDefinition {
    pub type_id: String,
    pub type_id_escaped: String,
    pub is_main_component: bool,
    pub is_primitive: bool,

    /// Flag describing whether this component definition is a "struct-only component", a
    /// struct decorated with `#[pax]` for use as the `T` in `Property<T>`.
    pub is_struct_only_component: bool,

    pub pascal_identifier: String,
    pub module_path: String,

    /// For primitives like Rectangle or Group, a separate import
    /// path is required for the Instance (render context) struct
    /// and the Definition struct.  For primitives, then, we need
    /// to store an additional import path to use when instantiating.
    pub primitive_instance_import_path: Option<String>,
    pub template: Option<HashMap<usize, TemplateNodeDefinition>>,
    pub settings: Option<Vec<SettingsBlockElement>>,
    pub handlers: Option<Vec<HandlersBlockElement>>,
    pub next_template_id: Option<usize>,
    pub template_source_file_path: Option<String>,
}

impl ComponentDefinition {
    pub fn get_snake_case_id(&self) -> String {
        self.type_id
            .replace("::", "_")
            .replace("/", "_")
            .replace("\\", "_")
            .replace(">", "_")
            .replace("<", "_")
            .replace(".", "_")
    }

    pub fn get_property_definitions<'a>(&self, tt: &'a TypeTable) -> &'a Vec<PropertyDefinition> {
        &tt.get(&self.type_id).unwrap().property_definitions
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "pax_message::serde")]
pub enum SettingsBlockElement {
    SelectorBlock(Token, LiteralBlockDefinition),
    Comment(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "pax_message::serde")]
pub enum HandlersBlockElement {
    Handler(Token, Vec<Token>),
    Comment(String),
}

/// Represents an entry within a component template, e.g. a <Rectangle> declaration inside a template
/// Each node in a template is represented by exactly one `TemplateNodeDefinition`, and this is a compile-time
/// concern.  Note the difference between compile-time `definitions` and runtime `instances`.
/// A compile-time `TemplateNodeDefinition` corresponds to a single runtime `RenderNode` instance.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(crate = "pax_message::serde")]
pub struct TemplateNodeDefinition {
    /// Component-unique int ID.  Conventionally, id 0 will be the root node for a component's template
    pub id: usize,
    /// Vec of int IDs representing the child TemplateNodeDefinitions of this TemplateNodeDefinition
    pub child_ids: Vec<usize>,
    /// Reference to the unique string ID for a component, e.g. `primitive::Frame` or `component::Stacker`
    pub type_id: String,
    /// Iff this TND is a control-flow node: parsed control flow attributes (slot/if/for)
    pub control_flow_settings: Option<ControlFlowSettingsDefinition>,
    /// IFF this TND is NOT a control-flow node: parsed key-value store of attribute definitions (like `some_key="some_value"`)
    pub settings: Option<Vec<SettingElement>>,
    /// e.g. the `SomeName` in `<SomeName some_key="some_value" />`
    pub pascal_identifier: String,
    /// IFF this TND is a comment node: raw comment string
    pub raw_comment_string: Option<String>,
}

pub type TypeTable = HashMap<String, TypeDefinition>;
pub fn get_primitive_type_table() -> TypeTable {
    let mut ret: TypeTable = Default::default();

    SUPPORTED_NUMERIC_PRIMITIVES.into_iter().for_each(|snp| {
        ret.insert(snp.to_string(), TypeDefinition::primitive(snp));
    });
    SUPPORTED_NONNUMERIC_PRIMITIVES
        .into_iter()
        .for_each(|snnp| {
            ret.insert(snnp.to_string(), TypeDefinition::primitive(snnp));
        });

    ret
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(crate = "pax_message::serde")]
pub struct PropertyDefinition {
    /// String representation of the symbolic identifier of a declared Property
    pub name: String,

    /// Flags, used ultimately by ExpressionSpecInvocations, to denote
    /// e.g. whether a property is the `i` or `elem` of a `Repeat`, which allows
    /// for special-handling the RIL that invokes these values
    pub flags: PropertyDefinitionFlags,

    /// Statically known type_id for this Property's associated TypeDefinition
    pub type_id: String,

    /// Pascalized type_id, used for enum identifiers
    pub type_id_escaped: String,
}

impl PropertyDefinition {
    pub fn get_type_definition<'a>(&'a self, tt: &'a TypeTable) -> &TypeDefinition {
        if let None = tt.get(&self.type_id) {
            panic!("TypeTable does not contain type_id: {}", &self.type_id);
        }
        tt.get(&self.type_id).unwrap()
    }

    pub fn get_inner_iterable_type_definition<'a>(
        &'a self,
        tt: &'a TypeTable,
    ) -> Option<&TypeDefinition> {
        if let Some(ref iiti) = tt.get(&self.type_id).unwrap().inner_iterable_type_id {
            Some(tt.get(iiti).unwrap())
        } else {
            None
        }
    }
}

/// These flags describe the aspects of properties that affect RIL codegen.
/// Properties are divided into modal axes (exactly one value should be true per axis per struct instance)
/// Codegen considers each element of the cartesian product of these axes
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(crate = "pax_message::serde")]
pub struct PropertyDefinitionFlags {
    // // //
    // Binding axis
    //
    /// Does this property represent the index `i` in `for (elem, i)` ?
    pub is_binding_repeat_i: bool,
    /// Does this property represent `elem` in `for (elem, i)` OR `for elem in 0..5` ?
    pub is_binding_repeat_elem: bool,

    // // //
    // Source axis
    //
    /// Is the source being iterated over a Range?
    pub is_repeat_source_range: bool,
    /// Is the source being iterated over an iterable, like Vec<T>?
    pub is_repeat_source_iterable: bool,

    /// Describes whether this property is a `Property`-wrapped `T` in `Property<T>`
    /// This distinction affects our ability to dirty-watch a particular property, and
    /// has implications on codegen
    pub is_property_wrapped: bool,

    /// Describes whether this property is an enum variant property
    pub is_enum: bool,
}

/// Describes static metadata surrounding a property, for example
/// the string representation of the property's name and a `TypeInfo`
/// entry for the property's statically discovered type
impl PropertyDefinition {
    /// Shorthand factory / constructor
    pub fn primitive_with_name(type_name: &str, symbol_name: &str) -> Self {
        PropertyDefinition {
            name: symbol_name.to_string(),
            flags: PropertyDefinitionFlags::default(),
            type_id: type_name.to_string(),
            type_id_escaped: escape_identifier(type_name.to_string()),
        }
    }
}

/// Describes metadata surrounding a property's type, gathered from a combination of static & dynamic analysis
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(crate = "pax_message::serde")]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct TypeDefinition {
    /// Program-unique ID for this type
    pub type_id: String,

    /// Same as fully qualified type, but Pascalized to make a suitable enum identifier
    pub type_id_escaped: String,

    /// Unlike type_id, contains no generics data.  Simply used for qualifying / importing a type, like `std::vec::Vec`
    pub import_path: String,

    /// Statically known type_id for this Property's iterable TypeDefinition, that is,
    /// T for some Property<Vec<T>>
    pub inner_iterable_type_id: Option<String>,

    /// A vec of PropertyType, describing known addressable (sub-)properties of this PropertyType
    pub property_definitions: Vec<PropertyDefinition>,
}

impl TypeDefinition {
    pub fn primitive(type_name: &str) -> Self {
        Self {
            type_id_escaped: escape_identifier(type_name.to_string()),
            type_id: type_name.to_string(),
            property_definitions: vec![],
            inner_iterable_type_id: None,
            import_path: type_name.to_string(),
        }
    }

    ///Used by Repeat for source expressions, e.g. the `self.some_vec` in `for elem in self.some_vec`
    pub fn builtin_vec_rc_ref_cell_any_properties(inner_iterable_type_id: String) -> Self {
        let type_id = "std::vec::Vec<std::rc::Rc<core::cell::RefCell<dyn Any>>>";
        Self {
            type_id: type_id.to_string(),
            type_id_escaped: escape_identifier(type_id.to_string()),
            property_definitions: vec![],
            inner_iterable_type_id: Some(inner_iterable_type_id),
            import_path: "std::vec::Vec".to_string(),
        }
    }

    pub fn builtin_range_isize() -> Self {
        let type_id = "std::ops::Range<isize>";
        Self {
            type_id: type_id.to_string(),
            type_id_escaped: escape_identifier(type_id.to_string()),
            property_definitions: vec![],
            inner_iterable_type_id: Some("isize".to_string()),
            import_path: "std::ops::Range".to_string(),
        }
    }
}
/// Container for settings values, storing all possible
/// variants, populated at parse-time and used at compile-time
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "pax_message::serde")]
pub enum ValueDefinition {
    #[default]
    Undefined, //Used for `Default`
    LiteralValue(Token),
    Block(LiteralBlockDefinition),
    /// (Expression contents, vtable id binding)
    Expression(Token, Option<usize>),
    /// (Expression contents, vtable id binding)
    Identifier(Token, Option<usize>),
    EventBindingTarget(Token),
}

/// Container for holding metadata about original Location in Pax Template
/// Used for source-mapping
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(crate = "pax_message::serde")]
pub struct LocationInfo {
    pub start_line_col: (usize, usize),
    pub end_line_col: (usize, usize),
}

/// Container for holding parsed data describing a Repeat (`for`)
/// predicate, for example the `(elem, i)` in `for (elem, i) in foo` or
/// the `elem` in `for elem in foo`
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "pax_message::serde")]
pub enum ControlFlowRepeatPredicateDefinition {
    ElemId(Token),
    ElemIdIndexId(Token, Token),
}

/// Container for storing parsed control flow information, for
/// example the string (PAXEL) representations of condition / slot / repeat
/// expressions and the related vtable ids (for "punching" during expression compilation)
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(crate = "pax_message::serde")]
pub struct ControlFlowSettingsDefinition {
    pub condition_expression_paxel: Option<Token>,
    pub condition_expression_vtable_id: Option<usize>,
    pub slot_index_expression_paxel: Option<Token>,
    pub slot_index_expression_vtable_id: Option<usize>,
    pub repeat_predicate_definition: Option<ControlFlowRepeatPredicateDefinition>,
    pub repeat_source_definition: Option<ControlFlowRepeatSourceDefinition>,
}

/// Container describing the possible variants of a Repeat source
/// — namely a range expression in PAXEL or a symbolic binding
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(crate = "pax_message::serde")]
pub struct ControlFlowRepeatSourceDefinition {
    pub range_expression_paxel: Option<Token>,
    pub vtable_id: Option<usize>,
    pub symbolic_binding: Option<Token>,
}

/// Container for a parsed Literal object
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
#[serde(crate = "pax_message::serde")]
pub struct LiteralBlockDefinition {
    pub explicit_type_pascal_identifier: Option<Token>,
    pub elements: Vec<SettingElement>,
}

impl LiteralBlockDefinition {
    pub fn new(elements: Vec<SettingElement>) -> Self {
        Self {
            explicit_type_pascal_identifier: None,
            elements,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(crate = "pax_message::serde")]
pub enum SettingElement {
    Setting(Token, ValueDefinition),
    Comment(String),
}

impl LiteralBlockDefinition {
    pub fn get_all_settings<'a>(&'a self) -> Vec<(&'a Token, &'a ValueDefinition)> {
        self.elements
            .iter()
            .filter_map(|lbe| {
                if let SettingElement::Setting(t, vd) = lbe {
                    Some((t, vd))
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Container for parsed values with optional location information
/// Location is optional in case this token was generated dynamically
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(crate = "pax_message::serde")]
pub struct Token {
    pub token_value: String,
    // Non-pratt parsed string
    pub raw_value: String,
    pub token_type: TokenType,
    pub source_line: Option<String>,
    pub token_location: Option<LocationInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(crate = "pax_message::serde")]
pub enum TokenType {
    Expression,
    Identifier,
    LiteralValue,
    IfExpression,
    ForPredicate,
    ForSource,
    SlotExpression,
    EventId,
    Handler,
    SettingKey,
    Selector,
    PascalIdentifier,
    #[default]
    Unknown,
}

impl PartialEq for Token {
    fn eq(&self, other: &Self) -> bool {
        self.token_value == other.token_value
    }
}

impl Eq for Token {}

impl Hash for Token {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.token_value.hash(state);
    }
}

fn get_line(s: &str, line_number: usize) -> Option<&str> {
    s.lines().nth(line_number)
}

impl Token {
    pub fn new(
        token_value: String,
        token_type: TokenType,
        token_location: LocationInfo,
        pax: &str,
    ) -> Self {
        let source_line = get_line(pax, token_location.start_line_col.0).map(|s| s.to_string());
        let raw_value = token_value.clone();
        Self {
            token_value,
            raw_value,
            token_type,
            source_line,
            token_location: Some(token_location),
        }
    }

    pub fn new_only_raw(raw_value: String, token_type: TokenType) -> Self {
        Self {
            token_value: "INVALID TOKEN".to_owned(),
            raw_value,
            token_type,
            source_line: Some("INVALID SOURCE".to_owned()),
            token_location: None,
        }
    }

    pub fn new_with_raw_value(
        token_value: String,
        raw_value: String,
        token_type: TokenType,
        token_location: LocationInfo,
        pax: &str,
    ) -> Self {
        let source_line = get_line(pax, token_location.start_line_col.0).map(|s| s.to_string());
        Self {
            token_value,
            raw_value,
            token_type,
            source_line,
            token_location: Some(token_location),
        }
    }

    pub fn new_from_raw_value(raw_value: String, token_type: TokenType) -> Self {
        Self {
            token_value: raw_value.clone(),
            raw_value,
            token_type,
            source_line: None,
            token_location: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "pax_message::serde")]
pub enum Number {
    Float(f64),
    Int(isize),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "pax_message::serde")]
pub enum Unit {
    Pixels,
    Percent,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(crate = "pax_message::serde")]
pub struct MappedString {
    pub content: String,
    /// Markers used to identify generated code range for source map.
    pub source_map_start_marker: Option<String>,
    pub source_map_end_marker: Option<String>,
}

impl PartialEq for MappedString {
    fn eq(&self, other: &Self) -> bool {
        self.content == other.content
    }
}

impl Eq for MappedString {}

impl Hash for MappedString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.content.hash(state);
    }
}

impl MappedString {
    pub fn none() -> Self {
        MappedString {
            content: "None".to_string(),
            source_map_start_marker: None,
            source_map_end_marker: None,
        }
    }

    pub fn new(content: String) -> Self {
        MappedString {
            content,
            source_map_start_marker: None,
            source_map_end_marker: None,
        }
    }
}

pub fn escape_identifier(input: String) -> String {
    input
        .replace("(", "LPAR")
        .replace("::", "COCO")
        .replace(")", "RPAR")
        .replace("<", "LABR")
        .replace(">", "RABR")
        .replace(",", "COMM")
        .replace(".", "PERI")
        .replace("[", "LSQB")
        .replace("]", "RSQB")
        .replace("/", "FSLA")
        .replace("\\", "BSLA")
        .replace("#", "HASH")
        .replace("-", "HYPH")
}

/// Pulled from host Cargo.toml
pub struct HostCrateInfo {
    /// for example: `pax-example`
    pub name: String,
    /// for example: `pax_example`
    pub identifier: String,
    /// for example: `some_crate::pax_reexports`,
    pub import_prefix: String,
}

pub const IMPORTS_BUILTINS: [&str; 28] = [
    "std::any::Any",
    "std::cell::RefCell",
    "std::collections::HashMap",
    "std::collections::VecDeque",
    "std::ops::Deref",
    "std::rc::Rc",
    "pax_core::RepeatItem",
    "pax_core::RepeatProperties",
    "pax_core::ConditionalProperties",
    "pax_core::SlotProperties",
    "pax_core::get_numeric_from_wrapped_properties",
    "pax_runtime_api::PropertyInstance",
    "pax_runtime_api::PropertyLiteral",
    "pax_runtime_api::CommonProperties",
    "pax_core::ComponentInstance",
    "pax_core::InstanceNodePtr",
    "pax_core::PropertyExpression",
    "pax_core::InstanceNodePtrList",
    "pax_core::ExpressionContext",
    "pax_core::PaxEngine",
    "pax_core::InstanceNode",
    "pax_core::HandlerRegistry",
    "pax_core::InstantiationArgs",
    "pax_core::ConditionalInstance",
    "pax_core::SlotInstance",
    "pax_core::properties::RuntimePropertiesStackFrame",
    "pax_core::repeat::RepeatInstance",
    "piet_common::RenderContext",
];

impl<'a> HostCrateInfo {
    pub fn fully_qualify_path(&self, path: &str) -> String {
        if path.contains("pax_reexports") {
            return path.replace("crate::", "").to_string();
        }
        #[allow(non_snake_case)]
        let IMPORT_PREFIX = format!("{}::pax_reexports::", self.identifier);
        let imports_builtins_set: HashSet<&str> = IMPORTS_BUILTINS.into_iter().collect();
        let mut primitives_set: HashSet<&str> = SUPPORTED_NUMERIC_PRIMITIVES
            .into_iter()
            .chain(SUPPORTED_NONNUMERIC_PRIMITIVES.into_iter())
            .collect();
        primitives_set.insert(TYPE_ID_IF);
        primitives_set.insert(TYPE_ID_REPEAT);
        primitives_set.insert(TYPE_ID_SLOT);
        primitives_set.insert(TYPE_ID_COMMENT);
        if primitives_set.contains(path) {
            path.to_string()
        } else if !imports_builtins_set.contains(path) {
            IMPORT_PREFIX.clone() + &path.replace("crate::", "")
        } else {
            "".to_string()
        }
    }
}
