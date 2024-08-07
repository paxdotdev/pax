use std::ops::Range;
use std::rc::Rc;


use pax_message::RefCell;

use super::ImplToFromPaxAny;
use super::Numeric;
use super::PaxAny;
use super::PaxValue;
use super::ToFromPaxAny;
use super::ToPaxValue;
use crate::impl_to_from_pax_value;
use crate::math::Space;
use crate::math::Transform2;
use crate::properties::PropertyValue;
use crate::Color;
use crate::ColorChannel;
use crate::Fill;
use crate::Percent;
use crate::Property;
use crate::Rotation;
use crate::Size;
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
impl_to_from_pax_value!(ColorChannel, PaxValue::ColorChannel);

// Pax internal types
impl_to_from_pax_value!(Numeric, PaxValue::Numeric);
impl_to_from_pax_value!(Size, PaxValue::Size);
impl_to_from_pax_value!(Color, PaxValue::Color);
impl_to_from_pax_value!(Transform2D, PaxValue::Transform2D);
impl_to_from_pax_value!(Rotation, PaxValue::Rotation);
impl_to_from_pax_value!(Percent, PaxValue::Percent);
impl_to_from_pax_value!(Fill, PaxValue::Fill);
impl_to_from_pax_value!(Stroke, PaxValue::Stroke);

// Pax Vec type
impl<T: ToPaxValue> ToPaxValue for Vec<T> {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Vec(
            self.into_iter()
                .map(|v| v.to_pax_value())
                .collect::<Vec<_>>(),
        )
    }
}

impl<T: ToPaxValue> ToPaxValue for Option<T> {
    fn to_pax_value(self) -> PaxValue {
        match self {
            Some(v) => PaxValue::Option(Box::new(Some(v.to_pax_value()))),
            None => PaxValue::Option(Box::new(None)),
        }
    }
}

impl<T: ToPaxValue> ToPaxValue for Range<T> {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Range(
            Box::new(self.start.to_pax_value()),
            Box::new(self.end.to_pax_value()),
        )
    }
}


impl<T: ToPaxValue + Clone> ToPaxValue for Rc<RefCell<T>> {
    fn to_pax_value(self) -> PaxValue {
        crate::borrow!(self).clone().to_pax_value()
    }
}

impl ToPaxValue for PaxValue {
    fn to_pax_value(self) -> PaxValue {
        self
    }
}

impl<T: ToPaxValue + PropertyValue> ToPaxValue for Property<T> {
    fn to_pax_value(self) -> PaxValue {
        self.get().to_pax_value()
    }
}