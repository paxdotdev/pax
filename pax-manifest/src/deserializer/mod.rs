use core::panic;
use pest::Parser;
use serde::de::{self, DeserializeOwned, Visitor};
use serde::forward_to_deserialize_any;

pub mod error;
mod helpers;
mod tests;

use self::helpers::{PaxEnum, PaxObject, PaxSeq};

pub use error::{Error, Result};

use crate::utils::{PaxParser, Rule};

use crate::constants::{
    DEGREES, FLOAT, INTEGER, NUMERIC, PERCENT, PIXELS, RADIANS, ROTATION, SIZE, STRING_BOX, TRUE, COLOR
};

use pax_runtime_api::Color;

pub struct Deserializer {
    input: String,
}

impl Deserializer {
    pub fn from_string(input: String) -> Self {
        Deserializer { input }
    }
}

/// Main entry-point for deserializing a type from Pax.
pub fn from_pax<T>(str: String) -> Result<T>
where
    T: DeserializeOwned,
{
    let deserializer: Deserializer = Deserializer::from_string(str.trim().to_string());
    let t = T::deserialize(deserializer)?;
    Ok(t)
}

fn color_visitor<'de, V>(visitor: V) -> V
    where
        V: Visitor<'de, Value = Color>,
{
    visitor
}

impl<'de> de::Deserializer<'de> for Deserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let ast = if let Ok(ast) = PaxParser::parse(Rule::literal_value, &self.input) {
            ast.clone().next().unwrap()
        } else if let Ok(ast) = PaxParser::parse(Rule::literal_object, &self.input) {
            ast.clone().next().unwrap()
        } else if let Ok(_) = PaxParser::parse(Rule::identifier, &self.input) {
            return self.deserialize_identifier(visitor);
        } else {
            panic!("Failed to parse: {}", &self.input)
        };

        let ret = match ast.as_rule() {
            Rule::literal_value => {
                let inner_pair = ast.clone().into_inner().next().unwrap();
                match inner_pair.as_rule() {
                    Rule::literal_color => {



                        // literal_color = {literal_color_space_func | literal_color_const}
                        let what_kind_of_color = inner_pair.into_inner().next().unwrap();
                        match what_kind_of_color.as_rule() {
                            Rule::literal_color_space_func => {
                                let mut lcsf_pairs = what_kind_of_color.into_inner();
                                let func = lcsf_pairs.next().unwrap().as_str().to_string().replace("(", "");
                                let args = if func == "rgb" || func == "hsl" {
                                    //three args
                                    vec![lcsf_pairs.next().unwrap().as_str().to_string(),lcsf_pairs.next().unwrap().as_str().to_string(),lcsf_pairs.next().unwrap().as_str().to_string()].join(",")
                                } else {
                                    //four args
                                    vec![lcsf_pairs.next().unwrap().as_str().to_string(),lcsf_pairs.next().unwrap().as_str().to_string(),lcsf_pairs.next().unwrap().as_str().to_string(),lcsf_pairs.next().unwrap().as_str().to_string()].join(",")
                                };

                                let explicit_color = visitor.visit_enum(PaxEnum::new(
                                    COLOR.to_string(),
                                    func.as_str().to_string(),
                                    Some(args)
                                ));
                                explicit_color
                            }
                            Rule::literal_color_const => {
                                let color_const = what_kind_of_color.into_inner().next().unwrap();

                                let explicit_color = visitor.visit_enum(PaxEnum::new(
                                    COLOR.to_string(),
                                    color_const.to_string(),
                                    None
                                ));
                                explicit_color
                            },
                            _ => {unreachable!()}
                        }
                    }
                    Rule::literal_number => {
                        let number = inner_pair.into_inner().next().unwrap();
                        match number.as_rule() {
                            Rule::literal_number_integer => visitor.visit_enum(PaxEnum::new(
                                NUMERIC.to_string(),
                                INTEGER.to_string(),
                                Some(number.as_str().to_string()),
                            )),
                            Rule::literal_number_float => visitor.visit_enum(PaxEnum::new(
                                NUMERIC.to_string(),
                                FLOAT.to_string(),
                                Some(number.as_str().to_string()),
                            )),
                            _ => Err(Error::UnsupportedType(number.as_str().to_string())),
                        }
                    }
                    Rule::literal_number_with_unit => {
                        let inner = inner_pair.into_inner();
                        let number = inner.clone().next().unwrap().as_str();
                        let unit = inner.clone().nth(1).unwrap().as_str();
                        match unit {
                            "%" => visitor.visit_enum(PaxEnum::new(
                                SIZE.to_string(),
                                PERCENT.to_string(),
                                Some(number.to_string()),
                            )),
                            "px" => visitor.visit_enum(PaxEnum::new(
                                SIZE.to_string(),
                                PIXELS.to_string(),
                                Some(number.to_string()),
                            )),
                            "rad" => visitor.visit_enum(PaxEnum::new(
                                ROTATION.to_string(),
                                RADIANS.to_string(),
                                Some(number.to_string()),
                            )),
                            "deg" => visitor.visit_enum(PaxEnum::new(
                                ROTATION.to_string(),
                                DEGREES.to_string(),
                                Some(number.to_string()),
                            )),
                            _ => {
                                unreachable!("Unsupported unit: {}", unit)
                            }
                        }
                    }
                    Rule::string => {
                        let string_within_quotes =
                            inner_pair.into_inner().next().unwrap().as_str().to_string();
                        visitor.visit_map(PaxObject::new(
                            Some(STRING_BOX.to_string()),
                            vec![("string".to_string(), string_within_quotes)],
                        ))
                    }
                    Rule::literal_tuple => {
                        let pairs = inner_pair.into_inner();
                        let elements = pairs
                            .map(|pair| pair.as_str().to_string())
                            .collect::<Vec<String>>();
                        visitor.visit_seq(PaxSeq::new(elements))
                    }
                    Rule::literal_enum_value => {
                        visitor.visit_enum(PaxEnum::from_string(inner_pair.as_str().to_string()))
                    }
                    Rule::literal_boolean => visitor.visit_bool(inner_pair.as_str() == TRUE),
                    _ => Err(Error::UnsupportedType(inner_pair.as_str().to_string())),
                }
            }
            Rule::literal_object => {
                visitor.visit_map(PaxObject::from_string(ast.as_str().to_string()))
            }
            _ => Err(Error::UnsupportedType(ast.as_str().to_string())),
        }?;

        Ok(ret.into())
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum ignored_any
    }

    fn deserialize_identifier<V>(
        self,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(&self.input)
    }
}
