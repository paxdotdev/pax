use std::any::Any;

use derive_more::{From, TryInto};

use crate::{math::Transform2, Color, Numeric, Rotation, Size, Transform2D};

#[derive(TryInto, From)]
#[try_into(owned, ref, ref_mut)]
pub enum PaxValue {
    // TODO remove this variant (use bellow numeric variants instead)
    Numeric(Numeric),
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),

    F64(f64),
    F32(f32),
    String(String),
    Transform(Transform2),
    Transform2D(Transform2D),
    Size(Size),
    Color(Color),
    Rotation(Rotation),

    Component {},
    Any(Box<dyn Any>),
}

pub trait ToFromPaxValue
where
    Self: Sized + 'static,
{
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Any(Box::new(self) as Box<dyn Any>)
    }

    fn from_pax_value(pax_value: PaxValue) -> Result<Self, String> {
        match pax_value {
            PaxValue::Any(v) => Ok(*v
                .downcast::<Self>()
                .map_err(|_e| "downcast failed".to_string())?),
            _ => Err("wasn't any".to_string()),
        }
    }

    fn ref_from_pax_value(pax_value: &PaxValue) -> Result<&Self, String> {
        match pax_value {
            PaxValue::Any(v) => v
                .downcast_ref::<Self>()
                .ok_or_else(|| "downcast failed".to_string()),
            _ => Err("wasn't any".to_string()),
        }
    }

    fn mut_from_pax_value(pax_value: &mut PaxValue) -> Result<&mut Self, String> {
        match pax_value {
            PaxValue::Any(v) => v
                .downcast_mut::<Self>()
                .ok_or_else(|| "downcast failed".to_string()),
            _ => Err("wasn't any".to_string()),
        }
    }
}

// Remove this impl, and impl all individual types using a macro later on
impl<T: 'static> ToFromPaxValue for T {}

// TODO check these spots after doing initial conversion
// - what can be done in the property system? (problem with recursive properties requiring Property to be a PaxValue type, not good?)
// - can interpolatable and other things be directly implemented on PaxValue instead?

// TODO final check:
// - do we need all of these enum variants
