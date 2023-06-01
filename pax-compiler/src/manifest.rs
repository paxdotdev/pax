use std::collections::{HashMap};
use std::cmp::Ordering;
use std::ops::Deref;
use std::rc::Rc;

use serde_derive::{Serialize, Deserialize};
#[allow(unused_imports)]
use serde_json;

/// Definition container for an entire Pax cartridge
#[derive(Serialize, Deserialize)]
pub struct PaxManifest {
    pub components: HashMap<String, ComponentDefinition>,
    pub main_component_id: String,
    pub expression_specs: Option<HashMap<usize, ExpressionSpec>>,
    pub type_table: TypeTable,
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

/// The spec of an expression `invocation`, the necessary configuration
/// for initializing a pointer to (or copy of, in some cases) the data behind a symbol.
/// For example, if an expression uses `i`, that `i` needs to be "invoked," bound dynamically
/// to some data on the other side of `i` for the context of a particular expression.  `ExpressionSpecInvocation`
/// holds the recipe for such an `invocation`, populated as a part of expression compilation.
#[derive(Serialize, Deserialize, Clone)]
pub struct ExpressionSpecInvocation {

    /// Identifier of the top-level symbol (stripped of `this` or `self`) for nested symbols (`foo` for `foo.bar`) or the
    /// identifier itself for non-nested symbols (`foo` for `foo`)
    pub root_identifier: String,

    /// Identifier escaped so that all operations (like `.` or `[...]`) are
    /// encoded as a valid single identifier
    pub escaped_identifier: String,

    /// Statically known stack offset for traversing Repeat-based scopes at runtime
    pub stack_offset: usize,

    /// Type of the containing Properties struct, for unwrapping from PropertiesCoproduct.  For example, `Foo` for `PropertiesCoproduct::Foo` or `RepeatItem` for PropertiesCoproduct::RepeatItem
    pub properties_coproduct_type: String,

    /// For symbolic invocations that refer to repeat elements, this is the enum identifier within
    /// the TypesCoproduct that represents the appropriate `datum_cast` type
    pub pascalized_iterable_type: Option<String>,

    /// Flag describing whether this invocation should be bound to the `elem` in `(elem, i)`
    pub is_repeat_elem: bool,

    /// Flag describing whether this invocation should be bound to the `i` in `(elem, i)`
    pub is_repeat_i: bool,

    /// Flags used for particular corner cases of `Repeat` codegen
    pub is_numeric_property: bool,
    pub is_iterable_numeric: bool,
    pub is_iterable_primitive_nonnumeric: bool,

    /// Metadata used for nested symbol invocation, like `foo.bar.baz`
    /// Holds an RIL string like `.bar.get().baz.get()` for the nested
    /// symbol invocation `foo.bar.baz`.
    pub nested_symbol_literal_tail: Option<String>,

}

const SUPPORTED_NUMERIC_PRIMITIVES : [&str; 13] = [
    "u8",
    "u16",
    "u32",
    "u64",
    "u128",
    "usize",
    "i8",
    "i16",
    "i32",
    "i64",
    "i128",
    "isize",
    "f64",
];

const SUPPORTED_NONNUMERIC_PRIMITIVES : [&str; 2] = [
    "String",
    "bool",
];

impl ExpressionSpecInvocation {
    pub fn is_iterable_numeric(pascalized_iterable_type: &Option<String>) -> bool {
        if let Some(pit) = &pascalized_iterable_type {
            SUPPORTED_NUMERIC_PRIMITIVES.contains(&&**pit)
        } else {
            false
        }
    }

    pub fn is_iterable_primitive_nonnumeric(pascalized_iterable_type: &Option<String>) -> bool {
        if let Some(pit) = &pascalized_iterable_type {
            SUPPORTED_NONNUMERIC_PRIMITIVES.contains(&&**pit)
        } else {
            false
        }
    }

    pub fn is_numeric_property(property_properties_coproduct_type: &str) -> bool {
        SUPPORTED_NUMERIC_PRIMITIVES.contains(&property_properties_coproduct_type)
    }
}

/// Container for an entire component definition — includes template, settings,
/// event bindings, property definitions, and compiler + reflection metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComponentDefinition {
    pub source_id: String,
    pub is_main_component: bool,
    pub is_primitive: bool,
    pub is_type: bool,
    pub pascal_identifier: String,
    pub module_path: String,

    /// For primitives like Rectangle or Group, a separate import
    /// path is required for the Instance (render context) struct
    /// and the Definition struct.  For primitives, then, we need
    /// to store an additional import path to use when instantiating.
    pub primitive_instance_import_path: Option<String>,
    pub template: Option<Vec<TemplateNodeDefinition>>,
    pub settings: Option<Vec<SettingsSelectorBlockDefinition>>,
    pub events: Option<Vec<EventDefinition>>,

    //Properties are a concern of the `TypeDefinition` and the `PropertyDefinition`s are stored therein
    //This self_type_id can be used to retrieve a TypeDefinition from a populated TypeTable
    pub self_type_id: String,
}

impl ComponentDefinition {
    pub fn get_snake_case_id(&self) -> String {
        self.source_id
            .replace("::", "_")
            .replace("/", "_")
            .replace("\\", "_")
            .replace(">", "_")
            .replace("<", "_")
            .replace(".", "_")
    }

    pub fn get_property_definitions(&self, tt: &TypeTable) -> &Vec<PropertyDefinition> {
        &tt.get(&self.self_type_id).unwrap().property_definitions
    }
}

/// Represents an entry within a component template, e.g. a <Rectangle> declaration inside a template
/// Each node in a template is represented by exactly one `TemplateNodeDefinition`, and this is a compile-time
/// concern.  Note the difference between compile-time `definitions` and runtime `instances`.
/// A compile-time `TemplateNodeDefinition` corresponds to a single runtime `RenderNode` instance.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct TemplateNodeDefinition {
    /// Component-unique int ID.  Conventionally, id 0 will be the root node for a component's template
    pub id: usize,
    /// Vec of int IDs representing the child TemplateNodeDefinitions of this TemplateNodeDefinition
    pub child_ids: Vec<usize>,
    /// Reference to the unique string ID for a component, e.g. `primitive::Frame` or `component::Stacker`
    pub component_id: String,
    /// Iff this TND is a control-flow node: parsed control flow attributes (slot/if/for)
    pub control_flow_settings: Option<ControlFlowSettingsDefinition>,
    /// IFF this TND is NOT a control-flow node: parsed key-value store of attribute definitions (like `some_key="some_value"`)
    pub settings: Option<Vec<(String, ValueDefinition)>>,
    /// e.g. the `SomeName` in `<SomeName some_key="some_value" />`
    pub pascal_identifier: String,
}

pub type TypeTable = HashMap<String, TypeDefinition>;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PropertyDefinition {
    /// String representation of the symbolic identifier of a declared Property
    pub name: String,

    /// Flags, used ultimately by ExpressionSpecInvocations, to denote
    /// e.g. whether a property is the `i` or `elem` of a `Repeat`, which allows
    /// for special-handling the RIL that invokes these values
    pub flags: Option<PropertyDefinitionFlags>,

    /// Statically known source_id for this Property's associated TypeDefinition
    pub type_id: String,

}

impl PropertyDefinition {

    pub fn get_type_definition(&self, tt: &TypeTable) -> &TypeDefinition {
        tt.get(&self.type_id).unwrap()
    }

    pub fn get_inner_iterable_type_definition(&self, tt: &TypeTable) -> Option<&TypeDefinition> {
        if let Some(ref iiti) = tt.get(&self.type_id).unwrap().inner_iterable_type_id {
            Some(tt.get(iiti).unwrap())
        } else {
            None
        }

    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PropertyDefinitionFlags {
    pub is_repeat_i: bool,
    pub is_repeat_elem: bool,
}

/// Describes static metadata surrounding a property, for example
/// the string representation of the property's name and a `TypeInfo`
/// entry for the property's statically discovered type
impl PropertyDefinition {
    /// Shorthand factory / constructor
    pub fn primitive_with_name(type_name: &str, symbol_name: &str) -> Self {
        PropertyDefinition {
            name: symbol_name.to_string(),
            flags: None,
            type_id: type_name.to_string(),
        }
    }
}

/// Describes metadata surrounding a property's type, gathered from a combination of static & dynamic analysis
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct
TypeDefinition {
    /// Type as authored, literally.  May be partially namespace-qualified or aliased.
    pub original_type: String,

    /// Same type as `original_type`, but dynamically normalized to be fully qualified, suitable for reexporting.  For example, the original_type `Vec<SomeStruct>` would be fully qualified as `std::vec::Vec<some_crate::SomeStruct>`
    pub type_id: String,

    /// Same as fully qualified type, but Pascalized to make a suitable enum identifier
    pub type_id_pascalized: String,

    /// Vec of constituent components of a possibly-compound type, for example `Rc<String>` breaks down into the qualified identifiers {`std::rc::Rc`, `std::string::String`}
    pub fully_qualified_constituent_types: Vec<String>,

    /// Statically known source_id for this Property's iterable TypeDefinition, that is,
    /// T for some Property<Vec<T>>
    pub inner_iterable_type_id: Option<String>,

    /// A vec of PropertyType, describing known addressable (sub-)properties of this PropertyType
    pub property_definitions: Vec<PropertyDefinition>,
}

impl TypeDefinition {


    pub fn primitive(type_name: &str) -> Self {
        Self {
            original_type: type_name.to_string(),
            type_id_pascalized: type_name.to_string(),
            type_id: type_name.to_string(),
            fully_qualified_constituent_types: vec![],
            property_definitions: vec![],
            inner_iterable_type_id: None,
        }
    }

    ///Used by Repeat for source expressions, e.g. the `self.some_vec` in `for elem in self.some_vec`
    pub fn builtin_vec_rc_properties_coproduct() -> Self {
        Self {
            original_type: "Vec<Rc<PropertiesCoproduct>>".to_string(),
            type_id: "std::vec::Vec<std::rc::Rc<PropertiesCoproduct>>".to_string(),
            type_id_pascalized: "Vec_Rc_PropertiesCoproduct___".to_string(),
            fully_qualified_constituent_types: vec!["std::vec::Vec".to_string(), "std::rc::Rc".to_string()],
            property_definitions: vec![],
            inner_iterable_type_id: None,
        }
    }

    pub fn builtin_range_isize() -> Self {
        Self {
            original_type: "std::ops::Range<isize>".to_string(),
            type_id: "std::ops::Range<isize>".to_string(),
            type_id_pascalized: "Range_isize_".to_string(),
            fully_qualified_constituent_types: vec!["std::ops::Range".to_string()],
            property_definitions: vec![],
            inner_iterable_type_id: None,
        }
    }

    pub fn builtin_rc_properties_coproduct() -> Self {
        Self {
            original_type: "Rc<PropertiesCoproduct>".to_string(),
            type_id: "std::rc::Rc<PropertiesCoproduct>".to_string(),
            type_id_pascalized: "Rc_PropertiesCoproduct__".to_string(),
            fully_qualified_constituent_types: vec!["std::rc::Rc".to_string()],
            property_definitions: vec![],
            inner_iterable_type_id: None,
        }
    }

}
/// Container for settings values, storing all possible
/// variants, populated at parse-time and used at compile-time
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub enum ValueDefinition {
    #[default]
    Undefined, //Used for `Default`
    LiteralValue(String),
    Block(LiteralBlockDefinition),
    /// (Expression contents, vtable id binding)
    Expression(String, Option<usize>),
    /// (Expression contents, vtable id binding)
    Identifier(String, Option<usize>),
    EventBindingTarget(String),
}

/// Container for holding parsed data describing a Repeat (`for`)
/// predicate, for example the `(elem, i)` in `for (elem, i) in foo` or
/// the `elem` in `for elem in foo`
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ControlFlowRepeatPredicateDefinition {
    ElemId(String),
    ElemIdIndexId(String, String),
}


/// Container for storing parsed control flow information, for
/// example the string (PAXEL) representations of condition / slot / repeat
/// expressions and the related vtable ids (for "punching" during expression compilation)
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ControlFlowSettingsDefinition {
    pub condition_expression_paxel: Option<String>,
    pub condition_expression_vtable_id: Option<usize>,
    pub slot_index_expression_paxel: Option<String>,
    pub slot_index_expression_vtable_id: Option<usize>,
    pub repeat_predicate_definition: Option<ControlFlowRepeatPredicateDefinition>,
    pub repeat_source_definition: Option<ControlFlowRepeatSourceDefinition>
}

/// Container describing the possible variants of a Repeat source
/// — namely a range expression in PAXEL or a symbolic binding
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ControlFlowRepeatSourceDefinition {
    pub range_expression_paxel: Option<String>,
    pub vtable_id: Option<usize>,
    pub symbolic_binding: Option<String>,
}

/// Container for parsed Settings blocks (inside `@settings`)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SettingsSelectorBlockDefinition {
    pub selector: String,
    pub value_block: LiteralBlockDefinition,
}


/// Container for a parsed
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LiteralBlockDefinition {
    pub explicit_type_pascal_identifier: Option<String>,
    pub settings_key_value_pairs: Vec<(String, ValueDefinition)>,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EventDefinition {
    pub key: String,
    pub value: Vec<String>,
}
