use std::borrow::Borrow;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Display;
use std::hash::Hasher;
use std::{cmp::Ordering, hash::Hash};

use constants::{TYPE_ID_COMMENT, TYPE_ID_IF, TYPE_ID_REPEAT, TYPE_ID_SLOT};
use pax_message::serde::{Deserialize, Serialize};

#[cfg(feature = "parsing")]
pub mod utils;

pub mod cartridge_generation;
pub mod constants;

#[cfg(feature = "parsing")]
pub mod deserializer;

/// Definition container for an entire Pax cartridge
#[serde_with::serde_as]
#[derive(Serialize, Deserialize)]
#[serde(crate = "pax_message::serde")]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct PaxManifest {
    #[serde_as(as = "HashMap<serde_with::json::JsonString, _>")]
    pub components: HashMap<TypeId, ComponentDefinition>,
    pub main_component_type_id: TypeId,
    pub expression_specs: Option<HashMap<usize, ExpressionSpec>>,
    #[serde_as(as = "HashMap<serde_with::json::JsonString, _>")]
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
    pub fn is_primitive_string(property_type: &TypeId) -> bool {
        if let PaxType::Singleton { pascal_identifier } = &property_type.pax_type {
            return &SUPPORTED_NONNUMERIC_PRIMITIVES[0] == &pascal_identifier;
        }
        false
    }

    pub fn is_primitive_bool(property_type: &TypeId) -> bool {
        if let PaxType::Singleton { pascal_identifier } = &property_type.pax_type {
            return &SUPPORTED_NONNUMERIC_PRIMITIVES[1] == &pascal_identifier;
        }
        false
    }

    pub fn is_numeric(property_type: &TypeId) -> bool {
        if let PaxType::Singleton { pascal_identifier } = &property_type.pax_type {
            return SUPPORTED_NUMERIC_PRIMITIVES.contains(&pascal_identifier.as_str());
        }
        false
    }
}

/// Container for an entire component definition — includes template, settings,
/// event bindings, property definitions, and compiler + reflection metadata
#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "pax_message::serde")]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct ComponentDefinition {
    pub type_id: TypeId,
    pub is_main_component: bool,
    pub is_primitive: bool,

    /// Flag describing whether this component definition is a "struct-only component", a
    /// struct decorated with `#[pax]` for use as the `T` in `Property<T>`.
    pub is_struct_only_component: bool,

    pub module_path: String,

    /// For primitives like Rectangle or Group, a separate import
    /// path is required for the Instance (render context) struct
    /// and the Definition struct.  For primitives, then, we need
    /// to store an additional import path to use when instantiating.
    pub primitive_instance_import_path: Option<String>,
    pub template: Option<ComponentTemplate>,
    pub settings: Option<Vec<SettingsBlockElement>>,
    pub handlers: Option<Vec<HandlerBindingElement>>,
}

impl ComponentDefinition {
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
pub enum HandlerBindingElement {
    Handler(Token, Vec<Token>),
    Comment(String),
}

#[derive(Serialize, Default, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
#[serde(crate = "pax_message::serde")]
pub struct TemplateNodeId(usize);

impl TemplateNodeId {
    pub fn build(id: usize) -> Self {
        TemplateNodeId(id)
    }
}

impl Display for TemplateNodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Serialize, Default, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
#[serde(crate = "pax_message::serde")]
pub enum PaxType {
    If,
    Slot,
    Repeat,
    Comment,
    Primitive {
        pascal_identifier: String,
    },
    Singleton {
        pascal_identifier: String,
    },
    Range {
        identifier: String,
    },
    Option {
        identifier: String,
    },
    Vector {
        elem_identifier: String,
    },
    Map {
        key_identifier: String,
        value_identifier: String,
    },
    #[default]
    Unknown,
}

#[derive(Serialize, Default, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
#[serde(crate = "pax_message::serde")]
pub struct TypeId {
    pax_type: PaxType,
    import_path: Option<String>,

    _type_id: String,
    _type_id_escaped: String,
}

impl Display for TypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_unique_identifier())
    }
}

impl TypeId {
    pub fn build_if() -> Self {
        TypeId {
            pax_type: PaxType::If,
            import_path: None,
            _type_id: "If".to_string(),
            _type_id_escaped: "".to_string(),
        }
    }

    pub fn build_repeat() -> Self {
        TypeId {
            pax_type: PaxType::Repeat,
            import_path: None,
            _type_id: "Repeat".to_string(),
            _type_id_escaped: "".to_string(),
        }
    }

    pub fn build_slot() -> Self {
        TypeId {
            pax_type: PaxType::Slot,
            import_path: None,
            _type_id: "Slot".to_string(),
            _type_id_escaped: "".to_string(),
        }
    }

    pub fn build_comment() -> Self {
        TypeId {
            pax_type: PaxType::Comment,
            import_path: None,
            _type_id: "Comment".to_string(),
            _type_id_escaped: "".to_string(),
        }
    }

    pub fn build_singleton(import_path: String, pascal_identifier: Option<String>) -> Self {
        let pascal_identifier = if let Some(p) = pascal_identifier {
            p
        } else {
            import_path.split("::").last().unwrap().to_string()
        };

        Self {
            pax_type: PaxType::Singleton { pascal_identifier },
            import_path: Some(import_path.clone()),
            _type_id: import_path.clone(),
            _type_id_escaped: escape_identifier(import_path.clone()),
        }
    }

    pub fn build_primitive(identifier: String) -> Self {
        TypeId {
            pax_type: PaxType::Primitive {
                pascal_identifier: identifier.clone(),
            },
            import_path: None,
            _type_id: identifier.clone(),
            _type_id_escaped: identifier.clone(),
        }
    }

    pub fn build_vector(elem_identifier: String) -> Self {
        let _id = format!("std::vec::Vec<{}>", elem_identifier);
        Self {
            pax_type: PaxType::Vector { elem_identifier },
            import_path: Some("std::vec::Vec".to_string()),
            _type_id: _id.clone(),
            _type_id_escaped: escape_identifier(_id),
        }
    }

    pub fn build_range(identifier: String) -> Self {
        let _id = format!("std::ops::Range<{}>", identifier);
        Self {
            pax_type: PaxType::Range { identifier },
            import_path: Some("std::ops::Range".to_string()),
            _type_id: _id.clone(),
            _type_id_escaped: escape_identifier(_id),
        }
    }

    pub fn build_option(identifier: String) -> Self {
        let _id = format!("std::option::Option<{}>", identifier);
        Self {
            pax_type: PaxType::Option { identifier },
            import_path: Some("std::option::Option".to_string()),
            _type_id: _id.clone(),
            _type_id_escaped: escape_identifier(_id),
        }
    }

    pub fn build_map(key_identifier: String, value_identifier: String) -> Self {
        let _id = format!("std::collections::HashMap<{}><{}>", key_identifier.clone(), value_identifier.clone());
        Self {
            pax_type: PaxType::Map {
                key_identifier,
                value_identifier,
            },
            import_path: Some("std::collections::HashMap".to_string()),
            _type_id: _id.clone(),
            _type_id_escaped: escape_identifier(_id),
        }
    }

    pub fn get_import_path(&self) -> Option<String> {
        if let PaxType::Primitive { pascal_identifier } = &self.pax_type {
            return Some(pascal_identifier.clone());
        }
        self.import_path.clone()
    }

    pub fn get_pascal_identifier(&self) -> Option<String> {
        match &self.pax_type {
            PaxType::Primitive { pascal_identifier } | 
            PaxType::Singleton { pascal_identifier } =>  Some(pascal_identifier.clone()),
            _ => None,
        }
    }

    pub fn get_unique_identifier(&self) -> String {
        self._type_id.clone()
    }

    pub fn get_pax_type(&self) -> &PaxType {
        self.pax_type.borrow()
    }

    pub fn get_snake_case_id(&self) -> String {
        self.get_unique_identifier()
            .replace("::", "_")
            .replace("/", "_")
            .replace("\\", "_")
            .replace(">", "_")
            .replace("<", "_")
            .replace(".", "_")
    }
    

    pub fn fully_qualify_id(host_crate_info: &HostCrateInfo, id: String) -> Option<String> {
        let mut primitives_set: HashSet<&str> = SUPPORTED_NUMERIC_PRIMITIVES
        .into_iter()
        .chain(SUPPORTED_NONNUMERIC_PRIMITIVES.into_iter())
        .collect();
        primitives_set.insert(TYPE_ID_IF);
        primitives_set.insert(TYPE_ID_REPEAT);
        primitives_set.insert(TYPE_ID_SLOT);
        primitives_set.insert(TYPE_ID_COMMENT);

        let ret = id.replace("crate::", "").to_string();
        #[allow(non_snake_case)]
        let IMPORT_PREFIX = format!("{}::pax_reexports::", host_crate_info.identifier);
        let imports_builtins_set: HashSet<&str> = IMPORTS_BUILTINS.into_iter().collect();
            
        if primitives_set.contains(id.as_str()) || id.contains("pax_reexports") {
            Some(ret.to_string())
        } else if !imports_builtins_set.contains(id.as_str()) {
            if id.contains("{PREFIX}") {
                Some(ret.replace("{PREFIX}", &IMPORT_PREFIX))
            } else {
                Some(IMPORT_PREFIX.clone() + ret.as_str())
            }
        } else {
            None
        }
    }

    pub fn fully_qualify_type_id(&mut self, host_crate_info: &HostCrateInfo) -> &Self {
       
        if let Some(path) = self.get_import_path() {
            self.import_path = Self::fully_qualify_id(host_crate_info, path);
        }
        if let Some(id) =  Self::fully_qualify_id(host_crate_info, self._type_id.clone()){
            self._type_id =id;
            self._type_id_escaped = self._type_id_escaped.replace("{PREFIX}", "");
        }
        self
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
#[serde(crate = "pax_message::serde")]
pub struct UniqueTemplateNodeIdentifier {
    component: TypeId,
    template_node_id: TemplateNodeId,
}

impl UniqueTemplateNodeIdentifier {
    pub fn build(component: TypeId, template_node_id: TemplateNodeId) -> Self {
        UniqueTemplateNodeIdentifier {
            component,
            template_node_id,
        }
    }

    pub fn get_type_id(self) -> TypeId {
        self.component
    }

    pub fn get_template_node_id(self) -> TemplateNodeId {
        self.template_node_id
    }
}

impl Display for UniqueTemplateNodeIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.component, self.template_node_id)
    }
}

pub enum TemplateLocation {
    Root,
    Parent(TemplateNodeId),
}

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(crate = "pax_message::serde")]
pub struct ComponentTemplate {
    containing_component: TypeId,
    root: VecDeque<TemplateNodeId>,
    #[serde_as(as = "HashMap<serde_with::json::JsonString, _>")]
    children: HashMap<TemplateNodeId, VecDeque<TemplateNodeId>>,
    #[serde_as(as = "HashMap<serde_with::json::JsonString, _>")]
    nodes: HashMap<TemplateNodeId, TemplateNodeDefinition>,
    next_id: usize,
    template_source_file_path: Option<String>,
}

impl ComponentTemplate {
    pub fn new(containing_component: TypeId, template_source_file_path: Option<String>) -> Self {
        Self {
            containing_component,
            root: VecDeque::new(),
            children: HashMap::new(),
            nodes: HashMap::new(),
            next_id: 0,
            template_source_file_path,
        }
    }

    pub fn get_next_id(&self) -> usize {
        self.next_id
    }

    pub fn set_next_id(&mut self, id: usize) {
        self.next_id = id;
    }

    pub fn get_unique_identifier(&self, id: TemplateNodeId) -> UniqueTemplateNodeIdentifier {
        let type_id = self.containing_component.clone();
        UniqueTemplateNodeIdentifier::build(type_id, id)
    }

    fn consume_next_id(&mut self) -> TemplateNodeId {
        let current_next_id = self.next_id;
        self.next_id = self.next_id + 1;
        TemplateNodeId::build(current_next_id)
    }

    pub fn add_root_node_front(
        &mut self,
        tnd: TemplateNodeDefinition,
    ) -> UniqueTemplateNodeIdentifier {
        let id = self.consume_next_id();
        self.root.push_front(id.clone());
        self.nodes.insert(id.clone(), tnd);
        self.get_unique_identifier(id)
    }

    pub fn add_root_node_back(
        &mut self,
        tnd: TemplateNodeDefinition,
    ) -> UniqueTemplateNodeIdentifier {
        let id = self.consume_next_id();
        self.root.push_back(id.clone());
        self.nodes.insert(id.clone(), tnd);
        self.get_unique_identifier(id)
    }

    pub fn add_root_node_at(
        &mut self,
        index: usize,
        tnd: TemplateNodeDefinition,
    ) -> UniqueTemplateNodeIdentifier {
        let id = self.consume_next_id();
        self.root.insert(index, id.clone());
        self.nodes.insert(id.clone(), tnd);
        self.get_unique_identifier(id)
    }

    pub fn add(&mut self, tnd: TemplateNodeDefinition) -> UniqueTemplateNodeIdentifier {
        self.add_root_node_front(tnd)
    }

    pub fn add_child_front(
        &mut self,
        id: TemplateNodeId,
        tnd: TemplateNodeDefinition,
    ) -> UniqueTemplateNodeIdentifier {
        if let Some(_) = self.nodes.get_mut(&id) {
            let child_id = self.consume_next_id();
            if let Some(children) = self.children.get_mut(&id) {
                children.push_front(child_id.clone());
                self.nodes.insert(child_id.clone(), tnd);
            } else {
                let mut children = VecDeque::new();
                children.push_front(child_id.clone());
                self.nodes.insert(child_id.clone(), tnd);
                self.children.insert(id, children);
            }
            self.get_unique_identifier(child_id)
        } else {
            panic!("Invalid parent");
        }
    }

    pub fn add_child_back(
        &mut self,
        id: TemplateNodeId,
        tnd: TemplateNodeDefinition,
    ) -> UniqueTemplateNodeIdentifier {
        if let Some(_) = self.nodes.get_mut(&id) {
            let child_id = self.consume_next_id();
            if let Some(children) = self.children.get_mut(&id) {
                children.push_back(child_id.clone());
                self.nodes.insert(child_id.clone(), tnd);
            } else {
                let mut children = VecDeque::new();
                children.push_back(child_id.clone());
                self.nodes.insert(child_id.clone(), tnd);
                self.children.insert(id, children);
            }
            self.get_unique_identifier(child_id)
        } else {
            panic!("Invalid parent");
        }
    }

    pub fn add_child_at(
        &mut self,
        id: TemplateNodeId,
        index: usize,
        tnd: TemplateNodeDefinition,
    ) -> UniqueTemplateNodeIdentifier {
        if let Some(_) = self.nodes.get_mut(&id) {
            let child_id = self.consume_next_id();
            if let Some(children) = self.children.get_mut(&id) {
                children.insert(index, child_id.clone());
                self.nodes.insert(child_id.clone(), tnd);
            } else {
                let mut children = VecDeque::new();
                children.insert(index, child_id.clone());
                self.nodes.insert(child_id.clone(), tnd);
                self.children.insert(id, children);
            }
            self.get_unique_identifier(child_id)
        } else {
            panic!("Invalid parent");
        }
    }

    pub fn add_child(
        &mut self,
        id: TemplateNodeId,
        tnd: TemplateNodeDefinition,
    ) -> UniqueTemplateNodeIdentifier {
        self.add_child_front(id, tnd)
    }

    pub fn remove_node(&mut self, id: TemplateNodeId) -> TemplateNodeDefinition {
        if let Some(tnd) = self.nodes.get(&id) {
            let node = tnd.clone();
            self.children.remove(&id);
            for (_, children) in self.children.iter_mut() {
                children.retain(|child| *child != id);
            }
            self.root.retain(|root_node| *root_node != id);
            node
        } else {
            panic!("Requested node doesn't exist in template");
        }
    }

    pub fn get_root(&self) -> Vec<TemplateNodeId> {
        self.root.clone().into()
    }

    pub fn get_children(&self, id: TemplateNodeId) -> Option<Vec<TemplateNodeId>> {
        if let Some(c) = self.children.get(&id) {
            return Some(c.clone().into());
        }
        None
    }

    pub fn get_node(&self, id: &TemplateNodeId) -> Option<&TemplateNodeDefinition> {
        self.nodes.get(id)
    }

    pub fn get_nodes(&self) -> Vec<&TemplateNodeDefinition> {
        self.nodes.values().collect()
    }

    pub fn get_ids(&self) -> Vec<&TemplateNodeId> {
        self.nodes.keys().collect()
    }

    pub fn fully_qualify_template_type_ids(&mut self, host_crate_info: &HostCrateInfo) {
        self.containing_component
            .fully_qualify_type_id(host_crate_info);
        for (_, val) in self.nodes.iter_mut() {
            val.type_id.fully_qualify_type_id(&host_crate_info);
        }
    }
}

/// Represents an entry within a component template, e.g. a <Rectangle> declaration inside a template
/// Each node in a template is represented by exactly one `TemplateNodeDefinition`, and this is a compile-time
/// concern.  Note the difference between compile-time `definitions` and runtime `instances`.
/// A compile-time `TemplateNodeDefinition` corresponds to a single runtime `RenderNode` instance.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(crate = "pax_message::serde")]
pub struct TemplateNodeDefinition {
    /// Reference to the unique string ID for a component, e.g. `primitive::Frame` or `component::Stacker`
    pub type_id: TypeId,
    /// Iff this TND is a control-flow node: parsed control flow attributes (slot/if/for)
    pub control_flow_settings: Option<ControlFlowSettingsDefinition>,
    /// IFF this TND is NOT a control-flow node: parsed key-value store of attribute definitions (like `some_key="some_value"`)
    pub settings: Option<Vec<SettingElement>>,
    /// IFF this TND is a comment node: raw comment string
    pub raw_comment_string: Option<String>,
}

pub type TypeTable = HashMap<TypeId, TypeDefinition>;
pub fn get_primitive_type_table() -> TypeTable {
    let mut ret: TypeTable = Default::default();

    SUPPORTED_NUMERIC_PRIMITIVES.into_iter().for_each(|snp| {
        ret.insert(
            TypeId::build_primitive(snp.to_string()),
            TypeDefinition::primitive(snp),
        );
    });
    SUPPORTED_NONNUMERIC_PRIMITIVES
        .into_iter()
        .for_each(|snnp| {
            ret.insert(
                TypeId::build_primitive(snnp.to_string()),
                TypeDefinition::primitive(snnp),
            );
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
    pub type_id: TypeId,
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
            type_id: TypeId::build_primitive(type_name.to_string()),
        }
    }
}

/// Describes metadata surrounding a property's type, gathered from a combination of static & dynamic analysis
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(crate = "pax_message::serde")]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct TypeDefinition {
    /// Program-unique ID for this type
    pub type_id: TypeId,

    /// Statically known type_id for this Property's iterable TypeDefinition, that is,
    /// T for some Property<Vec<T>>
    pub inner_iterable_type_id: Option<TypeId>,

    /// A vec of PropertyType, describing known addressable (sub-)properties of this PropertyType
    pub property_definitions: Vec<PropertyDefinition>,
}

impl TypeDefinition {
    pub fn primitive(type_name: &str) -> Self {
        Self {
            type_id: TypeId::build_primitive(type_name.to_string()),
            property_definitions: vec![],
            inner_iterable_type_id: None,
        }
    }

    ///Used by Repeat for source expressions, e.g. the `self.some_vec` in `for elem in self.some_vec`
    pub fn builtin_vec_rc_ref_cell_any_properties(inner_iterable_type_id: TypeId) -> Self {
        Self {
            type_id: TypeId::build_vector("std::rc::Rc<core::cell::RefCell<dyn Any>>".to_string()),
            property_definitions: vec![],
            inner_iterable_type_id: Some(inner_iterable_type_id),
        }
    }

    pub fn builtin_range_isize() -> Self {
        Self {
            type_id: TypeId::build_range("isize".to_string()),
            property_definitions: vec![],
            inner_iterable_type_id: Some(TypeId::build_primitive("isize".to_string())),
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
    "pax_runtime::RepeatItem",
    "pax_runtime::RepeatProperties",
    "pax_runtime::ConditionalProperties",
    "pax_runtime::SlotProperties",
    "pax_runtime::get_numeric_from_wrapped_properties",
    "pax_runtime::api::PropertyInstance",
    "pax_runtime::api::PropertyLiteral",
    "pax_runtime::api::CommonProperties",
    "pax_runtime::ComponentInstance",
    "pax_runtime::InstanceNodePtr",
    "pax_runtime::PropertyExpression",
    "pax_runtime::InstanceNodePtrList",
    "pax_runtime::ExpressionContext",
    "pax_runtime::PaxEngine",
    "pax_runtime::InstanceNode",
    "pax_runtime::HandlerRegistry",
    "pax_runtime::InstantiationArgs",
    "pax_runtime::ConditionalInstance",
    "pax_runtime::SlotInstance",
    "pax_runtime::properties::RuntimePropertiesStackFrame",
    "pax_runtime::repeat::RepeatInstance",
    "piet_common::RenderContext",
];
