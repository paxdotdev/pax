use super::ImplToFromPaxAny;
use super::Numeric;
use super::PaxAny;
use super::PaxValue;
use super::ToFromPaxAny;
use super::ToFromPaxValue;
use crate::impl_from_to_pax_any_for_from_to_pax_value;
use crate::impl_to_from_pax_value;
use crate::math::Space;
use crate::math::Transform2;
use crate::Color;
use crate::Fill;
use crate::Percent;
use crate::Rotation;
use crate::Size;
use crate::Transform2D;

// Primitive types
impl_to_from_pax_value!(bool, PaxValue::Bool);
impl_to_from_pax_value!(u8, PaxValue::Numeric, Numeric::U8);
impl_to_from_pax_value!(u16, PaxValue::Numeric, Numeric::U16);
impl_to_from_pax_value!(u32, PaxValue::Numeric, Numeric::U32);
impl_to_from_pax_value!(u64, PaxValue::Numeric, Numeric::U64);
impl_to_from_pax_value!(i8, PaxValue::Numeric, Numeric::I8);
impl_to_from_pax_value!(i16, PaxValue::Numeric, Numeric::I16);
impl_to_from_pax_value!(i32, PaxValue::Numeric, Numeric::I32);
impl_to_from_pax_value!(i64, PaxValue::Numeric, Numeric::I64);
impl_to_from_pax_value!(f32, PaxValue::Numeric, Numeric::F32);
impl_to_from_pax_value!(f64, PaxValue::Numeric, Numeric::F64);

// don't allow usize to be serialized/deserialized sucessfully, just store it as a dyn Any
impl ImplToFromPaxAny for () {}
impl ImplToFromPaxAny for usize {}
impl ImplToFromPaxAny for isize {}

// for now. TBD how to handle this when we join Transform2D with Transform2 at some point
impl<F: Space, T: Space> ImplToFromPaxAny for Transform2<F, T> {}

impl_to_from_pax_value!(String, PaxValue::String);

// Pax internal types
impl_to_from_pax_value!(Numeric, PaxValue::Numeric);
impl_to_from_pax_value!(Size, PaxValue::Size);
impl_to_from_pax_value!(Color, PaxValue::Color);
impl_to_from_pax_value!(Transform2D, PaxValue::Transform2D);
impl_to_from_pax_value!(Rotation, PaxValue::Rotation);
impl_to_from_pax_value!(Percent, PaxValue::Percent);

impl ToFromPaxValue for Fill {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Fill(self)
    }

    fn from_pax_value(pax_value: PaxValue) -> Result<Self, String> {
        match pax_value {
            PaxValue::Color(color) => Ok(Fill::Solid(color)),
            PaxValue::Fill(fill) => Ok(fill),
            _ => Err(format!(
                "owned {:?} can't be coerced into a Fill",
                pax_value
            )),
        }
    }

    fn ref_from_pax_value(pax_value: &PaxValue) -> Result<&Self, String> {
        match pax_value {
            PaxValue::Fill(fill) => Ok(fill),
            _ => Err(format!(
                "reference to {:?} can't be coerced into a &Fill",
                pax_value
            )),
        }
    }

    fn mut_from_pax_value(pax_value: &mut PaxValue) -> Result<&mut Self, String> {
        match pax_value {
            PaxValue::Fill(fill) => Ok(fill),
            _ => Err(format!(
                "mutable reference to {:?} can't be coerced into a &mut Fill",
                pax_value
            )),
        }
    }
}
impl_from_to_pax_any_for_from_to_pax_value!(Fill);
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
