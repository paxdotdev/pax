use core::panic;
use pax_lang::{Parser, PaxParser, Rule};
use pax_runtime_api::constants::I64;
use pax_runtime_api::pax_value::{CoercionRules, PaxAny, ToFromPaxAny};
use pax_runtime_api::{Color, Numeric, Percent};
use serde::de::{self, DeserializeOwned, Visitor};
use serde::forward_to_deserialize_any;
use std::any::TypeId;

pub mod error;
mod helpers;
mod tests;

use self::helpers::{PaxColor, PaxEnum, PaxObject, PaxSeq};

pub use error::{Error, Result};

use crate::constants::{
    COLOR, DEGREES, F64, NUMERIC, PERCENT, PIXELS, RADIANS, ROTATION, SIZE, STRING_BOX, TRUE,
};

use crate::deserializer::helpers::{ColorFuncArg, PaxSeqArg};
use std::cell::RefCell;
use std::collections::HashMap;

pub struct Deserializer {
    input: String,
}

impl Deserializer {
    pub fn from_string(input: String) -> Self {
        Deserializer { input }
    }
}
thread_local! {
    static CACHED_VALUES : RefCell<HashMap<String, PaxAny>> = RefCell::new(HashMap::new());
}

/// Given type information T, this coerces the value of the PaxAny into the expected
/// type if able, or returns an error
pub fn from_pax_try_coerce<T: ToFromPaxAny + CoercionRules + Clone + 'static>(
    str: &str,
) -> std::result::Result<PaxAny, String>
where
    T: DeserializeOwned,
{
    from_pax::<T>(str)
        .map_err(|e| format!("failed to deserialize: {:?}", e))
        .and_then(|v| v.try_coerce::<T>())
}

fn from_pax<T: ToFromPaxAny + CoercionRules + Clone + 'static>(str: &str) -> Result<PaxAny>
where
    T: DeserializeOwned,
{
    if let Some(cached) = CACHED_VALUES.with(|cache| {
        let cache = cache.borrow();
        let option_cached_dyn_any = cache.get(str);
        // down cast val to T
        if let Some(data) = &option_cached_dyn_any {
            return Some(data.try_clone::<T>().unwrap());
        }
        None
    }) {
        return Ok(cached);
    }

    let type_id = TypeId::of::<T>();
    if type_id != TypeId::of::<Color>()
        && type_id != TypeId::of::<Percent>()
        && type_id != TypeId::of::<Numeric>()
    {
        let ret = if let Ok(_ast) = PaxParser::parse(Rule::literal_color, str) {
            Ok(from_pax::<Color>(str).unwrap())
        } else if let Ok(ast) = PaxParser::parse(Rule::literal_number_with_unit, str) {
            let _number = ast.clone().next().unwrap().as_str();
            let unit = ast.clone().next().unwrap().as_str();
            match unit {
                "%" => Ok(from_pax::<Percent>(&str).unwrap()),
                _ => Err(Error::UnsupportedMethod),
            }
        } else if let Ok(_ast) = PaxParser::parse(Rule::literal_number, str) {
            Ok(from_pax::<Numeric>(str).unwrap())
        } else {
            Err(Error::UnsupportedMethod)
        };
        if let Ok(val) = ret {
            return Ok(val);
        }
    }

    let deserializer: Deserializer = Deserializer::from_string(str.trim().to_string());
    let t = T::deserialize(deserializer)?;

    CACHED_VALUES.with(|cache| {
        cache
            .borrow_mut()
            .insert(str.to_string(), t.clone().to_pax_any());
    });

    Ok(t.to_pax_any())
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
            return Err(Error::UnsupportedType(self.input));
        };

        let ret = match ast.as_rule() {
            Rule::literal_value => {
                let inner_pair = ast.clone().into_inner().next().unwrap();
                match inner_pair.as_rule() {
                    Rule::literal_color => {
                        // literal_color = {literal_color_space_func | literal_color_const}
                        let what_kind_of_color = inner_pair.clone().into_inner().next().unwrap();
                        match what_kind_of_color.as_rule() {
                            Rule::literal_color_space_func => {
                                let lcsf_pairs =
                                    inner_pair.clone().into_inner().next().unwrap().into_inner();
                                let func = inner_pair
                                    .clone()
                                    .into_inner()
                                    .next()
                                    .unwrap()
                                    .as_str()
                                    .to_string()
                                    .trim()
                                    .to_string()
                                    .split("(")
                                    .next()
                                    .unwrap()
                                    .to_string();

                                // pre-process each lcsf_pair and wrap into a ColorChannelDefinition

                                //literal_color_channel = {literal_number_with_unit | literal_number_integer}
                                let args = lcsf_pairs
                                    .into_iter()
                                    .map(|lcsf| {
                                        let lcsf = lcsf.into_inner().next().unwrap();
                                        match lcsf.as_rule() {
                                            Rule::literal_number_with_unit => {
                                                let inner = lcsf.clone().into_inner();
                                                let number = inner.clone().next().unwrap().as_str();
                                                let unit = inner.clone().nth(1).unwrap().as_str();
                                                match unit {
                                                    "%" => {
                                                        ColorFuncArg::Percent(number.to_string())
                                                    }
                                                    "rad" | "deg" => ColorFuncArg::Rotation(
                                                        lcsf.as_str().to_string(),
                                                    ),
                                                    _ => {
                                                        unreachable!(); //Unsupported unit
                                                    }
                                                }
                                            }
                                            Rule::literal_number_integer => {
                                                ColorFuncArg::Integer(lcsf.as_str().to_string())
                                            }
                                            _ => {
                                                panic!("{}", lcsf.as_str())
                                            }
                                        }
                                    })
                                    .collect();

                                visitor.visit_enum(PaxColor {
                                    color_func: func,
                                    args,
                                })
                            }
                            Rule::literal_color_const => {
                                let explicit_color = visitor.visit_enum(PaxEnum::new(
                                    Some(COLOR.to_string()),
                                    what_kind_of_color.as_str().to_string(),
                                    None,
                                ));
                                explicit_color
                            }
                            _ => {
                                unreachable!()
                            }
                        }
                    }
                    Rule::literal_number => {
                        let number = inner_pair.into_inner().next().unwrap();
                        match number.as_rule() {
                            Rule::literal_number_integer => visitor.visit_enum(PaxEnum::new(
                                Some(NUMERIC.to_string()),
                                I64.to_string(),
                                Some(number.as_str().to_string()),
                            )),
                            Rule::literal_number_float => visitor.visit_enum(PaxEnum::new(
                                Some(NUMERIC.to_string()),
                                F64.to_string(),
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
                                None,
                                PERCENT.to_string(),
                                Some(number.to_string()),
                            )),
                            "px" => visitor.visit_enum(PaxEnum::new(
                                Some(SIZE.to_string()),
                                PIXELS.to_string(),
                                Some(number.to_string()),
                            )),
                            "rad" => visitor.visit_enum(PaxEnum::new(
                                Some(ROTATION.to_string()),
                                RADIANS.to_string(),
                                Some(number.to_string()),
                            )),
                            "deg" => visitor.visit_enum(PaxEnum::new(
                                Some(ROTATION.to_string()),
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
                            .map(|pair| PaxSeqArg::String(pair.as_str().to_string()))
                            .collect::<Vec<PaxSeqArg>>();
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

        Ok(ret)
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
