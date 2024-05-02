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
use crate::StringBox;
use crate::Stroke;
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

impl_to_from_pax_value!(isize, PaxValue::Numeric, Numeric::ISize);
impl_to_from_pax_value!(usize, PaxValue::Numeric, Numeric::USize);

// don't allow to be serialized/deserialized sucessfully, just store it as a dyn Any
impl ImplToFromPaxAny for () {}

// for now. TBD how to handle this when we join Transform2D with Transform2 at some point
impl<F: Space, T: Space> ImplToFromPaxAny for Transform2<F, T> {}

impl_to_from_pax_value!(String, PaxValue::String);
impl_to_from_pax_value!(StringBox, PaxValue::StringBox);

// Pax internal types
impl_to_from_pax_value!(Numeric, PaxValue::Numeric);
impl_to_from_pax_value!(Size, PaxValue::Size);
impl_to_from_pax_value!(Color, PaxValue::Color);
impl_to_from_pax_value!(Transform2D, PaxValue::Transform2D);
impl_to_from_pax_value!(Rotation, PaxValue::Rotation);
impl_to_from_pax_value!(Percent, PaxValue::Percent);
impl_to_from_pax_value!(Fill, PaxValue::Fill);
impl_to_from_pax_value!(Stroke, PaxValue::Stroke);
