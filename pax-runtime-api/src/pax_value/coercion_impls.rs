// ------------------------------- Coersion rules ----------------------------------
// default coercion only allows a single type: the type expected
// custom coercion rules can be implemented by a type

use std::ops::Range;

use crate::{
    impl_default_coercion_rule, Color, ColorChannel, Fill, GradientStop, LinearGradient, Numeric,
    PaxValue, Percent, Property, RadialGradient, Rotation, Size, Stroke, Transform2D,
};

// Default coersion rules:
// call Into::<first param>::into() on contents of second enum variant
impl_default_coercion_rule!(bool, PaxValue::Bool);

impl_default_coercion_rule!(u8, PaxValue::Numeric);
impl_default_coercion_rule!(u16, PaxValue::Numeric);
impl_default_coercion_rule!(u32, PaxValue::Numeric);
impl_default_coercion_rule!(u64, PaxValue::Numeric);

impl_default_coercion_rule!(i8, PaxValue::Numeric);
impl_default_coercion_rule!(i16, PaxValue::Numeric);
impl_default_coercion_rule!(i32, PaxValue::Numeric);
impl_default_coercion_rule!(i64, PaxValue::Numeric);

impl_default_coercion_rule!(f32, PaxValue::Numeric);
impl_default_coercion_rule!(f64, PaxValue::Numeric);

impl_default_coercion_rule!(isize, PaxValue::Numeric);
impl_default_coercion_rule!(usize, PaxValue::Numeric);

// Pax internal types
impl_default_coercion_rule!(Color, PaxValue::Color);

pub trait CoercionRules
where
    Self: Sized + 'static,
{
    fn try_coerce(value: PaxValue) -> Result<Self, String>;
}

// pub enum Fill {
//     Solid(Color),
//     LinearGradient(LinearGradient),
//     RadialGradient(RadialGradient),
// }

// Fill is a type that other types (Color) can be coerced into, thus the default
// from to pax macro isn't enough (only translates directly back and forth, and returns
// an error if it contains any other type)
impl CoercionRules for Fill {
    fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
        Ok(match pax_value.clone() {
            PaxValue::Color(color) => Fill::Solid(color),
            PaxValue::Enum(variant, args) => match variant.as_str() {
                "Solid" => {
                    let color = Color::try_coerce(args[0].clone())?;
                    Fill::Solid(color)
                }
                "LinearGradient" => {
                    let gradient = LinearGradient::try_coerce(args[0].clone())?;
                    Fill::LinearGradient(gradient)
                }
                "RadialGradient" => {
                    let gradient = RadialGradient::try_coerce(args[0].clone())?;
                    Fill::RadialGradient(gradient)
                }
                _ => return Err(format!("{:?} can't be coerced into a Fill", pax_value)),
            },
            PaxValue::Option(mut o) => {
                if let Some(o) = o.take() {
                    Fill::try_coerce(o)?
                } else {
                    return Err(format!("{:?} can't be coerced into a Fill", pax_value));
                }
            }
            _ => return Err(format!("{:?} can't be coerced into a Fill", pax_value)),
        })
    }
}

impl CoercionRules for LinearGradient {
    fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
        Ok(match pax_value.clone() {
            PaxValue::Object(map) => {
                let start = map.get("start").unwrap().clone();
                let (s1, s2) = match start {
                    PaxValue::Vec(vec) => {
                        let s1 = Size::try_coerce(vec[0].clone())?;
                        let s2 = Size::try_coerce(vec[1].clone())?;
                        (s1, s2)
                    }
                    _ => {
                        return Err(format!(
                            "{:?} can't be coerced into a LinearGradient",
                            pax_value
                        ))
                    }
                };

                let end = map.get("end").unwrap().clone();
                let (e1, e2) = match end {
                    PaxValue::Vec(vec) => {
                        let e1 = Size::try_coerce(vec[0].clone())?;
                        let e2 = Size::try_coerce(vec[1].clone())?;
                        (e1, e2)
                    }
                    _ => {
                        return Err(format!(
                            "{:?} can't be coerced into a LinearGradient",
                            pax_value
                        ))
                    }
                };
                let stops = Vec::<GradientStop>::try_coerce(map.get("stops").unwrap().clone())?;
                LinearGradient {
                    start: (s1, s2),
                    end: (e1, e2),
                    stops,
                }
            }
            PaxValue::Option(mut o) => {
                if let Some(o) = o.take() {
                    LinearGradient::try_coerce(o)?
                } else {
                    return Err(format!(
                        "{:?} can't be coerced into a LinearGradient",
                        pax_value
                    ));
                }
            }
            _ => {
                return Err(format!(
                    "{:?} can't be coerced into a LinearGradient",
                    pax_value
                ))
            }
        })
    }
}

impl CoercionRules for RadialGradient {
    fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
        Ok(match pax_value.clone() {
            PaxValue::Object(map) => {
                let start = map.get("start").unwrap().clone();
                let (s1, s2) = match start {
                    PaxValue::Vec(vec) => {
                        let s1 = Size::try_coerce(vec[0].clone())?;
                        let s2 = Size::try_coerce(vec[1].clone())?;
                        (s1, s2)
                    }
                    _ => {
                        return Err(format!(
                            "{:?} can't be coerced into a RadialGradient",
                            pax_value
                        ))
                    }
                };

                let end = map.get("end").unwrap().clone();
                let (e1, e2) = match end {
                    PaxValue::Vec(vec) => {
                        let e1 = Size::try_coerce(vec[0].clone())?;
                        let e2 = Size::try_coerce(vec[1].clone())?;
                        (e1, e2)
                    }
                    _ => {
                        return Err(format!(
                            "{:?} can't be coerced into a RadialGradient",
                            pax_value
                        ))
                    }
                };
                let radius = match map.get("radius").unwrap().clone() {
                    PaxValue::Numeric(n) => n.to_float(),
                    _ => {
                        return Err(format!(
                            "{:?} can't be coerced into a RadialGradient",
                            pax_value
                        ))
                    }
                };
                let stops = Vec::<GradientStop>::try_coerce(map.get("stops").unwrap().clone())?;
                RadialGradient {
                    start: (s1, s2),
                    end: (e1, e2),
                    radius,
                    stops,
                }
            }
            PaxValue::Option(mut o) => {
                if let Some(o) = o.take() {
                    RadialGradient::try_coerce(o)?
                } else {
                    return Err(format!(
                        "{:?} can't be coerced into a RadialGradient",
                        pax_value
                    ));
                }
            }
            _ => {
                return Err(format!(
                    "{:?} can't be coerced into a RadialGradient",
                    pax_value
                ))
            }
        })
    }
}

impl CoercionRules for GradientStop {
    fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
        Ok(match pax_value {
            PaxValue::Object(map) => {
                let position = Size::try_coerce(map.get("position").unwrap().clone())?;
                let color = Color::try_coerce(map.get("color").unwrap().clone())?;
                GradientStop { position, color }
            }
            PaxValue::Option(mut o) => {
                if let Some(o) = o.take() {
                    GradientStop::try_coerce(o)?
                } else {
                    return Err(format!("None can't be coerced into a GradientStop"));
                }
            }
            _ => {
                return Err(format!(
                    "{:?} can't be coerced into a GradientStop",
                    pax_value
                ))
            }
        })
    }
}

impl CoercionRules for Stroke {
    fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
        Ok(match pax_value {
            PaxValue::Color(color) => Stroke {
                color: Property::new(color),
                width: Property::new(Size::Pixels(1.into())),
            },
            PaxValue::Object(map) => {
                let color = Property::new(Color::try_coerce(map.get("color").unwrap().clone())?);
                let width = Property::new(Size::try_coerce(map.get("width").unwrap().clone())?);
                Stroke { color, width }
            }
            PaxValue::Option(mut o) => {
                if let Some(o) = o.take() {
                    Stroke::try_coerce(o)?
                } else {
                    return Err(format!("None can't be coerced into a Stroke"));
                }
            }
            _ => return Err(format!("{:?} can't be coerced into a Stroke", pax_value)),
        })
    }
}

impl CoercionRules for ColorChannel {
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        Ok(match value.clone() {
            PaxValue::Rotation(rot) => ColorChannel::Rotation(rot),
            PaxValue::Percent(perc) => ColorChannel::Percent(perc.0),
            PaxValue::Numeric(num) => ColorChannel::Integer(num),
            PaxValue::Enum(variant, args) => match variant.as_str() {
                "Rotation" => {
                    let rot = Rotation::try_coerce(args[0].clone())?;
                    ColorChannel::Rotation(rot)
                }
                "Integer" => {
                    let num = Numeric::try_coerce(args[0].clone())?;
                    ColorChannel::Integer(num)
                }
                "Percent" => {
                    let num = Numeric::try_coerce(args[0].clone())?;
                    ColorChannel::Percent(num)
                }
                _ => return Err(format!("{:?} can't be coerced into a ColorChannel", value)),
            },
            PaxValue::Option(mut o) => {
                if let Some(o) = o.take() {
                    ColorChannel::try_coerce(o)?
                } else {
                    return Err(format!("{:?} can't be coerced into a ColorChannel", value));
                }
            }
            _ => return Err(format!("{:?} can't be coerced into a ColorChannel", value)),
        })
    }
}

impl CoercionRules for Percent {
    fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
        Ok(match pax_value {
            PaxValue::Percent(p) => p,
            PaxValue::Numeric(v) => Percent(v),
            PaxValue::Option(mut opt) => {
                if let Some(p) = opt.take() {
                    Percent::try_coerce(p)?
                } else {
                    return Err(format!("None can't be coerced into a Percent"));
                }
            }
            _ => return Err(format!("{:?} can't be coerced into a Percent", pax_value)),
        })
    }
}

impl CoercionRules for Size {
    fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
        Ok(match pax_value {
            PaxValue::Size(size) => size,
            PaxValue::Percent(p) => Size::Percent(p.0),
            PaxValue::Numeric(v) => Size::Pixels(v),
            PaxValue::Option(mut opt) => {
                if let Some(p) = opt.take() {
                    Size::try_coerce(p)?
                } else {
                    return Err(format!("None can't be coerced into a Size"));
                }
            }
            _ => return Err(format!("{:?} can't be coerced into a Size", pax_value)),
        })
    }
}

impl CoercionRules for String {
    fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
        Ok(match pax_value {
            PaxValue::String(s) => s,
            PaxValue::Numeric(n) => {
                if n.is_float() {
                    n.to_float().to_string()
                } else {
                    n.to_int().to_string()
                }
            }
            PaxValue::Option(mut opt) => {
                if let Some(p) = opt.take() {
                    String::try_coerce(p)?
                } else {
                    return Err(format!("None can't be coerced into a String"));
                }
            }
            _ => return Err(format!("{:?} can't be coerced into a String", pax_value)),
        })
    }
}

impl CoercionRules for Rotation {
    fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
        Ok(match pax_value {
            PaxValue::Rotation(r) => r,
            PaxValue::Numeric(n) => Rotation::Degrees(n),
            PaxValue::Percent(p) => Rotation::Percent(p.0),
            PaxValue::Option(mut opt) => {
                if let Some(p) = opt.take() {
                    Rotation::try_coerce(p)?
                } else {
                    return Err(format!("None can't be coerced into a Rotation"));
                }
            }
            _ => return Err(format!("{:?} can't be coerced into a Rotation", pax_value)),
        })
    }
}

impl CoercionRules for Numeric {
    fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
        Ok(match pax_value {
            PaxValue::Numeric(n) => n.into(),
            PaxValue::Size(n) => n.into(),
            PaxValue::Option(mut opt) => {
                if let Some(p) = opt.take() {
                    Numeric::try_coerce(p)?
                } else {
                    return Err(format!("None can't be coerced into a Numeric"));
                }
            }
            _ => return Err(format!("{:?} can't be coerced into a Numeric", pax_value)),
        })
    }
}

impl<T: CoercionRules> CoercionRules for Vec<T> {
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        match value {
            PaxValue::Vec(vec) => {
                let res: Result<Vec<T>, _> = vec.into_iter().map(|v| T::try_coerce(v)).collect();
                res.map_err(|e| format!("couldn't coerce vec, element {:?}", e))
            }
            PaxValue::Option(mut opt) => {
                if let Some(p) = opt.take() {
                    Vec::<T>::try_coerce(p)
                } else {
                    return Err(format!("None can't be coerced into a Vec"));
                }
            }
            v => Err(format!(
                "{:?} can't be coerced into {:?}",
                v,
                std::any::type_name::<Vec<T>>(),
            )),
        }
    }
}

impl<T: CoercionRules> CoercionRules for Option<T> {
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        match value {
            PaxValue::Option(opt) => {
                let res: Result<Option<T>, _> = opt.map(|v| T::try_coerce(v)).transpose();
                res.map_err(|e| format!("couldn't coerce option, element {:?}", e))
            }
            v => Err(format!(
                "{:?} can't be coerced into {:?}",
                v,
                std::any::type_name::<Option<T>>(),
            )),
        }
    }
}

impl<T: CoercionRules> CoercionRules for Range<T> {
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        match value {
            PaxValue::Range(start, end) => {
                let start = T::try_coerce(*start)?;
                let end = T::try_coerce(*end)?;
                Ok(start..end)
            }
            PaxValue::Option(mut opt) => {
                if let Some(p) = opt.take() {
                    Range::<T>::try_coerce(p)
                } else {
                    return Err(format!("None can't be coerced into a Range"));
                }
            }
            v => Err(format!(
                "{:?} can't be coerced into {:?}",
                v,
                std::any::type_name::<Range<T>>(),
            )),
        }
    }
}

impl CoercionRules for PaxValue {
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        Ok(value)
    }
}

impl<T: CoercionRules> CoercionRules for Box<T> {
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        Ok(Box::new(T::try_coerce(value)?))
    }
}

impl CoercionRules for Transform2D {
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        Ok(match value {
            PaxValue::Option(mut opt) => {
                if let Some(t) = opt.take() {
                    Transform2D::try_coerce(t)?
                } else {
                    return Err(format!("None can't be coerced into a Transform2D"));
                }
            }
            PaxValue::Object(map) => {
                let previous = match map.get("previous") {
                    Some(p) => Option::<Box<Transform2D>>::try_coerce(p.clone())?,
                    None => None,
                };
                let rotate = match map.get("rotate") {
                    Some(r) => Option::<Rotation>::try_coerce(r.clone())?,
                    None => None,
                };
                let translate = match map.get("translate") {
                    Some(t) => match t.clone() {
                        PaxValue::Option(mut opt) => {
                            if let Some(t) = opt.take() {
                                let t = Vec::<Size>::try_coerce(t.clone())?;
                                if t.len() != 2 {
                                    return Err(format!(
                                        "expected 2 elements in translate, got {:?}",
                                        t.len()
                                    ));
                                }
                                Some([t[0], t[1]])
                            } else {
                                None
                            }
                        }
                        _ => return Err(format!("{:?} can't be coerced into a Transform2D", t)),
                    },
                    None => None,
                };
                let anchor = match map.get("anchor") {
                    Some(a) => match a.clone() {
                        PaxValue::Option(mut opt) => {
                            if let Some(a) = opt.take() {
                                let a = Vec::<Size>::try_coerce(a.clone())?;
                                if a.len() != 2 {
                                    return Err(format!(
                                        "expected 2 elements in anchor, got {:?}",
                                        a.len()
                                    ));
                                }
                                Some([a[0], a[1]])
                            } else {
                                None
                            }
                        }
                        _ => return Err(format!("{:?} can't be coerced into a Transform2D", a)),
                    },
                    None => None,
                };
                let scale = match map.get("scale") {
                    Some(s) => match s.clone() {
                        PaxValue::Option(mut opt) => {
                            if let Some(s) = opt.take() {
                                let s = Vec::<Size>::try_coerce(s.clone())?;
                                if s.len() != 2 {
                                    return Err(format!(
                                        "expected 2 elements in scale, got {:?}",
                                        s.len()
                                    ));
                                }
                                Some([s[0], s[1]])
                            } else {
                                None
                            }
                        }
                        _ => return Err(format!("{:?} can't be coerced into a Transform2D", s)),
                    },
                    None => None,
                };
                let skew = match map.get("skew") {
                    Some(s) => match s.clone() {
                        PaxValue::Option(mut opt) => {
                            if let Some(s) = opt.take() {
                                let s = Vec::<Rotation>::try_coerce(s.clone())?;
                                if s.len() != 2 {
                                    return Err(format!(
                                        "expected 2 elements in skew, got {:?}",
                                        s.len()
                                    ));
                                }
                                Some([s[0], s[1]])
                            } else {
                                None
                            }
                        }
                        _ => return Err(format!("{:?} can't be coerced into a Transform2D", s)),
                    },
                    None => None,
                };
                Transform2D {
                    previous,
                    rotate,
                    translate,
                    anchor,
                    scale,
                    skew,
                }
            }
            _ => return Err(format!("{:?} can't be coerced into a Transform2D", value)),
        })
    }
}
