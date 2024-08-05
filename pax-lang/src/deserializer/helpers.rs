use pest::iterators::{Pair, Pairs};
use serde::{
    de::{self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor},
    forward_to_deserialize_any,
};

use crate::Rule;

use super::{
    error::{Error, Result},
    PaxDeserializer,
};

pub struct StringDeserializer<'de> {
    input: &'de str,
}

impl<'de> StringDeserializer<'de> {
    pub fn new(input: &'de str) -> Self {
        StringDeserializer { input }
    }
}

impl<'de> de::Deserializer<'de> for StringDeserializer<'de> {
    type Error = Error;

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple identifier
        tuple_struct map struct enum ignored_any
    }

    fn deserialize_any<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(&self.input)
    }
}

#[derive(Clone)]
pub struct PaxEnum<'de> {
    variant: &'de str,
    args: Option<Pair<'de, Rule>>,
    is_pax_value: bool,
}

impl<'de> PaxEnum<'de> {
    pub fn new(variant: &'de str, args: Option<Pair<'de, Rule>>) -> Self {
        PaxEnum {
            variant,
            args,
            is_pax_value: false,
        }
    }
    pub fn new_pax_value(variant: &'de str, args: Option<Pair<'de, Rule>>) -> Self {
        PaxEnum {
            variant,
            args,
            is_pax_value: true,
        }
    }
}

impl<'de> EnumAccess<'de> for PaxEnum<'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let val = seed.deserialize(StringDeserializer::new(&self.variant))?;
        Ok((val, self))
    }
}

impl<'de> VariantAccess<'de> for PaxEnum<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        let ast = self.args.unwrap();
        seed.deserialize(PaxDeserializer::from(ast))
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.is_pax_value && self.variant == "Enum" {
            let mut enum_pairs = self.args.unwrap().into_inner();
            while enum_pairs.len() > 2 {
                enum_pairs.next();
            }
            return visitor.visit_seq(PaxSeq::new(enum_pairs));
        }

        if let Some(args) = self.args {
            visitor.visit_seq(PaxSeq::new(args.into_inner()))
        } else {
            return Err(Error::Message("Expected enum arguments".to_string()));
        }
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::Message(
            "Enums with struct arguments are not supported".to_string(),
        ))
    }
}

pub struct PaxSeq<'de> {
    elements: Pairs<'de, Rule>,
    index: usize,
}

impl<'de> PaxSeq<'de> {
    pub fn new(elements: Pairs<'de, Rule>) -> Self {
        PaxSeq { elements, index: 0 }
    }
}

impl<'de> SeqAccess<'de> for PaxSeq<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if let Some(elem) = self.elements.next() {
            let val = seed.deserialize(PaxDeserializer::from(elem))?;
            self.index += 1;
            Ok(Some(val))
        } else {
            Ok(None)
        }
    }
}

pub struct PaxObject<'de> {
    #[allow(unused)]
    elements: Pairs<'de, Rule>,
    current_value: Option<Pair<'de, Rule>>,
}

impl<'de> PaxObject<'de> {
    pub fn new(mut elements: Pairs<'de, Rule>) -> Self {
        if let Some(first) = elements.peek() {
            if first.as_rule() == Rule::pascal_identifier {
                elements.next();
            }
        }
        PaxObject {
            elements,
            current_value: None,
        }
    }
}

impl<'de> MapAccess<'de> for PaxObject<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        while let Some(pair) = self.elements.next() {
            match pair.as_rule() {
                Rule::settings_key_value_pair => {
                    let mut pairs = pair.into_inner();
                    let key = pairs.next().unwrap().into_inner().next().unwrap();
                    self.current_value = Some(pairs.next().unwrap());
                    return Ok(Some(
                        seed.deserialize(StringDeserializer::new(key.as_str()))?,
                    ));
                }
                Rule::comment => {
                    continue;
                }
                _ => {
                    return Err(Error::Message(format!(
                        "Object should not include {:?}",
                        pair.as_rule()
                    )));
                }
            }
        }
        Ok(None)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        if let Some(settings_value) = self.current_value.take() {
            let inner = settings_value.into_inner().next().unwrap();
            return seed.deserialize(PaxDeserializer::from(inner));
        } else {
            return Err(Error::Message(
                "Did not find corresponding value for a key".to_string(),
            ));
        }
    }
}

pub struct PaxNumeric<'de> {
    ast: Pair<'de, Rule>,
    wrap_in_enum: bool,
}

impl<'de> PaxNumeric<'de> {
    pub fn new(ast: Pair<'de, Rule>, wrap_in_enum: bool) -> Self {
        PaxNumeric { ast, wrap_in_enum }
    }
}

impl<'de> EnumAccess<'de> for PaxNumeric<'de> {
    type Error = Error;

    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> std::result::Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let variant_name = match self.ast.as_rule() {
            Rule::literal_number_float => "F64",
            Rule::literal_number_integer => "I64",
            _ => return Err(Error::Message("Unsupported numeric type".to_string())),
        };
        let val = seed.deserialize(StringDeserializer::new(variant_name))?;
        Ok((val, self))
    }
}
impl<'de> VariantAccess<'de> for PaxNumeric<'de> {
    type Error = Error;

    fn unit_variant(self) -> std::result::Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(mut self, seed: T) -> std::result::Result<T::Value, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        self.wrap_in_enum = false;
        seed.deserialize(self)
    }

    fn tuple_variant<V>(
        self,
        _len: usize,
        _visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unreachable!("Numeric is not a tuple")
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        unreachable!("Numeric is not a struct")
    }
}

impl<'de> de::Deserializer<'de> for PaxNumeric<'de> {
    type Error = Error;

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple identifier
        tuple_struct map struct enum ignored_any
    }

    fn deserialize_any<V>(self, visitor: V) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.wrap_in_enum {
            return visitor.visit_enum(self);
        }

        match self.ast.as_rule() {
            Rule::literal_number_float => {
                visitor.visit_f64(self.ast.as_str().trim().parse::<f64>().unwrap())
            }
            Rule::literal_number_integer => {
                visitor.visit_i64(self.ast.as_str().trim().parse::<i64>().unwrap())
            }
            _ => Err(Error::Message("Unsupported numeric type".to_string())),
        }
    }
}
