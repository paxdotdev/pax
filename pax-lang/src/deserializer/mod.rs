use crate::{Parser, PaxParser, Rule};
use helpers::PaxNumeric;
use pax_runtime_api::PaxValue;
use pest::iterators::Pair;
use serde::de::{self, Visitor};
use serde::{forward_to_deserialize_any, Deserialize};

pub mod error;
mod helpers;
mod tests;

use self::helpers::{PaxEnum, PaxObject, PaxSeq};

pub use error::{Error, Result};

use pax_runtime_api::constants::{
    COLOR, DEGREES, NUMERIC, PERCENT, PIXELS, RADIANS, ROTATION, SIZE, INTEGER
};

const STRING: &str = "String";
const BOOL: &str = "Bool";
const OPTION: &str = "Option";
const VEC: &str = "Vec";
const ENUM: &str = "Enum";
const OBJECT: &str = "Object";

pub fn from_pax(str: &str) -> Result<PaxValue> {
    let ast = if let Ok(mut ast) = PaxParser::parse(Rule::literal_value, &str) {
        ast.next().unwrap()
    } else if let Ok(mut ast) = PaxParser::parse(Rule::literal_object, &str) {
        ast.next().unwrap()
    } else {
        return Err(Error::Message(format!("Could not parse: {}", str)));
    };
    let deserializer: PaxDeserializer = PaxDeserializer::from(ast);
    let t = PaxValue::deserialize(deserializer)?;
    Ok(t)
}

pub fn from_pax_ast(ast: Pair<Rule>) -> Result<PaxValue> {
    let deserializer: PaxDeserializer = PaxDeserializer::from(ast);
    let t = PaxValue::deserialize(deserializer)?;
    Ok(t)
}

pub struct PaxDeserializer<'de> {
    pub ast: Pair<'de, Rule>,
}

impl<'de> PaxDeserializer<'de> {
    pub fn from(ast: Pair<'de, Rule>) -> Self {
        PaxDeserializer { ast }
    }

    fn deserialize_pax_value<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
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
                let unit = self.ast.clone().into_inner().nth(1).unwrap().as_str();
                match unit {
                    "%" => visitor.visit_enum(PaxEnum::new_pax_value(PERCENT, Some(self.ast))),
                    "px" => visitor.visit_enum(PaxEnum::new_pax_value(SIZE, Some(self.ast))),
                    "rad" => visitor.visit_enum(PaxEnum::new_pax_value(ROTATION, Some(self.ast))),
                    "deg" => visitor.visit_enum(PaxEnum::new_pax_value(ROTATION, Some(self.ast))),
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
            Rule::literal_boolean => {
                visitor.visit_enum(PaxEnum::new(BOOL, Some(self.ast)))
            }
            Rule::literal_object => visitor.visit_enum(PaxEnum::new_pax_value(OBJECT, Some(self.ast))),
            _ => Err(Error::UnsupportedType(self.ast.to_string())),
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
                        visitor.visit_enum(PaxEnum::new(INTEGER, Some(channel)))
                    },
                    Rule::literal_number_with_unit => {
                        let unit = channel.clone().into_inner().nth(1).unwrap().as_str();
                        let number = channel.clone().into_inner().next().unwrap();
                        match unit {
                            "%" => visitor.visit_enum(PaxEnum::new(PERCENT, Some(number))),
                            "rad" => visitor.visit_enum(PaxEnum::new(ROTATION, Some(channel))),
                            "deg" => visitor.visit_enum(PaxEnum::new(ROTATION, Some(channel))),
                            _ => {
                                Err(Error::Message(format!("Unsupported unit: {} for ColorChannel", unit)))
                            }
                        }
                    }
                    _ => {
                        Err(Error::Message(format!("Unsupported type: {} for ColorChannel", channel.as_str())))
                    }
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
                let unit = inner.clone().nth(1).unwrap().as_str();
                match unit {
                    "%" => visitor.visit_newtype_struct(PaxDeserializer::from(number.unwrap())),
                    "px" => visitor.visit_enum(PaxEnum::new(PIXELS, number)),
                    "rad" => visitor.visit_enum(PaxEnum::new(RADIANS, number)),
                    "deg" => visitor.visit_enum(PaxEnum::new(DEGREES, number)),
                    _ => {
                        Err(Error::Message(format!("Unsupported unit: {}", unit)))
                    }
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
            },
            Rule::identifier | Rule::pascal_identifier => {
                visitor.visit_str(self.ast.as_str())
            }
            _ => Err(Error::UnsupportedType(self.ast.to_string())),
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
        bytes byte_buf unit unit_struct newtype_struct tuple identifier
        tuple_struct struct ignored_any
    }
    
    fn deserialize_enum<V>(
        self,
        name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
        if name == "PaxValue" {
            return self.deserialize_pax_value(visitor);
        } 
        self.deserialize_any(visitor)
    }
    
    fn deserialize_seq<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
            visitor.visit_seq(PaxSeq::new(self.ast.into_inner()))
    }
    
    fn deserialize_option<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
        let unwrapped_option = self.ast.into_inner().next().unwrap();
        match unwrapped_option.as_rule() {
            Rule::literal_none => visitor.visit_none(),
            Rule::literal_some => visitor.visit_some(PaxDeserializer::from(unwrapped_option.into_inner().next().unwrap())),
            _ => Err(Error::Message(format!("Unexpected format for Option: {}", unwrapped_option.as_str())))
        }
    }
    
    fn deserialize_map<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: Visitor<'de> {
        visitor.visit_map(PaxObject::new(self.ast.into_inner()))
    }

    
}