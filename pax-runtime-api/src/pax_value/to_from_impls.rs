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

// pub struct Transform2D {
//     /// Keeps track of a linked list of previous Transform2Ds, assembled e.g. via multiplication
//     pub previous: Option<Box<Transform2D>>,
//     /// Rotation is single-dimensional for 2D rendering, representing rotation over z axis
//     pub rotate: Option<Rotation>,
//     pub translate: Option<[Size; 2]>,
//     pub anchor: Option<[Size; 2]>,
//     pub scale: Option<[Size; 2]>,
//     pub skew: Option<[Rotation; 2]>,
// }

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

// // ------------------------------- Coersion rules ----------------------------------
// // default coercion only allows a single type: the type expected
// // custom coercion rules can be implemented by a type

// use std::ops::Range;

// use crate::{
//     impl_default_coercion_rule, Color, ColorChannel, Fill, GradientStop, LinearGradient, Numeric,
//     PaxValue, Percent, Property, RadialGradient, Rotation, Size, Stroke, Transform2D,
// };

// // Default coersion rules:
// // call Into::<first param>::into() on contents of second enum variant
// impl_default_coercion_rule!(bool, PaxValue::Bool);

// impl_default_coercion_rule!(u8, PaxValue::Numeric);
// impl_default_coercion_rule!(u16, PaxValue::Numeric);
// impl_default_coercion_rule!(u32, PaxValue::Numeric);
// impl_default_coercion_rule!(u64, PaxValue::Numeric);

// impl_default_coercion_rule!(i8, PaxValue::Numeric);
// impl_default_coercion_rule!(i16, PaxValue::Numeric);
// impl_default_coercion_rule!(i32, PaxValue::Numeric);
// impl_default_coercion_rule!(i64, PaxValue::Numeric);

// impl_default_coercion_rule!(f32, PaxValue::Numeric);
// impl_default_coercion_rule!(f64, PaxValue::Numeric);

// impl_default_coercion_rule!(isize, PaxValue::Numeric);
// impl_default_coercion_rule!(usize, PaxValue::Numeric);

// // Pax internal types
// impl_default_coercion_rule!(Color, PaxValue::Color);

// pub trait CoercionRules
// where
//     Self: Sized + 'static,
// {
//     fn try_coerce(value: PaxValue) -> Result<Self, String>;
// }

// // pub enum Fill {
// //     Solid(Color),
// //     LinearGradient(LinearGradient),
// //     RadialGradient(RadialGradient),
// // }

// // Fill is a type that other types (Color) can be coerced into, thus the default
// // from to pax macro isn't enough (only translates directly back and forth, and returns
// // an error if it contains any other type)
// impl CoercionRules for Fill {
//     fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
//         Ok(match pax_value.clone() {
//             PaxValue::Color(color) => Fill::Solid(color),
//             PaxValue::Enum(variant, args) => match variant.as_str() {
//                 "Solid" => {
//                     let color = Color::try_coerce(args[0].clone())?;
//                     Fill::Solid(color)
//                 }
//                 "LinearGradient" => {
//                     let gradient = LinearGradient::try_coerce(args[0].clone())?;
//                     Fill::LinearGradient(gradient)
//                 }
//                 "RadialGradient" => {
//                     let gradient = RadialGradient::try_coerce(args[0].clone())?;
//                     Fill::RadialGradient(gradient)
//                 }
//                 _ => return Err(format!("{:?} can't be coerced into a Fill", pax_value)),
//             },
//             _ => return Err(format!("{:?} can't be coerced into a Fill", pax_value)),
//         })
//     }
// }

// impl CoercionRules for LinearGradient {
//     fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
//         Ok(match pax_value.clone() {
//             PaxValue::Object(map) => {
//                 let start = map.get("start").unwrap().clone();
//                 let (s1, s2) = match start {
//                     PaxValue::Vec(vec) => {
//                         let s1 = Size::try_coerce(vec[0].clone())?;
//                         let s2 = Size::try_coerce(vec[1].clone())?;
//                         (s1, s2)
//                     }
//                     _ => {
//                         return Err(format!(
//                             "{:?} can't be coerced into a LinearGradient",
//                             pax_value
//                         ))
//                     }
//                 };

//                 let end = map.get("end").unwrap().clone();
//                 let (e1, e2) = match end {
//                     PaxValue::Vec(vec) => {
//                         let e1 = Size::try_coerce(vec[0].clone())?;
//                         let e2 = Size::try_coerce(vec[1].clone())?;
//                         (e1, e2)
//                     }
//                     _ => {
//                         return Err(format!(
//                             "{:?} can't be coerced into a LinearGradient",
//                             pax_value
//                         ))
//                     }
//                 };
//                 let stops = Vec::<GradientStop>::try_coerce(map.get("stops").unwrap().clone())?;
//                 LinearGradient {
//                     start: (s1, s2),
//                     end: (e1, e2),
//                     stops,
//                 }
//             }
//             _ => {
//                 return Err(format!(
//                     "{:?} can't be coerced into a LinearGradient",
//                     pax_value
//                 ))
//             }
//         })
//     }
// }

// impl CoercionRules for RadialGradient {
//     fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
//         Ok(match pax_value.clone() {
//             PaxValue::Object(map) => {
//                 let start = map.get("start").unwrap().clone();
//                 let (s1, s2) = match start {
//                     PaxValue::Vec(vec) => {
//                         let s1 = Size::try_coerce(vec[0].clone())?;
//                         let s2 = Size::try_coerce(vec[1].clone())?;
//                         (s1, s2)
//                     }
//                     _ => {
//                         return Err(format!(
//                             "{:?} can't be coerced into a RadialGradient",
//                             pax_value
//                         ))
//                     }
//                 };

//                 let end = map.get("end").unwrap().clone();
//                 let (e1, e2) = match end {
//                     PaxValue::Vec(vec) => {
//                         let e1 = Size::try_coerce(vec[0].clone())?;
//                         let e2 = Size::try_coerce(vec[1].clone())?;
//                         (e1, e2)
//                     }
//                     _ => {
//                         return Err(format!(
//                             "{:?} can't be coerced into a RadialGradient",
//                             pax_value
//                         ))
//                     }
//                 };
//                 let radius = match map.get("radius").unwrap().clone() {
//                     PaxValue::Numeric(n) => n.to_float(),
//                     _ => {
//                         return Err(format!(
//                             "{:?} can't be coerced into a RadialGradient",
//                             pax_value
//                         ))
//                     }
//                 };
//                 let stops = Vec::<GradientStop>::try_coerce(map.get("stops").unwrap().clone())?;
//                 RadialGradient {
//                     start: (s1, s2),
//                     end: (e1, e2),
//                     radius,
//                     stops,
//                 }
//             }
//             _ => {
//                 return Err(format!(
//                     "{:?} can't be coerced into a RadialGradient",
//                     pax_value
//                 ))
//             }
//         })
//     }
// }

// impl CoercionRules for GradientStop {
//     fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
//         Ok(match pax_value {
//             PaxValue::Object(map) => {
//                 let position = Size::try_coerce(map.get("position").unwrap().clone())?;
//                 let color = Color::try_coerce(map.get("color").unwrap().clone())?;
//                 GradientStop { position, color }
//             }
//             _ => {
//                 return Err(format!(
//                     "{:?} can't be coerced into a GradientStop",
//                     pax_value
//                 ))
//             }
//         })
//     }
// }

// impl CoercionRules for Stroke {
//     fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
//         Ok(match pax_value {
//             PaxValue::Color(color) => Stroke {
//                 color: Property::new(color),
//                 width: Property::new(Size::Pixels(1.into())),
//             },
//             PaxValue::Object(map) => {
//                 let color = Property::new(Color::try_coerce(map.get("color").unwrap().clone())?);
//                 let width = Property::new(Size::try_coerce(map.get("width").unwrap().clone())?);
//                 Stroke { color, width }
//             }
//             _ => return Err(format!("{:?} can't be coerced into a Stroke", pax_value)),
//         })
//     }
// }

// impl CoercionRules for ColorChannel {
//     fn try_coerce(value: PaxValue) -> Result<Self, String> {
//         Ok(match value.clone() {
//             PaxValue::Rotation(rot) => ColorChannel::Rotation(rot),
//             PaxValue::Percent(perc) => ColorChannel::Percent(perc.0),
//             PaxValue::Numeric(num) => ColorChannel::Integer(num),
//             PaxValue::Enum(variant, args) => match variant.as_str() {
//                 "Rotation" => {
//                     let rot = Rotation::try_coerce(args[0].clone())?;
//                     ColorChannel::Rotation(rot)
//                 }
//                 "Integer" => {
//                     let num = Numeric::try_coerce(args[0].clone())?;
//                     ColorChannel::Integer(num)
//                 }
//                 "Percent" => {
//                     let num = Numeric::try_coerce(args[0].clone())?;
//                     ColorChannel::Percent(num)
//                 }
//                 _ => return Err(format!("{:?} can't be coerced into a ColorChannel", value)),
//             },
//             _ => return Err(format!("{:?} can't be coerced into a ColorChannel", value)),
//         })
//     }
// }

// impl CoercionRules for Percent {
//     fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
//         Ok(match pax_value {
//             PaxValue::Percent(p) => p,
//             PaxValue::Numeric(v) => Percent(v),
//             _ => return Err(format!("{:?} can't be coerced into a Percent", pax_value)),
//         })
//     }
// }

// impl CoercionRules for Size {
//     fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
//         Ok(match pax_value {
//             PaxValue::Size(size) => size,
//             PaxValue::Percent(p) => Size::Percent(p.0),
//             PaxValue::Numeric(v) => Size::Pixels(v),
//             _ => return Err(format!("{:?} can't be coerced into a Size", pax_value)),
//         })
//     }
// }

// impl CoercionRules for String {
//     fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
//         Ok(match pax_value {
//             PaxValue::String(s) => s,
//             PaxValue::Numeric(n) => {
//                 if n.is_float() {
//                     n.to_float().to_string()
//                 } else {
//                     n.to_int().to_string()
//                 }
//             }
//             _ => return Err(format!("{:?} can't be coerced into a String", pax_value)),
//         })
//     }
// }

// impl CoercionRules for Rotation {
//     fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
//         Ok(match pax_value {
//             PaxValue::Rotation(r) => r,
//             PaxValue::Numeric(n) => Rotation::Degrees(n),
//             PaxValue::Percent(p) => Rotation::Percent(p.0),
//             _ => return Err(format!("{:?} can't be coerced into a Rotation", pax_value)),
//         })
//     }
// }

// impl CoercionRules for Numeric {
//     fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
//         Ok(match pax_value {
//             PaxValue::Numeric(n) => n.into(),
//             PaxValue::Size(n) => n.into(),
//             _ => return Err(format!("{:?} can't be coerced into a Numeric", pax_value)),
//         })
//     }
// }

// impl<T: CoercionRules> CoercionRules for Vec<T> {
//     fn try_coerce(value: PaxValue) -> Result<Self, String> {
//         match value {
//             PaxValue::Vec(vec) => {
//                 let res: Result<Vec<T>, _> = vec.into_iter().map(|v| T::try_coerce(v)).collect();
//                 res.map_err(|e| format!("couldn't coerce vec, element {:?}", e))
//             }
//             v => Err(format!(
//                 "{:?} can't be coerced into {:?}",
//                 v,
//                 std::any::type_name::<Vec<T>>(),
//             )),
//         }
//     }
// }

// impl<T: CoercionRules> CoercionRules for Option<T> {
//     fn try_coerce(value: PaxValue) -> Result<Self, String> {
//         match value {
//             PaxValue::Option(opt) => {
//                 let res: Result<Option<T>, _> = opt.map(|v| T::try_coerce(v)).transpose();
//                 res.map_err(|e| format!("couldn't coerce option, element {:?}", e))
//             }
//             v => Err(format!(
//                 "{:?} can't be coerced into {:?}",
//                 v,
//                 std::any::type_name::<Option<T>>(),
//             )),
//         }
//     }
// }

// impl<T: CoercionRules> CoercionRules for Range<T> {
//     fn try_coerce(value: PaxValue) -> Result<Self, String> {
//         match value {
//             PaxValue::Range(start, end) => {
//                 let start = T::try_coerce(*start)?;
//                 let end = T::try_coerce(*end)?;
//                 Ok(start..end)
//             }
//             v => Err(format!(
//                 "{:?} can't be coerced into {:?}",
//                 v,
//                 std::any::type_name::<Range<T>>(),
//             )),
//         }
//     }
// }

// impl CoercionRules for PaxValue {
//     fn try_coerce(value: PaxValue) -> Result<Self, String> {
//         Ok(value)
//     }
// }

// impl CoercionRules for Transform2D {
//     fn try_coerce(value: PaxValue) -> Result<Self, String> {
//         Ok(match value {
//             PaxValue::Object(map) => {
//                 let previous = match map.get("previous") {
//                     Some(p) => Some(Box::new(Transform2D::try_coerce(p.clone())?)),
//                     None => None,
//                 };
//                 let rotate = match map.get("rotate") {
//                     Some(r) => Some(Rotation::try_coerce(r.clone())?),
//                     None => None,
//                 };
//                 let translate = match map.get("translate") {
//                     Some(t) => {
//                         match t.clone() {
//                             PaxValue::Option(mut opt) => {
//                                 if let Some(t) = opt.take() {
//                                     let t = Vec::<Size>::try_coerce(t.clone())?;
//                                     if t.len() != 2 {
//                                         return Err(format!(
//                                             "expected 2 elements in translate, got {:?}",
//                                             t.len()
//                                         ));
//                                     }
//                                     Some([t[0], t[1]])
//                                 } else {
//                                     None
//                                 }
//                             }
//                             _ => return Err(format!("{:?} can't be coerced into a Transform2D", t)),
//                         }
//                     }
//                     None => None,
//                 };
//                 let anchor = match map.get("anchor") {
//                     Some(a) => {
//                         match a.clone() {
//                             PaxValue::Option(mut opt) => {
//                                 if let Some(a) = opt.take() {
//                                     let a = Vec::<Size>::try_coerce(a.clone())?;
//                                     if a.len() != 2 {
//                                         return Err(format!(
//                                             "expected 2 elements in anchor, got {:?}",
//                                             a.len()
//                                         ));
//                                     }
//                                     Some([a[0], a[1]])
//                                 } else {
//                                     None
//                                 }
//                             }
//                             _ => return Err(format!("{:?} can't be coerced into a Transform2D", a)),
//                         }
//                     }
//                     None => None,
//                 };
//                 let scale = match map.get("scale") {
//                     Some(s) => {
//                         match s.clone() {
//                             PaxValue::Option(mut opt) => {
//                                 if let Some(s) = opt.take() {
//                                     let s = Vec::<Size>::try_coerce(s.clone())?;
//                                     if s.len() != 2 {
//                                         return Err(format!(
//                                             "expected 2 elements in scale, got {:?}",
//                                             s.len()
//                                         ));
//                                     }
//                                     Some([s[0], s[1]])
//                                 } else {
//                                     None
//                                 }
//                             }
//                             _ => return Err(format!("{:?} can't be coerced into a Transform2D", s)),
//                         }
//                     }
//                     None => None,
//                 };
//                 let skew = match map.get("skew") {
//                     Some(s) => {
//                         match s.clone() {
//                             PaxValue::Option(mut opt) => {
//                                 if let Some(s) = opt.take() {
//                                     let s = Vec::<Rotation>::try_coerce(s.clone())?;
//                                     if s.len() != 2 {
//                                         return Err(format!(
//                                             "expected 2 elements in skew, got {:?}",
//                                             s.len()
//                                         ));
//                                     }
//                                     Some([s[0], s[1]])
//                                 } else {
//                                     None
//                                 }
//                             }
//                             _ => return Err(format!("{:?} can't be coerced into a Transform2D", s)),
//                         }
//                     }
//                     None => None,
//                 };
//                 Transform2D {
//                     previous,
//                     rotate,
//                     translate,
//                     anchor,
//                     scale,
//                     skew,
//                 }
//             }
//             _ => return Err(format!("{:?} can't be coerced into a Transform2D", value)),
//         })
//     }
// }
