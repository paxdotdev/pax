use crate::{Color, ColorChannel, Fill, Percent, Rotation, Size, Stroke, Transform2D};
use std::{any::Any, collections::HashMap};

use self::numeric::Numeric;
pub use coercion_impls::CoercionRules;
use serde::{Deserialize, Serialize};

mod arithmetic;
mod coercion_impls;
mod macros;
pub mod numeric;
pub mod functions;
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
    Transform2D(Transform2D),
    Size(Size),
    Percent(Percent), 
    Color(Color),
    ColorChannel(ColorChannel),
    Rotation(Rotation),
    Fill(Fill),
    Stroke(Stroke),
    Option(Box<Option<PaxValue>>),
    // Ideally this is later changed to Vec<PaxValue>, once structs can be
    // represented in PaxValue as a map, enabling serialize/deserialization
    // debug impl, etc.
    Vec(Vec<PaxValue>),
    Object(HashMap<String, PaxValue>),
    Enum(String, Vec<PaxValue>),
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

impl PaxAny {
    /// Try to co coerce the inner type to type T. For the any type, just make
    /// sure the stored any value is of type T. For a PaxValue, try to coerce it
    /// into the expected type
    pub fn try_coerce<T: ToFromPaxAny + CoercionRules + 'static>(self) -> Result<T, String> {
        let res = match self {
            PaxAny::Builtin(pax_type) => T::try_coerce(pax_type),
            PaxAny::Any(any) => any.downcast().map(|v| *v).map_err(|_| {
                format!(
                    "tried to coerce PaxAny into {} which wasn't the underlying type",
                    std::any::type_name::<T>()
                )
            }),
        };
        res
    }
}

/// This trait is implemented by all types that has a bultin equivalent
/// representation (see to_from_impls module) This is NOT responsible for
/// coercing between types, but returns an err in all cases where the underlying
/// type is not exactly what is expected
pub trait ToPaxValue{
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
