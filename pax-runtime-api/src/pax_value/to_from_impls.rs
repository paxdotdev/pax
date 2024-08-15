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
use crate::math::Vector2;
use crate::properties::PropertyValue;
use crate::Color;
use crate::ColorChannel;
use crate::Fill;
use crate::GradientStop;
use crate::LinearGradient;
use crate::Percent;
use crate::Property;
use crate::RadialGradient;
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

// Pax internal types
impl_to_from_pax_value!(Numeric, PaxValue::Numeric);
impl_to_from_pax_value!(Size, PaxValue::Size);
impl_to_from_pax_value!(Color, PaxValue::Color);
impl_to_from_pax_value!(Rotation, PaxValue::Rotation);
impl_to_from_pax_value!(Percent, PaxValue::Percent);

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

impl ToPaxValue for Fill {
    fn to_pax_value(self) -> PaxValue {
        match self {
            Fill::Solid(color) => PaxValue::Enum("Solid".to_string(), vec![color.to_pax_value()]),
            Fill::LinearGradient(gradient) => {
                PaxValue::Enum("LinearGradient".to_string(), vec![gradient.to_pax_value()])
            }
            Fill::RadialGradient(gradient) => {
                PaxValue::Enum("RadialGradient".to_string(), vec![gradient.to_pax_value()])
            }
        }
    }
}

impl ToPaxValue for ColorChannel {
    fn to_pax_value(self) -> PaxValue {
        match self {
            ColorChannel::Rotation(rot) => {
                PaxValue::Enum("Rotation".to_string(), vec![rot.to_pax_value()])
            }
            ColorChannel::Percent(perc) => {
                PaxValue::Enum("Percent".to_string(), vec![perc.to_pax_value()])
            }
            ColorChannel::Integer(num) => {
                PaxValue::Enum("Integer".to_string(), vec![num.to_pax_value()])
            }
        }
    }
}

impl ToPaxValue for Stroke {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(
            vec![
                ("color".to_string(), self.color.to_pax_value()),
                ("width".to_string(), self.width.to_pax_value()),
            ]
            .into_iter()
            .collect(),
        )
    }
}

impl ToPaxValue for GradientStop {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(
            vec![
                ("position".to_string(), self.position.to_pax_value()),
                ("color".to_string(), self.color.to_pax_value()),
            ]
            .into_iter()
            .collect(),
        )
    }
}

impl ToPaxValue for LinearGradient {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(
            vec![
                (
                    "start".to_string(),
                    PaxValue::Vec(vec![
                        self.start.0.to_pax_value(),
                        self.start.1.to_pax_value(),
                    ]),
                ),
                (
                    "end".to_string(),
                    PaxValue::Vec(vec![self.end.0.to_pax_value(), self.end.1.to_pax_value()]),
                ),
                ("stops".to_string(), self.stops.to_pax_value()),
            ]
            .into_iter()
            .collect(),
        )
    }
}

impl ToPaxValue for RadialGradient {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(
            vec![
                (
                    "start".to_string(),
                    PaxValue::Vec(vec![
                        self.start.0.to_pax_value(),
                        self.start.1.to_pax_value(),
                    ]),
                ),
                (
                    "end".to_string(),
                    PaxValue::Vec(vec![self.end.0.to_pax_value(), self.end.1.to_pax_value()]),
                ),
                ("radius".to_string(), self.radius.to_pax_value()),
                ("stops".to_string(), self.stops.to_pax_value()),
            ]
            .into_iter()
            .collect(),
        )
    }
}

impl ToPaxValue for Transform2D {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(
            vec![
                (
                    "previous".to_string(),
                    self.previous
                        .map(|p| p.to_pax_value())
                        .unwrap_or(PaxValue::Option(Box::new(None))),
                ),
                (
                    "rotate".to_string(),
                    self.rotate
                        .map(|r| r.to_pax_value())
                        .unwrap_or(PaxValue::Option(Box::new(None))),
                ),
                (
                    "translate".to_string(),
                    self.translate
                        .map(|t| PaxValue::Option(Box::new(Some(t.to_vec().to_pax_value()))))
                        .unwrap_or(PaxValue::Option(Box::new(None))),
                ),
                (
                    "anchor".to_string(),
                    self.anchor
                        .map(|a| PaxValue::Option(Box::new(Some(a.to_vec().to_pax_value()))))
                        .unwrap_or(PaxValue::Option(Box::new(None))),
                ),
                (
                    "scale".to_string(),
                    self.scale
                        .map(|s| PaxValue::Option(Box::new(Some(s.to_vec().to_pax_value()))))
                        .unwrap_or(PaxValue::Option(Box::new(None))),
                ),
                (
                    "skew".to_string(),
                    self.skew
                        .map(|s| PaxValue::Option(Box::new(Some(s.to_vec().to_pax_value()))))
                        .unwrap_or(PaxValue::Option(Box::new(None))),
                ),
            ]
            .into_iter()
            .collect(),
        )
    }
}

impl ToPaxValue for Transform2 {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(
            vec![
                (
                    "m".to_string(),
                    PaxValue::Vec(
                        self.m.iter().map(|v| v.to_pax_value()).collect::<Vec<_>>(),
                    ),
                ),
            ]
            .into_iter()
            .collect(),
        )
    }
}

impl ToPaxValue for Vector2 {
    fn to_pax_value(self) -> PaxValue {
        PaxValue::Object(
            vec![
                ("x".to_string(), self.x.to_pax_value()),
                ("y".to_string(), self.y.to_pax_value()),
            ]
            .into_iter()
            .collect(),
        )
    }
}