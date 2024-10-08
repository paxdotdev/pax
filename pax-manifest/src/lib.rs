use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fmt::Display;
use std::hash::Hasher;
use std::{cmp::Ordering, hash::Hash};

pub use pax_lang::interpreter::{PaxExpression, PaxIdentifier, PaxPrimary};
use pax_lang::DependencyCollector;
use pax_message::serde::{Deserialize, Serialize};
pub use pax_runtime_api;
use pax_runtime_api::{CoercionRules, HelperFunctions, Interpolatable, PaxValue, ToPaxValue};
pub mod parsing;
pub mod server;

#[cfg(feature = "parsing")]
pub mod utils;

pub mod cartridge_generation;
pub mod code_serialization;
pub mod constants;

/// Definition container for an entire Pax cartridge
#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "pax_message::serde")]
pub struct PaxManifest {
    #[serde_as(as = "BTreeMap<serde_with::json::JsonString, _>")]
    pub components: BTreeMap<TypeId, ComponentDefinition>,
    pub main_component_type_id: TypeId,
    #[serde_as(as = "HashMap<serde_with::json::JsonString, _>")]
    pub type_table: TypeTable,
    /// Compiler metadata: list of fully qualified asset directories, gathered during compiletime,
    /// from which assets will be copied for bundling into executable binaries
    pub assets_dirs: Vec<String>,
    /// Compiler metadata: the import prefix for the engine module, `pax_kit::pax_engine` by default
    /// but parameterizable for integrating with pax_engine directly, e.g. pax_std and pax_designer
    pub engine_import_path: String,
}

impl PaxManifest {
    pub fn is_designer(&self) -> bool {
        if let Some(identifier) = self.main_component_type_id.get_pascal_identifier() {
            return identifier == "PaxDesigner";
        }
        false
    }

    pub fn get_template_node(
        &self,
        uni: &UniqueTemplateNodeIdentifier,
    ) -> Option<&TemplateNodeDefinition> {
        self.components
            .get(&uni.component)?
            .template
            .as_ref()?
            .nodes
            .get(&uni.template_node_id)
    }

    pub fn get_all_component_property_definitions(
        &self,
        type_id: &TypeId,
    ) -> Vec<PropertyDefinition> {
        if let None = self.components.get(type_id) {
            return Vec::default();
        }
        let mut common_properties = get_common_properties_as_property_definitions();
        common_properties.extend(
            self.type_table
                .get(type_id)
                .map(|table| table.property_definitions.clone())
                .unwrap_or_default(),
        );
        common_properties
    }

    pub fn get_node_location(&self, uni: &UniqueTemplateNodeIdentifier) -> Option<NodeLocation> {
        self.components
            .get(&uni.component)
            .unwrap()
            .template
            .as_ref()
            .unwrap()
            .get_location(&uni.template_node_id)
    }

    pub fn get_all_property_names(&self, type_id: &TypeId) -> HashSet<String> {
        let mut ret = HashSet::new();
        self.get_all_component_property_definitions(type_id)
            .iter()
            .for_each(|prop| {
                ret.insert(prop.name.clone());
            });
        ret
    }

    // String representing the symbolic ID for the &dyn PaxCartridge struct
    // generated for this manifest's cartridge in #[main]
    pub fn get_main_cartridge_struct_id(&self) -> String {
        format!(
            "{}{}",
            &self.main_component_type_id.get_pascal_identifier().unwrap(),
            crate::constants::CARTRIDGE_PARTIAL_STRUCT_ID
        )
    }

    // String representing the symbolic ID for the &dyn DefinitionToInstanceTraverser struct
    // generated for this manifest's cartridge in #[main]
    pub fn get_main_definition_to_instance_traverser_struct_id(&self) -> String {
        format!(
            "{}{}",
            &self.main_component_type_id.get_pascal_identifier().unwrap(),
            crate::constants::DEFINITION_TO_INSTANCE_TRAVERSER_PARTIAL_STRUCT_ID
        )
    }

    pub fn merge_in_place(&mut self, other: &PaxManifest) {
        self.components.extend(other.components.clone());
        self.type_table.extend(other.type_table.clone());
        self.assets_dirs.extend(other.assets_dirs.clone());
    }
}

pub fn get_common_properties_type_ids() -> Vec<TypeId> {
    let mut ret = vec![];
    for (_, import_path) in constants::COMMON_PROPERTIES_TYPE {
        if SUPPORTED_NUMERIC_PRIMITIVES.contains(import_path)
            || SUPPORTED_NONNUMERIC_PRIMITIVES.contains(import_path)
        {
            ret.push(TypeId::build_primitive(import_path));
        } else {
            ret.push(TypeId::build_singleton(import_path, None));
        }
    }
    ret
}

pub fn get_common_properties_as_property_definitions() -> Vec<PropertyDefinition> {
    let mut ret = vec![];
    for (cp, import_path) in constants::COMMON_PROPERTIES_TYPE {
        if SUPPORTED_NUMERIC_PRIMITIVES.contains(import_path)
            || SUPPORTED_NONNUMERIC_PRIMITIVES.contains(import_path)
        {
            ret.push(PropertyDefinition {
                name: cp.to_string(),
                flags: Default::default(),
                type_id: TypeId::build_primitive(import_path),
            });
        } else {
            ret.push(PropertyDefinition {
                name: cp.to_string(),
                flags: Default::default(),
                type_id: TypeId::build_singleton(import_path, None),
            });
        }
    }
    ret
}

pub const SUPPORTED_NUMERIC_PRIMITIVES: [&str; 13] = [
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "f64",
];

pub const SUPPORTED_NONNUMERIC_PRIMITIVES: [&str; 2] = ["String", "bool"];

/// Container for an entire component definition — includes template, settings,
/// event bindings, property definitions, and compiler + reflection metadata
#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "pax_message::serde")]
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
}

impl ComponentDefinition {
    pub fn get_property_definitions<'a>(&self, tt: &'a TypeTable) -> &'a Vec<PropertyDefinition> {
        &tt.get(&self.type_id).unwrap().property_definitions
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "pax_message::serde")]
pub enum SettingsBlockElement {
    SelectorBlock(Token, LiteralBlockDefinition),
    Handler(Token, Vec<Token>),
    Comment(String),
}

#[derive(Serialize, Default, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
#[serde(crate = "pax_message::serde")]
pub struct TemplateNodeId(usize);

impl CoercionRules for TemplateNodeId {
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        match value {
            PaxValue::Numeric(_) => Ok(TemplateNodeId(usize::try_coerce(value)?)),
            _ => Err("Cannot coerce PaxValue into TemplateNodeId".to_string()),
        }
    }
}

impl HelperFunctions for TemplateNodeId {}

impl ToPaxValue for TemplateNodeId {
    fn to_pax_value(self) -> PaxValue {
        self.0.to_pax_value()
    }
}

impl TemplateNodeId {
    pub fn build(id: usize) -> Self {
        TemplateNodeId(id)
    }

    pub fn as_usize(&self) -> usize {
        self.0
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
    BlankComponent {
        pascal_identifier: String,
    },
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

impl HelperFunctions for PaxType {}

impl CoercionRules for PaxType {
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        match value {
            PaxValue::Enum(_name, variant, args) => match variant.as_str() {
                "If" => Ok(PaxType::If),
                "Slot" => Ok(PaxType::Slot),
                "Repeat" => Ok(PaxType::Repeat),
                "Comment" => Ok(PaxType::Comment),
                "BlankComponent" => {
                    let pascal_identifier = String::try_coerce(args[0].clone())?;
                    Ok(PaxType::BlankComponent { pascal_identifier })
                }
                "Primitive" => {
                    let pascal_identifier = String::try_coerce(args[0].clone())?;
                    Ok(PaxType::Primitive { pascal_identifier })
                }
                "Singleton" => {
                    let pascal_identifier = String::try_coerce(args[0].clone())?;
                    Ok(PaxType::Singleton { pascal_identifier })
                }
                "Range" => {
                    let identifier = String::try_coerce(args[0].clone())?;
                    Ok(PaxType::Range { identifier })
                }
                "Option" => {
                    let identifier = String::try_coerce(args[0].clone())?;
                    Ok(PaxType::Option { identifier })
                }
                "Vector" => {
                    let elem_identifier = String::try_coerce(args[0].clone())?;
                    Ok(PaxType::Vector { elem_identifier })
                }
                "Map" => {
                    let key_identifier = String::try_coerce(args[0].clone())?;
                    let value_identifier = String::try_coerce(args[1].clone())?;
                    Ok(PaxType::Map {
                        key_identifier,
                        value_identifier,
                    })
                }
                _ => Ok(PaxType::Unknown),
            },
            _ => Err("Cannot coerce PaxValue into PaxType".to_string()),
        }
    }
}

impl ToPaxValue for PaxType {
    fn to_pax_value(self) -> PaxValue {
        match self {
            PaxType::If => PaxValue::Enum("PaxType".to_string(), "If".to_string(), vec![]),
            PaxType::Slot => PaxValue::Enum("PaxType".to_string(), "Slot".to_string(), vec![]),
            PaxType::Repeat => PaxValue::Enum("PaxType".to_string(), "Repeat".to_string(), vec![]),
            PaxType::Comment => {
                PaxValue::Enum("PaxType".to_string(), "Comment".to_string(), vec![])
            }
            PaxType::BlankComponent { pascal_identifier } => PaxValue::Enum(
                "PaxType".to_string(),
                "BlankComponent".to_string(),
                vec![pascal_identifier.to_pax_value()],
            ),
            PaxType::Primitive { pascal_identifier } => PaxValue::Enum(
                "PaxType".to_string(),
                "Primitive".to_string(),
                vec![pascal_identifier.to_pax_value()],
            ),
            PaxType::Singleton { pascal_identifier } => PaxValue::Enum(
                "PaxType".to_string(),
                "Singleton".to_string(),
                vec![pascal_identifier.to_pax_value()],
            ),
            PaxType::Range { identifier } => PaxValue::Enum(
                "PaxType".to_string(),
                "Range".to_string(),
                vec![identifier.to_pax_value()],
            ),
            PaxType::Option { identifier } => PaxValue::Enum(
                "PaxType".to_string(),
                "Option".to_string(),
                vec![identifier.to_pax_value()],
            ),
            PaxType::Vector { elem_identifier } => PaxValue::Enum(
                "PaxType".to_string(),
                "Vector".to_string(),
                vec![elem_identifier.to_pax_value()],
            ),
            PaxType::Map {
                key_identifier,
                value_identifier,
            } => PaxValue::Enum(
                "PaxType".to_string(),
                "Map".to_string(),
                vec![
                    key_identifier.to_pax_value(),
                    value_identifier.to_pax_value(),
                ],
            ),
            PaxType::Unknown => {
                PaxValue::Enum("PaxType".to_string(), "Unknown".to_string(), vec![])
            }
        }
    }
}

impl Display for PaxType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaxType::If => write!(f, "If"),
            PaxType::Slot => write!(f, "Slot"),
            PaxType::Repeat => write!(f, "Repeat"),
            PaxType::Comment => write!(f, "Comment"),
            PaxType::BlankComponent { pascal_identifier } => write!(f, "{}", pascal_identifier),
            PaxType::Primitive { pascal_identifier } => write!(f, "{}", pascal_identifier),
            PaxType::Singleton { pascal_identifier } => write!(f, "{}", pascal_identifier),
            PaxType::Range { identifier } => write!(f, "std::ops::Range<{}>", identifier),
            PaxType::Option { identifier } => write!(f, "std::option::Option<{}>", identifier),
            PaxType::Vector { elem_identifier } => write!(f, "std::vec::Vec<{}>", elem_identifier),
            PaxType::Map {
                key_identifier,
                value_identifier,
            } => write!(
                f,
                "std::collections::HashMap<{}><{}>",
                key_identifier, value_identifier
            ),
            PaxType::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Serialize, Default, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
#[serde(crate = "pax_message::serde")]
pub struct TypeId {
    pax_type: PaxType,
    import_path: Option<String>,
    is_intoable_downstream_type: bool,

    _type_id: String,
    _type_id_escaped: String,
}

impl HelperFunctions for TypeId {}

impl PartialOrd for TypeId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self._type_id.partial_cmp(&other._type_id)
    }
}

impl CoercionRules for TypeId {
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        match value {
            PaxValue::Object(map) => {
                let pax_type = PaxType::try_coerce(map.get("pax_type").unwrap().clone())?;
                let import_path = map
                    .get("import_path")
                    .map(|v| Option::<String>::try_coerce(v.clone()))
                    .unwrap_or(Ok(None))?;
                let is_intoable_downstream_type = map
                    .get("is_intoable_downstream_type")
                    .map(|v| bool::try_coerce(v.clone()))
                    .unwrap_or(Ok(false))?;
                let _type_id = map
                    .get("_type_id")
                    .map(|v| String::try_coerce(v.clone()))
                    .unwrap_or(Ok("".to_string()))?;
                let _type_id_escaped = map
                    .get("_type_id_escaped")
                    .map(|v| String::try_coerce(v.clone()))
                    .unwrap_or(Ok("".to_string()))?;
                Ok(Self {
                    pax_type,
                    import_path,
                    is_intoable_downstream_type,
                    _type_id,
                    _type_id_escaped,
                })
            }
            _ => Err("Cannot coerce PaxValue into TypeId".to_string()),
        }
    }
}

impl ToPaxValue for TypeId {
    fn to_pax_value(self) -> PaxValue {
        let mut map = HashMap::new();
        map.insert("pax_type".to_string(), self.pax_type.to_pax_value());
        map.insert("import_path".to_string(), self.import_path.to_pax_value());
        map.insert(
            "is_intoable_downstream_type".to_string(),
            self.is_intoable_downstream_type.to_pax_value(),
        );
        map.insert("_type_id".to_string(), self._type_id.to_pax_value());
        map.insert(
            "_type_id_escaped".to_string(),
            self._type_id_escaped.to_pax_value(),
        );
        PaxValue::Object(map)
    }
}

impl Ord for TypeId {
    fn cmp(&self, other: &Self) -> Ordering {
        self._type_id.cmp(&other._type_id)
    }
}

impl Interpolatable for TypeId {}
impl Interpolatable for TemplateNodeId {}

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
            is_intoable_downstream_type: false,
            _type_id: "If".to_string(),
            _type_id_escaped: "If".to_string(),
        }
    }

    pub fn build_repeat() -> Self {
        TypeId {
            pax_type: PaxType::Repeat,
            import_path: None,
            is_intoable_downstream_type: false,
            _type_id: "Repeat".to_string(),
            _type_id_escaped: "Repeat".to_string(),
        }
    }

    pub fn build_slot() -> Self {
        TypeId {
            pax_type: PaxType::Slot,
            import_path: None,
            is_intoable_downstream_type: false,
            _type_id: "Slot".to_string(),
            _type_id_escaped: "Slot".to_string(),
        }
    }

    pub fn build_comment() -> Self {
        TypeId {
            pax_type: PaxType::Comment,
            import_path: None,
            is_intoable_downstream_type: false,
            _type_id: "Comment".to_string(),
            _type_id_escaped: "Comment".to_string(),
        }
    }

    /// Build a typeid for a transient component
    pub fn build_blank_component(pascal_identifier: &str) -> Self {
        TypeId {
            pax_type: PaxType::BlankComponent {
                pascal_identifier: pascal_identifier.to_owned(),
            },
            import_path: None,
            is_intoable_downstream_type: false,
            _type_id: pascal_identifier.to_owned(),
            _type_id_escaped: escape_identifier(pascal_identifier.to_owned()),
        }
    }

    /// Build a TypeId for a most types, like `Stacker` or `SpecialComponent`
    pub fn build_singleton(import_path: &str, pascal_identifier: Option<&str>) -> Self {
        let pascal_identifier = if let Some(p) = pascal_identifier {
            p.to_owned()
        } else {
            import_path.split("::").last().unwrap().to_string()
        };

        Self {
            pax_type: PaxType::Singleton { pascal_identifier },
            import_path: Some(import_path.to_owned()),
            is_intoable_downstream_type: crate::constants::is_intoable_downstream_type(import_path),
            _type_id: import_path.to_owned(),
            _type_id_escaped: escape_identifier(import_path.to_owned()),
        }
    }

    /// Build a TypeId for rust primitives like `u8` or `String`
    pub fn build_primitive(identifier: &str) -> Self {
        TypeId {
            pax_type: PaxType::Primitive {
                pascal_identifier: identifier.to_owned(),
            },
            import_path: None,
            is_intoable_downstream_type: crate::constants::is_intoable_downstream_type(identifier),
            _type_id: identifier.to_owned(),
            _type_id_escaped: identifier.to_owned(),
        }
    }

    /// Build a TypeId for vector types like `Vec<Color>`
    pub fn build_vector(elem_identifier: &str) -> Self {
        let _id = format!("std::vec::Vec<{}>", elem_identifier);
        Self {
            pax_type: PaxType::Vector {
                elem_identifier: elem_identifier.to_owned(),
            },
            import_path: Some("std::vec::Vec".to_string()),
            is_intoable_downstream_type: false,
            _type_id: _id.clone(),
            _type_id_escaped: escape_identifier(_id),
        }
    }

    /// Build a TypeId for range types like `std::ops::Range<Color>`
    pub fn build_range(identifier: &str) -> Self {
        let _id = format!("std::ops::Range<{}>", identifier);
        Self {
            pax_type: PaxType::Range {
                identifier: identifier.to_owned(),
            },
            import_path: Some("std::ops::Range".to_string()),
            is_intoable_downstream_type: false,
            _type_id: _id.clone(),
            _type_id_escaped: escape_identifier(_id),
        }
    }

    /// Build a TypeId for option types like `std::option::Option<Color>`
    pub fn build_option(identifier: &str) -> Self {
        let _id = format!("std::option::Option<{}>", identifier);
        Self {
            pax_type: PaxType::Option {
                identifier: identifier.to_owned(),
            },
            import_path: Some("std::option::Option".to_string()),
            is_intoable_downstream_type: false,
            _type_id: _id.clone(),
            _type_id_escaped: escape_identifier(_id),
        }
    }

    /// Build a TypeId for map types like `std::collections::HashMap<String><Color>`
    pub fn build_map(key_identifier: &str, value_identifier: &str) -> Self {
        let _id = format!(
            "std::collections::HashMap<{}><{}>",
            key_identifier.to_owned(),
            value_identifier.to_owned()
        );
        Self {
            pax_type: PaxType::Map {
                key_identifier: key_identifier.to_owned(),
                value_identifier: value_identifier.to_owned(),
            },
            import_path: Some("std::collections::HashMap".to_string()),
            is_intoable_downstream_type: false,
            _type_id: _id.clone(),
            _type_id_escaped: escape_identifier(_id),
        }
    }

    pub fn import_path(&self) -> Option<String> {
        if let PaxType::Primitive { pascal_identifier } = &self.pax_type {
            return Some(pascal_identifier.clone());
        }
        self.import_path.clone()
    }

    pub fn get_pascal_identifier(&self) -> Option<String> {
        match &self.pax_type {
            PaxType::Primitive { pascal_identifier }
            | PaxType::Singleton { pascal_identifier }
            | PaxType::BlankComponent { pascal_identifier } => Some(pascal_identifier.clone()),
            PaxType::If | PaxType::Slot | PaxType::Repeat | PaxType::Comment => {
                Some(self.pax_type.to_string())
            }
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

    pub fn is_blank_component(&self) -> bool {
        if let PaxType::BlankComponent { .. } = self.pax_type {
            true
        } else {
            false
        }
    }
}

impl Interpolatable for UniqueTemplateNodeIdentifier {}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq, Default)]
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

    pub fn get_containing_component_type_id(&self) -> TypeId {
        self.component.clone()
    }

    pub fn get_template_node_id(&self) -> TemplateNodeId {
        self.template_node_id.clone()
    }
}

impl Display for UniqueTemplateNodeIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.component, self.template_node_id)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub enum TreeLocation {
    #[default]
    Root,
    Parent(TemplateNodeId),
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub enum TreeIndexPosition {
    #[default]
    Top,
    Bottom,
    At(usize),
}

impl TreeIndexPosition {
    pub fn get_index(&self, len: usize) -> usize {
        match self {
            TreeIndexPosition::Top => 0,
            TreeIndexPosition::Bottom => len,
            TreeIndexPosition::At(index) => *index,
        }
    }

    pub fn new(index: usize) -> Self {
        TreeIndexPosition::At(index)
    }
}

impl PartialOrd for TreeIndexPosition {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self {
            TreeIndexPosition::Top => match other {
                TreeIndexPosition::Top => Some(Ordering::Equal),
                TreeIndexPosition::Bottom => Some(Ordering::Less),
                TreeIndexPosition::At(_) => Some(Ordering::Less),
            },
            TreeIndexPosition::Bottom => match other {
                TreeIndexPosition::Top => Some(Ordering::Greater),
                TreeIndexPosition::Bottom => Some(Ordering::Equal),
                TreeIndexPosition::At(_) => Some(Ordering::Less),
            },
            TreeIndexPosition::At(index) => match other {
                TreeIndexPosition::Top => Some(Ordering::Greater),
                TreeIndexPosition::Bottom => Some(Ordering::Greater),
                TreeIndexPosition::At(other_index) => index.partial_cmp(other_index),
            },
        }
    }
}

impl Ord for TreeIndexPosition {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct NodeLocation {
    pub type_id: TypeId,
    pub tree_location: TreeLocation,
    pub index: TreeIndexPosition,
}

impl NodeLocation {
    pub fn new(type_id: TypeId, location: TreeLocation, index: TreeIndexPosition) -> Self {
        NodeLocation {
            type_id,
            tree_location: location,
            index,
        }
    }

    pub fn get_tree_location(&self) -> &TreeLocation {
        &self.tree_location
    }

    pub fn get_type_id(&self) -> &TypeId {
        &self.type_id
    }

    pub fn root(type_id: TypeId) -> Self {
        NodeLocation {
            type_id,
            tree_location: TreeLocation::Root,
            index: TreeIndexPosition::Top,
        }
    }

    pub fn parent(type_id: TypeId, parent: TemplateNodeId) -> Self {
        NodeLocation {
            type_id,
            tree_location: TreeLocation::Parent(parent),
            index: TreeIndexPosition::Top,
        }
    }

    pub fn set_index(&mut self, index: TreeIndexPosition) {
        self.index = index;
    }
}

impl PartialOrd for NodeLocation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.tree_location {
            TreeLocation::Root => match other.tree_location {
                TreeLocation::Root => {
                    return self.index.partial_cmp(&other.index);
                }
                TreeLocation::Parent(_) => {
                    return Some(Ordering::Less);
                }
            },
            TreeLocation::Parent(_) => match other.tree_location {
                TreeLocation::Root => {
                    return Some(Ordering::Greater);
                }
                TreeLocation::Parent(_) => {
                    return self.index.partial_cmp(&other.index);
                }
            },
        }
    }
}

impl Ord for NodeLocation {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
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

    pub fn get_containing_component_type_id(&self) -> TypeId {
        self.containing_component.clone()
    }

    pub fn get_next_id(&self) -> usize {
        self.next_id
    }

    pub fn set_next_id(&mut self, id: usize) {
        self.next_id = id;
    }

    pub fn get_file_path(&self) -> Option<String> {
        self.template_source_file_path.clone()
    }

    pub fn get_unique_identifier(&self, id: TemplateNodeId) -> UniqueTemplateNodeIdentifier {
        let type_id = self.containing_component.clone();
        UniqueTemplateNodeIdentifier::build(type_id, id)
    }

    pub fn contains_slots(&self) -> bool {
        self.nodes
            .values()
            .any(|v| v.type_id.pax_type == PaxType::Slot)
    }

    fn consume_next_id(&mut self) -> TemplateNodeId {
        let current_next_id = self.next_id;
        self.next_id = self.next_id + 1;
        TemplateNodeId::build(current_next_id)
    }

    pub fn add_at(
        &mut self,
        tnd: TemplateNodeDefinition,
        location: NodeLocation,
    ) -> UniqueTemplateNodeIdentifier {
        match location.get_tree_location() {
            TreeLocation::Root => match location.index {
                TreeIndexPosition::Top => {
                    return self.add_root_node_front(tnd);
                }
                TreeIndexPosition::Bottom => {
                    return self.add_root_node_back(tnd);
                }
                TreeIndexPosition::At(index) => {
                    return self.add_root_node_at(index, tnd);
                }
            },
            TreeLocation::Parent(p) => match location.index {
                TreeIndexPosition::Top => {
                    return self.add_child_front(p.clone(), tnd);
                }
                TreeIndexPosition::Bottom => {
                    return self.add_child_back(p.clone(), tnd);
                }
                TreeIndexPosition::At(index) => {
                    return self.add_child_at(p.clone(), index, tnd);
                }
            },
        }
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
        self.children
            .entry(id.clone())
            .or_insert_with(VecDeque::new);
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
            let subtree = self.get_subtree(&id);
            for node in subtree {
                self.nodes.remove(&node);
                self.children.remove(&node);
                for (_, children) in self.children.iter_mut() {
                    children.retain(|child| *child != node);
                }
            }
            self.nodes.remove(&id);
            self.children.remove(&id);
            for c in self.children.values_mut() {
                c.retain(|v| v != &id);
            }
            self.root.retain(|child| *child != id);
            node
        } else {
            panic!("Requested node doesn't exist in template");
        }
    }

    fn get_subtree(&self, id: &TemplateNodeId) -> Vec<TemplateNodeId> {
        let mut ret = vec![];
        if let Some(children) = self.children.get(&id) {
            for child in children {
                ret.push(child.clone());
                ret.extend(self.get_subtree(child));
            }
        }
        ret
    }

    pub fn get_root(&self) -> Vec<TemplateNodeId> {
        self.root.clone().into()
    }

    pub fn get_children(&self, id: &TemplateNodeId) -> Option<Vec<TemplateNodeId>> {
        if let Some(c) = self.children.get(&id) {
            return Some(c.clone().into());
        }
        None
    }

    pub fn get_parent(&self, id: &TemplateNodeId) -> Option<TemplateNodeId> {
        if self.root.contains(id) {
            return None;
        }
        self.children
            .iter()
            .find_map(|(n_id, child_ids)| child_ids.contains(&id).then_some(n_id).cloned())
    }

    pub fn get_node(&self, id: &TemplateNodeId) -> Option<&TemplateNodeDefinition> {
        self.nodes.get(id)
    }

    pub fn set_node(&mut self, id: TemplateNodeId, tnd: TemplateNodeDefinition) {
        self.nodes.insert(id, tnd);
    }

    pub fn update_node_type_id(&mut self, id: &TemplateNodeId, new_type: &TypeId) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.type_id = new_type.clone();
        }
    }

    pub fn update_node_properties(
        &mut self,
        id: &TemplateNodeId,
        mut properties: HashMap<Token, Option<ValueDefinition>>,
    ) {
        if let Some(node) = self.nodes.get_mut(id) {
            if let Some(settings) = &mut node.settings {
                let mut indexes_to_remove: Vec<usize> = vec![];
                for (i, setting) in settings.iter_mut().enumerate() {
                    if let SettingElement::Setting(key, v) = setting {
                        if let Some(new_value) = properties.remove(key) {
                            if let Some(updated) = new_value {
                                *v = updated;
                            } else {
                                indexes_to_remove.push(i);
                            }
                        }
                    }
                }

                // Remove propertiest that have been set to None
                for i in indexes_to_remove.iter().rev() {
                    settings.remove(*i);
                }
            } else {
                node.settings = Some(vec![]);
            }
        }
        // Add remaining (aka new properties) to settings
        for (k, v) in properties.iter() {
            if let Some(node) = self.nodes.get_mut(id) {
                if let Some(settings) = &mut node.settings {
                    if let Some(value) = v {
                        settings.push(SettingElement::Setting(k.clone(), value.clone()));
                    }
                }
            }
        }
    }

    pub fn update_control_flow_properties(
        &mut self,
        id: &TemplateNodeId,
        repeat_predicate_definition: Option<Option<ControlFlowRepeatPredicateDefinition>>,
        repeat_source_expression: Option<Option<ExpressionInfo>>,
        conditional_expression: Option<Option<ExpressionInfo>>,
        slot_index_expression: Option<Option<ExpressionInfo>>,
    ) {
        let Some(control_flow_settings) = self
            .nodes
            .get_mut(id)
            .and_then(|n| n.control_flow_settings.as_mut())
        else {
            return;
        };
        if let Some(value) = repeat_predicate_definition {
            control_flow_settings.repeat_predicate_definition = value;
        }
        if let Some(value) = repeat_source_expression {
            control_flow_settings.repeat_source_expression = value;
        }
        if let Some(value) = conditional_expression {
            control_flow_settings.condition_expression = value;
        }
        if let Some(value) = slot_index_expression {
            control_flow_settings.slot_index_expression = value;
        }
    }

    pub fn find_node_with_str_id(&self, id: &str) -> Option<&TemplateNodeId> {
        for (i, n) in &self.nodes {
            if let Some(settings) = &n.settings {
                for setting in settings {
                    if let SettingElement::Setting(setting, value) = setting {
                        if setting.token_value == "id" {
                            if let ValueDefinition::LiteralValue(val) = value {
                                if let PaxValue::String(val_id) = val {
                                    if id == val_id {
                                        return Some(i);
                                    }
                                };
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn get_nodes(&self) -> Vec<&TemplateNodeDefinition> {
        self.nodes.values().collect()
    }

    pub fn get_nodes_mut(&mut self) -> Vec<&mut TemplateNodeDefinition> {
        self.nodes.values_mut().collect()
    }

    pub fn get_nodes_owned(&self) -> Vec<TemplateNodeDefinition> {
        self.nodes.values().map(|x| x.clone()).collect()
    }

    pub fn get_ids(&self) -> Vec<&TemplateNodeId> {
        self.nodes.keys().collect()
    }

    pub fn get_location(&self, id: &TemplateNodeId) -> Option<NodeLocation> {
        if self.root.contains(&id) {
            let mut node_location = NodeLocation::root(self.containing_component.clone());
            node_location.set_index(TreeIndexPosition::new(
                self.root.iter().position(|x| *x == *id).unwrap(),
            ));
            return Some(node_location);
        }
        for (parent, children) in self.children.iter() {
            if children.contains(&id) {
                let mut node_location =
                    NodeLocation::parent(self.containing_component.clone(), parent.clone());
                node_location.set_index(TreeIndexPosition::new(
                    children.iter().position(|x| *x == *id).unwrap(),
                ));
                return Some(node_location);
            }
        }
        None
    }

    pub fn detach_node(&mut self, id: &TemplateNodeId) -> NodeLocation {
        let current_location = self
            .get_location(id)
            .expect("Node doesn't exist in template");
        let parent = match current_location.get_tree_location() {
            TreeLocation::Root => {
                self.root.retain(|root_node| *root_node != *id);
                return current_location;
            }
            TreeLocation::Parent(parent) => parent,
        };
        let children = self.children.get_mut(&parent).unwrap();
        children.retain(|child| *child != *id);
        current_location
    }

    pub fn get_siblings(&self, id: &TemplateNodeId) -> Option<VecDeque<TemplateNodeId>> {
        if self.root.contains(id) {
            Some(self.root.clone())
        } else {
            Some(self.children.values().find(|&v| v.contains(id)).cloned()?)
        }
    }

    pub fn move_node(&mut self, id: &TemplateNodeId, new_location: NodeLocation) {
        self.detach_node(id);
        let (target_list, index) = match new_location.get_tree_location() {
            TreeLocation::Root => (&mut self.root, new_location.index),
            TreeLocation::Parent(p) => (
                self.children.entry(p.clone()).or_default(),
                new_location.index,
            ),
        };
        let index = match index {
            TreeIndexPosition::Top => 0,
            TreeIndexPosition::Bottom => target_list.len(),
            TreeIndexPosition::At(i) => i,
        };
        target_list.insert(index.clamp(0, target_list.len()), id.clone());
    }

    pub fn get_all_children_relationships(
        &self,
    ) -> HashMap<TemplateNodeId, VecDeque<TemplateNodeId>> {
        self.children.clone()
    }

    pub fn merge_with_settings(&mut self, settings_block: &Option<Vec<SettingsBlockElement>>) {
        for node in self.get_nodes_mut() {
            node.settings = PaxManifest::merge_inline_settings_with_settings_block(
                &mut node.settings,
                settings_block,
            );
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

impl TemplateNodeDefinition {
    pub fn get_node_type(&self) -> NodeType {
        if let Some(cfsd) = &self.control_flow_settings {
            NodeType::ControlFlow(Box::new(cfsd.clone()))
        } else if let Some(settings) = &self.settings {
            NodeType::Template(settings.clone())
        } else if let Some(comment) = &self.raw_comment_string {
            NodeType::Comment(comment.clone())
        } else {
            panic!("Invalid TemplateNodeDefinition");
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum NodeType {
    Template(Vec<SettingElement>),
    ControlFlow(Box<ControlFlowSettingsDefinition>),
    Comment(String),
}

pub type TypeTable = HashMap<TypeId, TypeDefinition>;
pub fn get_primitive_type_table() -> TypeTable {
    let mut ret: TypeTable = Default::default();

    SUPPORTED_NUMERIC_PRIMITIVES.into_iter().for_each(|snp| {
        ret.insert(TypeId::build_primitive(snp), TypeDefinition::primitive(snp));
    });
    SUPPORTED_NONNUMERIC_PRIMITIVES
        .into_iter()
        .for_each(|snnp| {
            ret.insert(
                TypeId::build_primitive(snnp),
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
            type_id: TypeId::build_primitive(type_name),
        }
    }
}

/// Describes metadata surrounding a property's type, gathered from a combination of static & dynamic analysis
#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(crate = "pax_message::serde")]
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
            type_id: TypeId::build_primitive(type_name),
            property_definitions: vec![],
            inner_iterable_type_id: None,
        }
    }

    ///Used by Repeat for source expressions, e.g. the `self.some_vec` in `for elem in self.some_vec`
    pub fn builtin_vec_rc_ref_cell_any_properties(inner_iterable_type_id: TypeId) -> Self {
        Self {
            type_id: TypeId::build_vector("std::rc::Rc<core::cell::RefCell<PaxAny>>"),
            property_definitions: vec![],
            inner_iterable_type_id: Some(inner_iterable_type_id),
        }
    }

    pub fn builtin_range_isize() -> Self {
        Self {
            type_id: TypeId::build_range("isize"),
            property_definitions: vec![],
            inner_iterable_type_id: Some(TypeId::build_primitive("isize")),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct ExpressionCompilationInfo {
    pub vtable_id: usize,
    /// symbols used in the expression
    pub dependencies: Vec<String>,
}

/// Container for settings values, storing all possible
/// variants, populated at parse-time and used at compile-time
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(crate = "pax_message::serde")]
pub enum ValueDefinition {
    #[default]
    Undefined, //Used for `Default`
    LiteralValue(PaxValue),
    Block(LiteralBlockDefinition),
    /// (Expression contents, vtable id binding)
    Expression(ExpressionInfo),
    /// (Expression contents, vtable id binding)
    Identifier(PaxIdentifier),
    /// (Expression contents, vtable id binding)
    DoubleBinding(PaxIdentifier),
    EventBindingTarget(PaxIdentifier),
}

/// Container for holding metadata about original Location in Pax Template
/// Used for source-mapping
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Hash, Eq, PartialOrd, Ord)]
#[serde(crate = "pax_message::serde")]
pub struct LocationInfo {
    pub start_line_col: (usize, usize),
    pub end_line_col: (usize, usize),
}

/// Container for holding parsed data describing a Repeat (`for`)
/// predicate, for example the `(elem, i)` in `for (elem, i) in foo` or
/// the `elem` in `for elem in foo`
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(crate = "pax_message::serde")]
pub enum ControlFlowRepeatPredicateDefinition {
    ElemId(String),
    ElemIdIndexId(String, String),
}

impl ControlFlowRepeatPredicateDefinition {
    pub fn get_symbols(&self) -> HashSet<String> {
        match self {
            ControlFlowRepeatPredicateDefinition::ElemId(t) => {
                vec![t.clone()].into_iter().collect()
            }
            ControlFlowRepeatPredicateDefinition::ElemIdIndexId(t1, t2) => {
                vec![t1.clone(), t2.clone()].into_iter().collect()
            }
        }
    }
}

/// Container for storing parsed control flow information, for
/// example the string (PAXEL) representations of condition / slot / repeat
/// expressions and the related vtable ids (for "punching" during expression compilation)
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(crate = "pax_message::serde")]
pub struct ControlFlowSettingsDefinition {
    pub condition_expression: Option<ExpressionInfo>,
    pub slot_index_expression: Option<ExpressionInfo>,
    pub repeat_predicate_definition: Option<ControlFlowRepeatPredicateDefinition>,
    pub repeat_source_expression: Option<ExpressionInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(crate = "pax_message::serde")]
pub struct ExpressionInfo {
    pub expression: PaxExpression,
    pub dependencies: Vec<String>,
}

impl Display for ExpressionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.expression)
    }
}

impl ExpressionInfo {
    pub fn new(expr: PaxExpression) -> Self {
        Self {
            dependencies: expr.collect_dependencies(),
            expression: expr,
        }
    }
}

/// Container for a parsed Literal object
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
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
#[derive(Serialize, Deserialize, Debug, Clone, Default, Eq)]
#[serde(crate = "pax_message::serde")]
pub struct Token {
    pub token_value: String,
    pub token_location: Option<LocationInfo>,
}

impl PartialEq for Token {
    fn eq(&self, other: &Self) -> bool {
        self.token_value == other.token_value
    }
}

impl Hash for Token {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.token_value.hash(state)
    }
}

impl PartialOrd for Token {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.token_value.partial_cmp(&other.token_value)
    }
}

impl Ord for Token {
    fn cmp(&self, other: &Self) -> Ordering {
        self.token_value.cmp(&other.token_value)
    }
}

impl Token {
    pub fn new(token_value: String, token_location: LocationInfo) -> Self {
        Self {
            token_value,
            token_location: Some(token_location),
        }
    }

    pub fn new_without_location(token_value: String) -> Self {
        Self {
            token_value,
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

//Effectively our `Prelude` types
pub const IMPORTS_BUILTINS: &[&str] = &[
    "std::any::Any",
    "pax_runtime::api::{use_RefCell}",
    "std::collections::HashMap",
    "std::collections::VecDeque",
    "std::ops::Deref",
    "std::rc::Rc",
    "pax_runtime::RepeatItem",
    "pax_runtime::RepeatProperties",
    "pax_runtime::ConditionalProperties",
    "pax_runtime::Slot",
    "pax_runtime::api::Property",
    "pax_runtime::api::CommonProperties",
    "pax_runtime::api::Color::*",
    "pax_runtime::ComponentInstance",
    "pax_runtime::InstanceNodePtr",
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
