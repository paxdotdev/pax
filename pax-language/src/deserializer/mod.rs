use crate::{Parser, PaxParser, Rule};
use helpers::PaxNumeric;
use pax_runtime_api::PaxValue;
use pest::iterators::Pair;
use serde::de::{self, Visitor};
use serde::{forward_to_deserialize_any, Deserialize};

pub mod error;
mod helpers;
mod tests;

use self::helpers::{ColorChannelInteger, PaxEnum, PaxObject, PaxSeq};

pub use error::{Error, Result};

use pax_runtime_api::constants::{
    COLOR, DEGREES, DURATION, FRAMES, MILLISECONDS, NUMERIC, PERCENT, PIXELS, RADIANS, ROTATION,
    SECONDS, SIZE,
};

const STRING: &str = "String";
const BOOL: &str = "Bool";
const OPTION: &str = "Option";
const VEC: &str = "Vec";
const ENUM: &str = "Enum";
const OBJECT: &str = "Object";

/// Deserialize a Pax literal string into a runtime `PaxValue`.
pub fn from_pax(str: &str) -> Result<PaxValue> {
    let ast = if let Ok(mut ast) = PaxParser::parse(Rule::literal_value, &str) {
        ast.next().unwrap()
    } else {
        return Err(Error::Message(format!("Could not parse: {}", str)));
    };
    from_pax_ast(ast)
}

/// Deserialize a parsed literal AST node into a runtime `PaxValue`.
pub fn from_pax_ast(ast: Pair<Rule>) -> Result<PaxValue> {
    let ast = if matches!(
        ast.as_rule(),
        Rule::literal_value | Rule::template_list_value | Rule::settings_value
    ) {
        ast.into_inner().next().unwrap()
    } else {
        ast
    };
    if matches!(ast.as_rule(), Rule::literal_list | Rule::literal_tuple) {
        return ast
            .into_inner()
            .map(from_pax_ast)
            .collect::<Result<Vec<_>>>()
            .map(PaxValue::Vec);
    }
    if ast.as_rule() == Rule::literal_object {
        let mut fields = ast.clone().into_inner();
        let name = if fields
            .peek()
            .is_some_and(|v| v.as_rule() == Rule::pascal_identifier)
        {
            Some(fields.next().unwrap().as_str().to_string())
        } else {
            None
        };
        if name
            .as_deref()
            .is_some_and(|name| name == "Fill" || name == "Stroke")
        {
            let mut values = Vec::new();
            for field in fields {
                if field.as_rule() != Rule::settings_key_value_pair {
                    continue;
                }
                let mut field = field.into_inner();
                let key = field
                    .next()
                    .unwrap()
                    .into_inner()
                    .next()
                    .unwrap()
                    .as_str()
                    .to_string();
                values.push((key, from_pax_ast(field.next().unwrap())?));
            }
            return Ok(PaxValue::Enum(Box::new((
                name.unwrap(),
                "__layer".into(),
                vec![PaxValue::Object(values)],
            ))));
        }
    }
    let deserializer: PaxDeserializer = PaxDeserializer::from(ast);
    let t = PaxValue::deserialize(deserializer)?;
    Ok(t)
}

/// Serde bridge from Pax literal grammar nodes into runtime values.
pub struct PaxDeserializer<'de> {
    pub ast: Pair<'de, Rule>,
}

impl<'de> PaxDeserializer<'de> {
    /// Wrap a pest pair as a deserializer.
    pub fn from(ast: Pair<'de, Rule>) -> Self {
        PaxDeserializer { ast }
    }

    fn deserialize_pax_value<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Flatten settings_value to internal typed rules
        if matches!(
            self.ast.as_rule(),
            Rule::settings_value | Rule::template_list_value
        ) {
            self.ast = self.ast.into_inner().next().unwrap();
        }

        // Flatten literal_value to internal typed rules
        if let Rule::literal_value = self.ast.as_rule() {
            self.ast = self.ast.into_inner().next().unwrap();
        }

        match self.ast.as_rule() {
            Rule::literal_color => {
                visitor.visit_enum(PaxEnum::new_pax_value(COLOR, Some(self.ast)))
            }
            Rule::literal_number => {
                visitor.visit_enum(PaxEnum::new_pax_value(NUMERIC, Some(self.ast)))
            }
            Rule::literal_number_with_unit => {
                let unit = self
                    .ast
                    .clone()
                    .into_inner()
                    .nth(1)
                    .unwrap()
                    .as_str()
                    .trim();
                match unit {
                    "%" => visitor.visit_enum(PaxEnum::new_pax_value(PERCENT, Some(self.ast))),
                    "px" => visitor.visit_enum(PaxEnum::new_pax_value(SIZE, Some(self.ast))),
                    "rad" => visitor.visit_enum(PaxEnum::new_pax_value(ROTATION, Some(self.ast))),
                    "deg" => visitor.visit_enum(PaxEnum::new_pax_value(ROTATION, Some(self.ast))),
                    "ms" | "s" | "f" => {
                        visitor.visit_enum(PaxEnum::new_pax_value(DURATION, Some(self.ast)))
                    }
                    _ => {
                        unreachable!("Unsupported unit: {}", unit)
                    }
                }
            }
            Rule::string => visitor.visit_enum(PaxEnum::new_pax_value(STRING, Some(self.ast))),
            Rule::literal_list | Rule::literal_tuple => {
                visitor.visit_enum(PaxEnum::new_pax_value(VEC, Some(self.ast)))
            }
            Rule::literal_enum_value => {
                visitor.visit_enum(PaxEnum::new_pax_value(ENUM, Some(self.ast)))
            }
            Rule::literal_option => {
                visitor.visit_enum(PaxEnum::new_pax_value(OPTION, Some(self.ast)))
            }
            Rule::literal_boolean => visitor.visit_enum(PaxEnum::new(BOOL, Some(self.ast))),
            Rule::literal_object => {
                visitor.visit_enum(PaxEnum::new_pax_value(OBJECT, Some(self.ast)))
            }
            _ => Err(Error::UnsupportedType(format!(
                "Rule : {:?}, raw_string: {}",
                self.ast.as_rule(),
                self.ast.as_str().to_string()
            ))),
        }
    }

    fn deserialize_builtin<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let ret = match self.ast.as_rule() {
            Rule::literal_color => {
                // literal_color = {literal_color_space_func | literal_color_const}
                let what_kind_of_color = self.ast.into_inner().next().unwrap();
                match what_kind_of_color.as_rule() {
                    Rule::literal_color_space_func => {
                        let func = what_kind_of_color
                            .as_str()
                            .trim()
                            .split("(")
                            .next()
                            .unwrap();

                        let color_args = Some(what_kind_of_color);

                        visitor.visit_enum(PaxEnum::new(func, color_args))
                    }
                    Rule::literal_color_const => {
                        let explicit_color =
                            visitor.visit_enum(PaxEnum::new(what_kind_of_color.as_str(), None));
                        explicit_color
                    }
                    _ => {
                        unreachable!()
                    }
                }
            }
            Rule::literal_color_channel => {
                let channel = self.ast.into_inner().next().unwrap();
                match channel.as_rule() {
                    Rule::literal_number_integer => {
                        visitor.visit_enum(ColorChannelInteger(channel.as_str().parse().unwrap()))
                    }
                    Rule::literal_number_with_unit => {
                        let unit = channel.clone().into_inner().nth(1).unwrap().as_str().trim();
                        let number = channel.clone().into_inner().next().unwrap();
                        match unit {
                            "%" => visitor.visit_enum(PaxEnum::new(PERCENT, Some(number))),
                            "rad" => visitor.visit_enum(PaxEnum::new(ROTATION, Some(channel))),
                            "deg" => visitor.visit_enum(PaxEnum::new(ROTATION, Some(channel))),
                            _ => Err(Error::Message(format!(
                                "Unsupported unit: {} for ColorChannel",
                                unit
                            ))),
                        }
                    }
                    _ => Err(Error::Message(format!(
                        "Unsupported type: {} for ColorChannel",
                        channel.as_str()
                    ))),
                }
            }
            Rule::literal_number => {
                let number = self.ast.into_inner().next().unwrap();
                visitor.visit_enum(PaxNumeric::new(number, true))
            }
            Rule::literal_number_integer | Rule::literal_number_float => {
                visitor.visit_enum(PaxNumeric::new(self.ast, true))
            }
            Rule::literal_number_with_unit => {
                let inner = self.ast.into_inner();
                let number = inner.clone().next();
                let unit = inner.clone().nth(1).unwrap().as_str().trim();
                match unit {
                    "%" => visitor.visit_newtype_struct(PaxDeserializer::from(number.unwrap())),
                    "px" => visitor.visit_enum(PaxEnum::new(PIXELS, number)),
                    "rad" => visitor.visit_enum(PaxEnum::new(RADIANS, number)),
                    "deg" => visitor.visit_enum(PaxEnum::new(DEGREES, number)),
                    "ms" => visitor.visit_enum(PaxEnum::new(MILLISECONDS, number)),
                    "s" => visitor.visit_enum(PaxEnum::new(SECONDS, number)),
                    "f" => visitor.visit_enum(PaxEnum::new(FRAMES, number)),
                    _ => Err(Error::Message(format!("Unsupported unit: {}", unit))),
                }
            }
            Rule::string => {
                let string_within_quotes =
                    self.ast.into_inner().next().unwrap().as_str().to_string();
                visitor.visit_string(string_within_quotes)
            }
            Rule::literal_boolean => {
                let bool_str = self.ast.as_str();
                visitor.visit_bool(bool_str.parse::<bool>().unwrap())
            }
            Rule::identifier | Rule::pascal_identifier => visitor.visit_str(self.ast.as_str()),
            Rule::settings_key => visitor.visit_str(self.ast.into_inner().next().unwrap().as_str()),
            _ => Err(Error::UnsupportedType(self.ast.as_str().to_string())),
        }?;

        Ok(ret)
    }
}

impl<'de> de::Deserializer<'de> for PaxDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_builtin(visitor)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct newtype_struct identifier
        tuple_struct struct ignored_any
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if name == "PaxValue" {
            return self.deserialize_pax_value(visitor);
        }
        self.deserialize_any(visitor)
    }

    fn deserialize_seq<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let rule = self.ast.as_rule();
        if rule == Rule::literal_object {
            visitor.visit_seq(PaxObject::new(self.ast.into_inner()))
        } else {
            visitor.visit_seq(PaxSeq::new(self.ast.into_inner()))
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let unwrapped_option = self.ast.into_inner().next().unwrap();
        match unwrapped_option.as_rule() {
            Rule::literal_none => visitor.visit_none(),
            Rule::literal_some => visitor.visit_some(PaxDeserializer::from(
                unwrapped_option.into_inner().next().unwrap(),
            )),
            _ => Err(Error::Message(format!(
                "Unexpected format for Option: {}",
                unwrapped_option.as_str()
            ))),
        }
    }

    fn deserialize_map<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(PaxObject::new(self.ast.into_inner()))
    }

    fn deserialize_tuple<V>(
        self,
        _len: usize,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(PaxSeq::new(self.ast.into_inner()))
    }
}
