use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt;

use pax_message::serde::de::{
    self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess,
    Visitor,
};
use pax_message::serde::ser::{
    self, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant,
};
use pax_message::serde::{Deserialize, Serialize};

use crate::{
    ComponentDefinition, ComponentTemplate, PaxManifest, RouteBranchDescriptor,
    SettingsBlockElement, TemplateNodeDefinition, TemplateNodeId, TimelineDefinition,
    TypeDefinition, TypeId,
};

const MAGIC: &[u8; 8] = b"PAXM\x00BIN";
const VERSION: u8 = 4;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    message: String,
}

impl Error {
    fn message(message: impl fmt::Display) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

impl ser::Error for Error {
    fn custom<T>(message: T) -> Self
    where
        T: fmt::Display,
    {
        Error::message(message)
    }
}

impl de::Error for Error {
    fn custom<T>(message: T) -> Self
    where
        T: fmt::Display,
    {
        Error::message(message)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "pax_message::serde")]
struct BinaryPaxManifest {
    components: Vec<(TypeId, BinaryComponentDefinition)>,
    main_component_type_id: TypeId,
    type_table: Vec<(TypeId, TypeDefinition)>,
    assets_dirs: Vec<String>,
    engine_import_path: String,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "pax_message::serde")]
struct BinaryComponentDefinition {
    type_id: TypeId,
    is_main_component: bool,
    is_primitive: bool,
    is_struct_only_component: bool,
    module_path: String,
    primitive_instance_import_path: Option<String>,
    template: Option<BinaryComponentTemplate>,
    settings: Option<Vec<SettingsBlockElement>>,
    timelines: Vec<TimelineDefinition>,
    route_branch: Option<RouteBranchDescriptor>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "pax_message::serde")]
struct BinaryComponentTemplate {
    containing_component: TypeId,
    root: VecDeque<TemplateNodeId>,
    children: Vec<(TemplateNodeId, VecDeque<TemplateNodeId>)>,
    nodes: Vec<(TemplateNodeId, TemplateNodeDefinition)>,
    next_id: usize,
    template_source_file_path: Option<String>,
}

impl From<&PaxManifest> for BinaryPaxManifest {
    fn from(manifest: &PaxManifest) -> Self {
        let components = manifest
            .components
            .iter()
            .map(|(type_id, definition)| {
                (type_id.clone(), BinaryComponentDefinition::from(definition))
            })
            .collect();

        let mut type_table = manifest
            .type_table
            .iter()
            .map(|(type_id, definition)| (type_id.clone(), definition.clone()))
            .collect::<Vec<_>>();
        type_table.sort_by(|a, b| a.0.cmp(&b.0));

        Self {
            components,
            main_component_type_id: manifest.main_component_type_id.clone(),
            type_table,
            assets_dirs: manifest.assets_dirs.clone(),
            engine_import_path: manifest.engine_import_path.clone(),
        }
    }
}

impl BinaryPaxManifest {
    fn into_manifest(self) -> PaxManifest {
        PaxManifest {
            components: self
                .components
                .into_iter()
                .map(|(type_id, definition)| (type_id, definition.into_component_definition()))
                .collect::<BTreeMap<_, _>>(),
            main_component_type_id: self.main_component_type_id,
            type_table: self.type_table.into_iter().collect::<HashMap<_, _>>(),
            assets_dirs: self.assets_dirs,
            engine_import_path: self.engine_import_path,
        }
    }
}

impl From<&ComponentDefinition> for BinaryComponentDefinition {
    fn from(definition: &ComponentDefinition) -> Self {
        Self {
            type_id: definition.type_id.clone(),
            is_main_component: definition.is_main_component,
            is_primitive: definition.is_primitive,
            is_struct_only_component: definition.is_struct_only_component,
            module_path: definition.module_path.clone(),
            primitive_instance_import_path: definition.primitive_instance_import_path.clone(),
            template: definition
                .template
                .as_ref()
                .map(BinaryComponentTemplate::from),
            settings: definition.settings.clone(),
            timelines: definition.timelines.clone(),
            route_branch: definition.route_branch.clone(),
        }
    }
}

impl BinaryComponentDefinition {
    fn into_component_definition(self) -> ComponentDefinition {
        ComponentDefinition {
            type_id: self.type_id,
            is_main_component: self.is_main_component,
            is_primitive: self.is_primitive,
            is_struct_only_component: self.is_struct_only_component,
            module_path: self.module_path,
            primitive_instance_import_path: self.primitive_instance_import_path,
            template: self
                .template
                .map(BinaryComponentTemplate::into_component_template),
            settings: self.settings,
            timelines: self.timelines,
            route_branch: self.route_branch,
        }
    }
}

impl From<&ComponentTemplate> for BinaryComponentTemplate {
    fn from(template: &ComponentTemplate) -> Self {
        let mut children = template
            .children
            .iter()
            .map(|(id, children)| (id.clone(), children.clone()))
            .collect::<Vec<_>>();
        children.sort_by_key(|(id, _)| id.as_usize());

        let mut nodes = template
            .nodes
            .iter()
            .map(|(id, node)| (id.clone(), node.clone()))
            .collect::<Vec<_>>();
        nodes.sort_by_key(|(id, _)| id.as_usize());

        Self {
            containing_component: template.containing_component.clone(),
            root: template.root.clone(),
            children,
            nodes,
            next_id: template.next_id,
            template_source_file_path: template.template_source_file_path.clone(),
        }
    }
}

impl BinaryComponentTemplate {
    fn into_component_template(self) -> ComponentTemplate {
        ComponentTemplate {
            containing_component: self.containing_component,
            root: self.root,
            children: self.children.into_iter().collect::<HashMap<_, _>>(),
            nodes: self.nodes.into_iter().collect::<HashMap<_, _>>(),
            next_id: self.next_id,
            template_source_file_path: self.template_source_file_path,
        }
    }
}

pub(crate) fn encode_with_header<T: Serialize>(
    magic: &[u8; 8],
    version: u8,
    value: &T,
) -> Result<Vec<u8>> {
    let mut serializer = Serializer::new();
    serializer.output.extend_from_slice(magic);
    serializer.output.push(version);
    value.serialize(&mut serializer)?;
    Ok(serializer.output)
}

pub(crate) fn decode_with_header<T>(bytes: &[u8], magic: &[u8; 8], version: u8) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    if bytes.len() < magic.len() + 1 {
        return Err(Error::message("binary manifest is too short"));
    }
    if &bytes[..magic.len()] != magic {
        return Err(Error::message(
            "binary manifest has an invalid magic header",
        ));
    }
    let actual_version = bytes[magic.len()];
    if actual_version != version {
        return Err(Error::message(format!(
            "unsupported binary manifest version {actual_version}"
        )));
    }

    let mut deserializer = Deserializer::new(&bytes[magic.len() + 1..]);
    let value = T::deserialize(&mut deserializer)?;
    if !deserializer.is_finished() {
        return Err(Error::message("binary manifest has trailing bytes"));
    }
    Ok(value)
}

pub fn to_vec(manifest: &PaxManifest) -> Result<Vec<u8>> {
    let wire_manifest = BinaryPaxManifest::from(manifest);
    encode_with_header(MAGIC, VERSION, &wire_manifest)
}

pub fn from_slice(bytes: &[u8]) -> Result<PaxManifest> {
    let wire_manifest: BinaryPaxManifest = decode_with_header(bytes, MAGIC, VERSION)?;
    Ok(wire_manifest.into_manifest())
}

struct Serializer {
    output: Vec<u8>,
}

impl Serializer {
    fn new() -> Self {
        Self { output: Vec::new() }
    }

    fn write_byte(&mut self, value: u8) {
        self.output.push(value);
    }

    fn write_all(&mut self, bytes: &[u8]) {
        self.output.extend_from_slice(bytes);
    }

    fn write_u64(&mut self, mut value: u64) {
        while value >= 0x80 {
            self.write_byte((value as u8) | 0x80);
            value >>= 7;
        }
        self.write_byte(value as u8);
    }

    fn write_i64(&mut self, value: i64) {
        let encoded = ((value << 1) ^ (value >> 63)) as u64;
        self.write_u64(encoded);
    }

    fn write_len(&mut self, len: Option<usize>) -> Result<()> {
        let len = len.ok_or_else(|| Error::message("binary manifest requires known lengths"))?;
        self.write_u64(len as u64);
        Ok(())
    }
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Compound<'a>;
    type SerializeTuple = Compound<'a>;
    type SerializeTupleStruct = Compound<'a>;
    type SerializeTupleVariant = Compound<'a>;
    type SerializeMap = Compound<'a>;
    type SerializeStruct = Compound<'a>;
    type SerializeStructVariant = Compound<'a>;

    fn serialize_bool(self, value: bool) -> Result<()> {
        self.write_byte(u8::from(value));
        Ok(())
    }

    fn serialize_i8(self, value: i8) -> Result<()> {
        self.write_i64(value as i64);
        Ok(())
    }

    fn serialize_i16(self, value: i16) -> Result<()> {
        self.write_i64(value as i64);
        Ok(())
    }

    fn serialize_i32(self, value: i32) -> Result<()> {
        self.write_i64(value as i64);
        Ok(())
    }

    fn serialize_i64(self, value: i64) -> Result<()> {
        self.write_i64(value);
        Ok(())
    }

    fn serialize_u8(self, value: u8) -> Result<()> {
        self.write_u64(value as u64);
        Ok(())
    }

    fn serialize_u16(self, value: u16) -> Result<()> {
        self.write_u64(value as u64);
        Ok(())
    }

    fn serialize_u32(self, value: u32) -> Result<()> {
        self.write_u64(value as u64);
        Ok(())
    }

    fn serialize_u64(self, value: u64) -> Result<()> {
        self.write_u64(value);
        Ok(())
    }

    fn serialize_f32(self, value: f32) -> Result<()> {
        self.write_all(&value.to_le_bytes());
        Ok(())
    }

    fn serialize_f64(self, value: f64) -> Result<()> {
        self.write_all(&value.to_le_bytes());
        Ok(())
    }

    fn serialize_char(self, value: char) -> Result<()> {
        self.write_u64(value as u32 as u64);
        Ok(())
    }

    fn serialize_str(self, value: &str) -> Result<()> {
        self.write_u64(value.len() as u64);
        self.write_all(value.as_bytes());
        Ok(())
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<()> {
        self.write_u64(value.len() as u64);
        self.write_all(value);
        Ok(())
    }

    fn serialize_none(self) -> Result<()> {
        self.write_byte(0);
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.write_byte(1);
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        self.write_u64(variant_index as u64);
        Ok(())
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.write_u64(variant_index as u64);
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.write_len(len)?;
        Ok(Compound { serializer: self })
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(Compound { serializer: self })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Ok(Compound { serializer: self })
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.write_u64(variant_index as u64);
        Ok(Compound { serializer: self })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        self.write_len(len)?;
        Ok(Compound { serializer: self })
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Ok(Compound { serializer: self })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.write_u64(variant_index as u64);
        Ok(Compound { serializer: self })
    }
}

struct Compound<'a> {
    serializer: &'a mut Serializer,
}

impl<'a> SerializeSeq for Compound<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut *self.serializer)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> SerializeTuple for Compound<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut *self.serializer)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> SerializeTupleStruct for Compound<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut *self.serializer)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> SerializeTupleVariant for Compound<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut *self.serializer)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> SerializeMap for Compound<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        key.serialize(&mut *self.serializer)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut *self.serializer)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> SerializeStruct for Compound<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut *self.serializer)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> SerializeStructVariant for Compound<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut *self.serializer)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

struct Deserializer<'de> {
    input: &'de [u8],
    offset: usize,
}

impl<'de> Deserializer<'de> {
    fn new(input: &'de [u8]) -> Self {
        Self { input, offset: 0 }
    }

    fn is_finished(&self) -> bool {
        self.offset == self.input.len()
    }

    fn read_byte(&mut self) -> Result<u8> {
        let byte = self
            .input
            .get(self.offset)
            .copied()
            .ok_or_else(|| Error::message("unexpected end of binary manifest"))?;
        self.offset += 1;
        Ok(byte)
    }

    fn read_exact(&mut self, len: usize) -> Result<&'de [u8]> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| Error::message("binary manifest offset overflow"))?;
        if end > self.input.len() {
            return Err(Error::message("unexpected end of binary manifest"));
        }
        let bytes = &self.input[self.offset..end];
        self.offset = end;
        Ok(bytes)
    }

    fn read_u64(&mut self) -> Result<u64> {
        let mut value = 0u64;
        let mut shift = 0u32;
        loop {
            let byte = self.read_byte()?;
            value |= ((byte & 0x7f) as u64) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
            shift += 7;
            if shift >= 64 {
                return Err(Error::message("invalid varint in binary manifest"));
            }
        }
    }

    fn read_i64(&mut self) -> Result<i64> {
        let value = self.read_u64()?;
        Ok(((value >> 1) as i64) ^ (-((value & 1) as i64)))
    }

    fn read_len(&mut self) -> Result<usize> {
        let value = self.read_u64()?;
        usize::try_from(value).map_err(|_| Error::message("manifest length overflows usize"))
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::message(
            "binary manifest deserializer is not self-describing",
        ))
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.read_byte()? {
            0 => visitor.visit_bool(false),
            1 => visitor.visit_bool(true),
            other => Err(Error::message(format!("invalid bool tag {other}"))),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.read_i64()?;
        let value = i8::try_from(value).map_err(|_| Error::message("i8 out of range"))?;
        visitor.visit_i8(value)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.read_i64()?;
        let value = i16::try_from(value).map_err(|_| Error::message("i16 out of range"))?;
        visitor.visit_i16(value)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.read_i64()?;
        let value = i32::try_from(value).map_err(|_| Error::message("i32 out of range"))?;
        visitor.visit_i32(value)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.read_i64()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.read_u64()?;
        let value = u8::try_from(value).map_err(|_| Error::message("u8 out of range"))?;
        visitor.visit_u8(value)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.read_u64()?;
        let value = u16::try_from(value).map_err(|_| Error::message("u16 out of range"))?;
        visitor.visit_u16(value)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.read_u64()?;
        let value = u32::try_from(value).map_err(|_| Error::message("u32 out of range"))?;
        visitor.visit_u32(value)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.read_u64()?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(self.read_exact(4)?);
        visitor.visit_f32(f32::from_le_bytes(bytes))
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(self.read_exact(8)?);
        visitor.visit_f64(f64::from_le_bytes(bytes))
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.read_u64()?;
        let value = u32::try_from(value).map_err(|_| Error::message("char out of range"))?;
        let value = char::from_u32(value).ok_or_else(|| Error::message("invalid char scalar"))?;
        visitor.visit_char(value)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let len = self.read_len()?;
        let bytes = self.read_exact(len)?;
        let value = std::str::from_utf8(bytes)
            .map_err(|err| Error::message(format!("invalid UTF-8 string: {err}")))?;
        visitor.visit_borrowed_str(value)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let len = self.read_len()?;
        visitor.visit_borrowed_bytes(self.read_exact(len)?)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let len = self.read_len()?;
        visitor.visit_byte_buf(self.read_exact(len)?.to_vec())
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.read_byte()? {
            0 => visitor.visit_none(),
            1 => visitor.visit_some(self),
            other => Err(Error::message(format!("invalid option tag {other}"))),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let len = self.read_len()?;
        visitor.visit_seq(SequenceAccess {
            deserializer: self,
            remaining: len,
        })
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(SequenceAccess {
            deserializer: self,
            remaining: len,
        })
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let len = self.read_len()?;
        visitor.visit_map(BinaryMapAccess {
            deserializer: self,
            remaining: len,
        })
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(SequenceAccess {
            deserializer: self,
            remaining: fields.len(),
        })
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let variant_index = self.read_u64()?;
        let variant_index =
            u32::try_from(variant_index).map_err(|_| Error::message("enum index out of range"))?;
        visitor.visit_enum(BinaryEnumAccess {
            deserializer: self,
            variant_index,
        })
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u32(visitor)
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::message(
            "binary manifest cannot skip unknown values without type information",
        ))
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.read_i64()?;
        visitor.visit_i128(value as i128)
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = self.read_u64()?;
        visitor.visit_u128(value as u128)
    }
}

struct SequenceAccess<'a, 'de> {
    deserializer: &'a mut Deserializer<'de>,
    remaining: usize,
}

impl<'de, 'a> SeqAccess<'de> for SequenceAccess<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.remaining == 0 {
            return Ok(None);
        }
        self.remaining -= 1;
        seed.deserialize(&mut *self.deserializer).map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.remaining)
    }
}

struct BinaryMapAccess<'a, 'de> {
    deserializer: &'a mut Deserializer<'de>,
    remaining: usize,
}

impl<'de, 'a> MapAccess<'de> for BinaryMapAccess<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.remaining == 0 {
            return Ok(None);
        }
        seed.deserialize(&mut *self.deserializer).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        self.remaining -= 1;
        seed.deserialize(&mut *self.deserializer)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.remaining)
    }
}

struct BinaryEnumAccess<'a, 'de> {
    deserializer: &'a mut Deserializer<'de>,
    variant_index: u32,
}

impl<'de, 'a> EnumAccess<'de> for BinaryEnumAccess<'a, 'de> {
    type Error = Error;
    type Variant = BinaryVariantAccess<'a, 'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(self.variant_index.into_deserializer())?;
        Ok((
            variant,
            BinaryVariantAccess {
                deserializer: self.deserializer,
            },
        ))
    }
}

struct BinaryVariantAccess<'a, 'de> {
    deserializer: &'a mut Deserializer<'de>,
}

impl<'de, 'a> VariantAccess<'de> for BinaryVariantAccess<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.deserializer)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_tuple(self.deserializer, len, visitor)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_tuple(self.deserializer, fields.len(), visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        GradientDefinition, GradientElement, GradientShapeDefinition, GradientStopDefinition,
        PropertyDefinition, SettingElement, Token, TypeTable, ValueDefinition,
    };
    use pax_runtime_api::{Color, Numeric, PaxValue, Size};

    #[test]
    fn binary_manifest_round_trips_template_maps_without_json_keys() {
        let type_id = TypeId::build_singleton("crate::App", Some("App"));
        let mut template = ComponentTemplate::new(type_id.clone(), Some("src/lib.pax".to_string()));

        let mut root = TemplateNodeDefinition::default();
        root.type_id = type_id.clone();
        root.raw_comment_string = Some("root".to_string());
        root.selector_info.class_binding =
            Some(ValueDefinition::LiteralValue(PaxValue::Vec(vec![
                PaxValue::String("card".to_string()),
                PaxValue::String("elevated".to_string()),
            ])));
        root.settings = Some(vec![
            SettingElement::Setting(
                Token::new_without_location("fill".to_string()),
                ValueDefinition::Gradient(GradientDefinition {
                    shape: GradientShapeDefinition::Linear {
                        start: None,
                        end: None,
                    },
                    elements: vec![
                        GradientElement::Stop(GradientStopDefinition {
                            position: Size::Percent(Numeric::F64(0.0)),
                            color: ValueDefinition::LiteralValue(PaxValue::Color(Box::new(
                                Color::RED,
                            ))),
                        }),
                        GradientElement::Stop(GradientStopDefinition {
                            position: Size::Percent(Numeric::F64(100.0)),
                            color: ValueDefinition::LiteralValue(PaxValue::Color(Box::new(
                                Color::BLUE,
                            ))),
                        }),
                    ],
                }),
            ),
            SettingElement::Setting(
                Token::new_without_location("corner_radius".to_string()),
                ValueDefinition::LiteralValue(PaxValue::Vec(vec![
                    PaxValue::Numeric(Numeric::F64(12.0)),
                    PaxValue::Numeric(Numeric::F64(4.0)),
                ])),
            ),
        ]);
        let root_id = template.add(root).template_node_id;

        let mut child = TemplateNodeDefinition::default();
        child.type_id = TypeId::build_comment();
        child.raw_comment_string = Some("child".to_string());
        template.add_child(root_id, child);

        let mut components = BTreeMap::new();
        components.insert(
            type_id.clone(),
            ComponentDefinition {
                type_id: type_id.clone(),
                is_main_component: true,
                is_primitive: false,
                is_struct_only_component: false,
                module_path: "crate".to_string(),
                primitive_instance_import_path: None,
                template: Some(template),
                settings: None,
                timelines: vec![],
                route_branch: None,
            },
        );

        let mut type_table = TypeTable::default();
        type_table.insert(
            type_id.clone(),
            TypeDefinition {
                type_id: type_id.clone(),
                inner_iterable_type_id: None,
                property_definitions: vec![PropertyDefinition::primitive_with_name(
                    "String", "label",
                )],
            },
        );

        let manifest = PaxManifest {
            components,
            main_component_type_id: type_id.clone(),
            type_table,
            assets_dirs: vec!["assets".to_string()],
            engine_import_path: "pax_kit::pax_engine".to_string(),
        };

        let encoded = to_vec(&manifest).expect("manifest should encode");
        let decoded = from_slice(&encoded).expect("manifest should decode");
        let decoded_component = decoded.components.get(&type_id).unwrap();
        let decoded_template = decoded_component.template.as_ref().unwrap();

        #[cfg(feature = "json")]
        serde_json::to_value(&decoded).expect("decoded manifest should serialize to json");

        assert_eq!(
            decoded.main_component_type_id,
            manifest.main_component_type_id
        );
        assert_eq!(decoded.type_table.len(), manifest.type_table.len());
        assert_eq!(decoded.assets_dirs, manifest.assets_dirs);
        assert_eq!(decoded_template.get_root().len(), 1);
        assert_eq!(
            decoded_template
                .get_children(&TemplateNodeId::build(0))
                .unwrap()
                .len(),
            1
        );
        let root = decoded_template
            .get_node(&TemplateNodeId::build(0))
            .expect("root should round-trip");
        assert!(matches!(
            root.settings
                .as_ref()
                .and_then(|settings| settings.iter().find(|setting| {
                    matches!(setting, SettingElement::Setting(token, _) if token.token_value == "fill")
                })),
            Some(SettingElement::Setting(_, ValueDefinition::Gradient(gradient)))
                if matches!(&gradient.shape, GradientShapeDefinition::Linear { start: None, end: None })
                    && gradient.stops().count() == 2
        ));
        assert!(matches!(
            root.settings.as_ref().and_then(|settings| settings.iter().find(|setting| {
                matches!(setting, SettingElement::Setting(token, _) if token.token_value == "corner_radius")
            })),
            Some(SettingElement::Setting(
                _,
                ValueDefinition::LiteralValue(PaxValue::Vec(values))
            )) if values == &vec![
                PaxValue::Numeric(Numeric::F64(12.0)),
                PaxValue::Numeric(Numeric::F64(4.0)),
            ]
        ));
        let classes = root.selector_info.literal_classes();
        assert_eq!(classes, vec!["card", "elevated"]);
    }
}
