use crate::{
    properties::PropertyValue, Color, ColorChannel, Fill, Interpolatable, Percent, Rotation, Size,
    Stroke, Transform2D,
};
use std::{any::Any, collections::HashMap, default, fmt::Display};

use self::numeric::Numeric;
pub use coercion_impls::CoercionRules;
use serde::{Deserialize, Serialize};

mod arithmetic;
mod coercion_impls;
pub mod functions;
mod macros;
pub mod numeric;
mod to_from_impls;

/// Container for all internal pax types
/// Two important traits are related to this type:
/// ToFromPaxValue - responsible for converting to and from specific types (u8,
/// String, Color, etc)
/// CoercionRules - responsible for coercing a PaxValue to a specific type
/// (possibly from multiple different variants)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "crate::serde")]
pub enum PaxValue {
    Bool(bool),
    Numeric(Numeric),
    String(String),
    Size(Size),
    Percent(Percent),
    Color(Color),
    Rotation(Rotation),
    Option(Box<Option<PaxValue>>),
    Vec(Vec<PaxValue>),
    Range(Box<PaxValue>, Box<PaxValue>),
    Object(HashMap<String, PaxValue>),
    Enum(String, Vec<PaxValue>),
}

// Make sure Enum looks like an enum and vec looks like a vec
impl Display for PaxValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaxValue::Bool(b) => write!(f, "{}", b),
            PaxValue::Numeric(n) => write!(f, "{}", n),
            PaxValue::String(s) => write!(f, "\"{}\"", s),
            PaxValue::Size(s) => write!(f, "{}", s),
            PaxValue::Percent(p) => write!(f, "{}", p),
            PaxValue::Color(c) => write!(f, "{}", c),
            PaxValue::Rotation(r) => write!(f, "{}", r),
            PaxValue::Option(o) => match o.as_ref() {
                Some(v) => write!(f, "Some({})", v),
                None => write!(f, "None"),
            },
            PaxValue::Vec(v) => {
                write!(f, "[")?;
                for (i, val) in v.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", val)?;
                }
                write!(f, "]")
            }
            PaxValue::Range(start, end) => write!(f, "{}..{}", start, end),
            PaxValue::Object(o) => {
                write!(f, "{{")?;
                for (i, (key, val)) in o.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", key, val)?;
                }
                write!(f, "}}")
            }
            PaxValue::Enum(name, values) => {
                write!(f, "{}(", name)?;
                for (i, val) in values.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", val)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl Default for PaxValue {
    fn default() -> Self {
        PaxValue::Numeric(Numeric::F64(0.0))
    }
}

/// This type serves a similar purpose as Box<dyn Any>, but allows for special
/// handling of some types, enabling things like coercion.
pub enum PaxAny {
    Builtin(PaxValue),
    Any(Box<dyn Any>),
}

impl std::fmt::Debug for PaxAny {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PaxAny {{ .. }}")
    }
}

impl Interpolatable for PaxValue {}

/// This trait is implemented by all types that has a bultin equivalent
/// representation (see to_from_impls module) This is NOT responsible for
/// coercing between types, but returns an err in all cases where the underlying
/// type is not exactly what is expected
pub trait ToPaxValue {
    fn to_pax_value(self) -> PaxValue;
}

/// Trait that marks a type as being representable as a PaxAny, and provides
/// the implementation for going to/from that type. For all builtins this
/// means going to/from a pax value. For others to a Box<dyn Any>. This
/// is automatically Implemented for PaxValue types through the macro
/// impl_to_from_pax_value!, and for other types by implementing the marker
/// trait ImplToFromPaxAny.
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
            PaxAny::Any(_) => Err("tried to unwrap any as builtin".to_string()),
        }
    }

    fn ref_from_pax_any(pax_any: &PaxAny) -> Result<&Self, String> {
        match pax_any {
            PaxAny::Builtin(val) => Ok(val),
            PaxAny::Any(_) => Err("tried to unwrap any as builtin".to_string()),
        }
    }

    fn mut_from_pax_any(pax_any: &mut PaxAny) -> Result<&mut Self, String> {
        match pax_any {
            PaxAny::Builtin(val) => Ok(val),
            PaxAny::Any(_) => Err("tried to unwrap any as builtin".to_string()),
        }
    }
}

/// Marker trait. Implement only for types that are not part of PaxValue, but
/// need to be stored inside a PaxAny. If they are part of pax value, instead
/// implement CoercionRules manually, or using the default impl macro as seen
/// in coercion_impls.rs
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

// PaxAny can turn into PaxAny
impl ToFromPaxAny for PaxAny {
    fn to_pax_any(self) -> PaxAny {
        self
    }

    fn from_pax_any(pax_any: PaxAny) -> Result<Self, String> {
        Ok(pax_any)
    }

    fn ref_from_pax_any(pax_any: &PaxAny) -> Result<&Self, String> {
        Ok(pax_any)
    }

    fn mut_from_pax_any(pax_any: &mut PaxAny) -> Result<&mut Self, String> {
        Ok(pax_any)
    }
}
