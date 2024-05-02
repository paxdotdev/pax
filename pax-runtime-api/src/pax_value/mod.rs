use serde::{Deserialize, Serialize};

use crate::{Color, Fill, Percent, Rotation, Size, Transform2D};
use std::any::{Any, TypeId};

use self::numeric::Numeric;
pub use coercion_impls::CoercionRules;

mod coercion_impls;
mod macros;
pub mod numeric;
mod to_from_impls;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "crate::serde")]
pub enum PaxValue {
    Bool(bool),
    Numeric(Numeric),
    String(String),
    Transform2D(Transform2D),
    Size(Size),
    Percent(Percent),
    Color(Color),
    Rotation(Rotation),
    Fill(Fill),
    Component {},
}

pub enum PaxAny {
    Builtin(PaxValue),
    Any(Box<dyn Any>),
}

impl PaxAny {
    pub fn try_clone<T: Clone + 'static>(&self) -> Result<Self, String> {
        Ok(match self {
            PaxAny::Builtin(pax_value) => pax_value.clone().to_pax_any(),
            PaxAny::Any(any) => PaxAny::Any(Box::new(
                any.downcast_ref::<T>()
                    .ok_or_else(|| "downcast failed while trying to clone PaxAny")?
                    .clone(),
            )),
        })
    }

    pub fn try_coerce<T: CoercionRules + 'static>(self) -> Result<Self, String> {
        match self {
            PaxAny::Builtin(pax_type) => T::try_coerce(pax_type).map(|v| v.to_pax_any()),
            PaxAny::Any(any) => {
                if any.as_ref().type_id() == TypeId::of::<T>() {
                    Ok(PaxAny::Any(any))
                } else {
                    Err("tried to coerce PaxAny into non-underlying type".to_string())
                }
            }
        }
    }
}

pub trait ToFromPaxValue
where
    Self: Sized + 'static,
{
    fn to_pax_value(self) -> PaxValue;
    fn from_pax_value(pax_value: PaxValue) -> Result<Self, String>;
    fn ref_from_pax_value(pax_value: &PaxValue) -> Result<&Self, String>;
    fn mut_from_pax_value(pax_value: &mut PaxValue) -> Result<&mut Self, String>;
}

pub trait IntoablePaxValue {
    fn coerce_to_type(value: &mut PaxValue);
}

pub trait ToFromPaxAny
where
    Self: Sized + 'static,
{
    fn to_pax_any(self) -> PaxAny;
    fn from_pax_any(pax_any: PaxAny) -> Result<Self, String>;
    fn ref_from_pax_any(pax_any: &PaxAny) -> Result<&Self, String>;
    fn mut_from_pax_any(pax_any: &mut PaxAny) -> Result<&mut Self, String>;
}

impl ToFromPaxAny for PaxValue {
    fn to_pax_any(self) -> PaxAny {
        PaxAny::Builtin(self)
    }

    fn from_pax_any(pax_any: PaxAny) -> Result<Self, String> {
        match pax_any {
            PaxAny::Builtin(val) => Ok(val),
            PaxAny::Any(_) => Err("tried to unwrap builtin as any".to_string()),
        }
    }

    fn ref_from_pax_any(pax_any: &PaxAny) -> Result<&Self, String> {
        match pax_any {
            PaxAny::Builtin(val) => Ok(val),
            PaxAny::Any(_) => Err("tried to unwrap builtin as any".to_string()),
        }
    }

    fn mut_from_pax_any(pax_any: &mut PaxAny) -> Result<&mut Self, String> {
        match pax_any {
            PaxAny::Builtin(val) => Ok(val),
            PaxAny::Any(_) => Err("tried to unwrap builtin as any".to_string()),
        }
    }
}

//Marker trait
pub trait ImplToFromPaxAny: 'static {}

// If a type has marker trait, implement to from
// pax any automatically by wrapping in Box<dyn Any>
impl<T: ImplToFromPaxAny> ToFromPaxAny for T {
    fn to_pax_any(self) -> PaxAny {
        PaxAny::Any(Box::new(self) as Box<dyn Any>)
    }

    fn from_pax_any(pax_any: PaxAny) -> Result<Self, String> {
        match pax_any {
            PaxAny::Any(v) => Ok(*v
                .downcast::<Self>()
                .map_err(|_e| "downcast failed".to_string())?),
            _ => Err("wasn't any".to_string()),
        }
    }

    fn ref_from_pax_any(pax_any: &PaxAny) -> Result<&Self, String> {
        match pax_any {
            PaxAny::Any(v) => v
                .downcast_ref::<Self>()
                .ok_or_else(|| "downcast failed".to_string()),
            _ => Err("wasn't any".to_string()),
        }
    }

    fn mut_from_pax_any(pax_any: &mut PaxAny) -> Result<&mut Self, String> {
        match pax_any {
            PaxAny::Any(v) => v
                .downcast_mut::<Self>()
                .ok_or_else(|| "downcast failed".to_string()),
            _ => Err("wasn't any".to_string()),
        }
    }
}

impl<T: ImplToFromPaxAny> CoercionRules for T {
    fn try_coerce(_value: PaxValue) -> Result<PaxValue, String> {
        unreachable!("can't")
    }
}

// TODO check these spots after doing initial conversion
// - what can be done in the property system? (problem with recursive properties requiring Property to be a PaxValue type, not good?)
// - can interpolatable and other things be directly implemented on PaxValue instead?

// TODO final check:
// - do we need all of these enum variants
