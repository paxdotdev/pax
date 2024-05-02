// ------------------------------- Coersion rules ----------------------------------
// default coercion only allows a single type: the type expected
// custom coercion rules can be implemented by a type

use crate::{
    impl_default_coercion_rule, Color, Fill, Numeric, PaxValue, Percent, Rotation, Size,
    Transform2D,
};

impl_default_coercion_rule!(bool, PaxValue::Bool);

impl_default_coercion_rule!(u8, PaxValue::Numeric, Numeric::U8);
impl_default_coercion_rule!(u16, PaxValue::Numeric, Numeric::U16);
impl_default_coercion_rule!(u32, PaxValue::Numeric, Numeric::U32);
impl_default_coercion_rule!(u64, PaxValue::Numeric, Numeric::U64);
impl_default_coercion_rule!(u128, PaxValue::Numeric, Numeric::U128);

impl_default_coercion_rule!(i8, PaxValue::Numeric, Numeric::I8);
impl_default_coercion_rule!(i16, PaxValue::Numeric, Numeric::I16);
impl_default_coercion_rule!(i32, PaxValue::Numeric, Numeric::I32);
impl_default_coercion_rule!(i64, PaxValue::Numeric, Numeric::I64);
impl_default_coercion_rule!(i128, PaxValue::Numeric, Numeric::I128);

impl_default_coercion_rule!(f32, PaxValue::Numeric, Numeric::F32);
impl_default_coercion_rule!(f64, PaxValue::Numeric, Numeric::F64);

impl_default_coercion_rule!(isize, PaxValue::Numeric, Numeric::ISize);
impl_default_coercion_rule!(usize, PaxValue::Numeric, Numeric::USize);
impl_default_coercion_rule!(String, PaxValue::String);

// Pax internal types
impl_default_coercion_rule!(Numeric, PaxValue::Numeric);
impl_default_coercion_rule!(Size, PaxValue::Size);
impl_default_coercion_rule!(Color, PaxValue::Color);
impl_default_coercion_rule!(Transform2D, PaxValue::Transform2D);
impl_default_coercion_rule!(Rotation, PaxValue::Rotation);
impl_default_coercion_rule!(Percent, PaxValue::Percent);

pub trait CoercionRules {
    fn try_coerce(value: PaxValue) -> Result<PaxValue, String>;
}

// Fill is a type that other types (Color) can be coerced into, thus the default
// from to pax macro isn't enough (only translates directly back and forth, and returns
// an error if it contains any other type)
impl CoercionRules for Fill {
    fn try_coerce(pax_value: PaxValue) -> Result<PaxValue, String> {
        match pax_value {
            PaxValue::Color(color) => Ok(PaxValue::Fill(Fill::Solid(color))),
            PaxValue::Fill(_) => Ok(pax_value),
            _ => Err(format!(
                "owned {:?} can't be coerced into a Fill",
                pax_value
            )),
        }
    }
}

// Now that we have an impl to/from pax_value, we can
// automatically fill in the impl to go to/from PaxAny

// TODO end
// Literal Intoable Graph, as of initial impl:
// Numeric
// - Size
// - Rotation
// - ColorChannel
// Percent
// - ColorChannel
// - Rotation
// - Size
// Color
// - Stroke (1px solid)
// - Fill (solid)
