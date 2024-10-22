// ------------------------------- Coercion rules ----------------------------------
// default coercion only allows a single type: the type expected
// custom coercion rules can be implemented by a type

use std::ops::Range;

use crate::{
    impl_default_coercion_rule,
    math::{Transform2, Vector2},
    Color, ColorChannel, Fill, GradientStop, LinearGradient, Numeric, PathElement, PaxValue,
    Percent, Property, RadialGradient, Rotation, Size, Stroke, Transform2D,
};

// Default coercion rules:
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

pub trait CoercionRules
where
    Self: Sized + 'static,
{
    fn try_coerce(value: PaxValue) -> Result<Self, String>;
}

// #[allow(non_camel_case_types)]
// #[derive(Default, Clone, Serialize, Deserialize, Debug, PartialEq)]
// pub enum Color {
//     /// Models a color in the RGB space, with an alpha channel of 100%
//     rgb(ColorChannel, ColorChannel, ColorChannel),
//     /// Models a color in the RGBA space
//     rgba(ColorChannel, ColorChannel, ColorChannel, ColorChannel),

//     /// Models a color in the HSL space.
//     hsl(Rotation, ColorChannel, ColorChannel),
//     /// Models a color in the HSLA space.
//     hsla(Rotation, ColorChannel, ColorChannel, ColorChannel),

//     #[default]
//     SLATE,
//     GRAY,
//     ZINC,
//     NEUTRAL,
//     STONE,
//     RED,
//     ORANGE,
//     AMBER,
//     YELLOW,
//     LIME,
//     GREEN,
//     EMERALD,
//     TEAL,
//     CYAN,
//     SKY,
//     BLUE,
//     INDIGO,
//     VIOLET,
//     PURPLE,
//     FUCHSIA,
//     PINK,
//     ROSE,
//     BLACK,
//     WHITE,
//     TRANSPARENT,
//     NONE,
// }

impl CoercionRules for Color {
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        match value {
            PaxValue::Color(color) => Ok(*color),
            _ => return Err(format!("{:?} can't be coerced into a Color", value)),
        }
    }
}

impl CoercionRules for PathElement {
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        match value {
            PaxValue::PathElement(path_elem) => Ok(*path_elem),
            // Why is this needed? should never deserialize a path into this
            PaxValue::Enum(contents) => {
                let (name, variant, values) = *contents;
                if name == "PathElement" {
                    let mut values_itr = values.into_iter();
                    match variant.as_str() {
                        "Line" => Ok(PathElement::Line),
                        "Close" => Ok(PathElement::Close),
                        "Empty" => Ok(PathElement::Empty),
                        "Point" => Ok(PathElement::Point(
                            Size::try_coerce(values_itr.next().unwrap())?,
                            Size::try_coerce(values_itr.next().unwrap())?,
                        )),
                        "Quadratic" => Ok(PathElement::Quadratic(
                            Size::try_coerce(values_itr.next().unwrap())?,
                            Size::try_coerce(values_itr.next().unwrap())?,
                        )),
                        "Cubic" => Ok(PathElement::Cubic(
                            Size::try_coerce(values_itr.next().unwrap())?,
                            Size::try_coerce(values_itr.next().unwrap())?,
                            Size::try_coerce(values_itr.next().unwrap())?,
                            Size::try_coerce(values_itr.next().unwrap())?,
                        )),
                        _ => {
                            return Err(format!(
                                "failed to coerce PathElement: unknown enum variant: {:?}",
                                variant
                            ))
                        }
                    }
                } else {
                    return Err(format!(
                        "failed to coerce PathElement: enum name doesn't match"
                    ));
                }
            }
            _ => return Err(format!("failed to coerce PathElement: PaxValue not a path")),
        }
    }
}

// Fill is a type that other types (Color) can be coerced into, thus the default
// from to pax macro isn't enough (only translates directly back and forth, and returns
// an error if it contains any other type)
impl CoercionRules for Fill {
    fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
        Ok(match pax_value {
            PaxValue::Color(color) => Fill::Solid(*color),
            PaxValue::Enum(contents) => {
                let (_, variant, args) = *contents;
                match variant.as_str() {
                    "Solid" => {
                        let color = Color::try_coerce(args.into_iter().next().unwrap())?;
                        Fill::Solid(color)
                    }
                    "LinearGradient" => {
                        let gradient =
                            LinearGradient::try_coerce(args.into_iter().next().unwrap())?;
                        Fill::LinearGradient(gradient)
                    }
                    "RadialGradient" => {
                        let gradient =
                            RadialGradient::try_coerce(args.into_iter().next().unwrap())?;
                        Fill::RadialGradient(gradient)
                    }
                    _ => {
                        return Err(format!(
                            "failed to coerce Fill: unknown enum variant {:?}",
                            variant
                        ))
                    }
                }
            }
            PaxValue::Option(o) => {
                if let Some(o) = *o {
                    Fill::try_coerce(o)?
                } else {
                    return Err(format!("failed to coerce Fill: can't coerce None"));
                }
            }
            _ => return Err(format!("{:?} can't be coerced into a Fill", pax_value)),
        })
    }
}

impl CoercionRules for LinearGradient {
    fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
        Ok(match pax_value {
            PaxValue::Object(map) => {
                let [start, end, stops] = extract_options(["start", "end", "stops"], map)
                    .map_err(|e| format!("failed to convert to LinearGradient: {e}"))?;
                let (s1, s2) = match start {
                    PaxValue::Vec(vec) => {
                        let mut itr = vec.into_iter();
                        let s1 = Size::try_coerce(itr.next().unwrap())?;
                        let s2 = Size::try_coerce(itr.next().unwrap())?;
                        (s1, s2)
                    }
                    _ => return Err(format!("failed to coerce LinearGradient")),
                };

                let (e1, e2) = match end {
                    PaxValue::Vec(vec) => {
                        let mut itr = vec.into_iter();
                        let e1 = Size::try_coerce(itr.next().unwrap())?;
                        let e2 = Size::try_coerce(itr.next().unwrap())?;
                        (e1, e2)
                    }
                    _ => return Err(format!("failed to coerce LinearGradient")),
                };
                let stops = Vec::<GradientStop>::try_coerce(stops)?;
                LinearGradient {
                    start: (s1, s2),
                    end: (e1, e2),
                    stops,
                }
            }
            PaxValue::Option(o) => {
                if let Some(o) = *o {
                    LinearGradient::try_coerce(o)?
                } else {
                    return Err(format!("failed to coerce LinearGradient"));
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
        Ok(match pax_value {
            PaxValue::Object(map) => {
                let [start, end, stops, radius] =
                    extract_options(["start", "end", "stops", "radius"], map)
                        .map_err(|e| format!("failed to convert to RadialGradient: {e}"))?;
                let (s1, s2) = match start {
                    PaxValue::Vec(vec) => {
                        let mut itr = vec.into_iter();
                        let s1 = Size::try_coerce(itr.next().unwrap())?;
                        let s2 = Size::try_coerce(itr.next().unwrap())?;
                        (s1, s2)
                    }
                    _ => {
                        return Err(format!("failed to coerce RadialGradient"));
                    }
                };

                let (e1, e2) = match end {
                    PaxValue::Vec(vec) => {
                        let mut itr = vec.into_iter();
                        let e1 = Size::try_coerce(itr.next().unwrap())?;
                        let e2 = Size::try_coerce(itr.next().unwrap())?;
                        (e1, e2)
                    }
                    _ => {
                        return Err(format!("failed to coerce RadialGradient"));
                    }
                };
                let radius = match radius {
                    PaxValue::Numeric(n) => n.to_float(),
                    _ => {
                        return Err(format!("failed to coerce RadialGradient"));
                    }
                };
                let stops = Vec::<GradientStop>::try_coerce(stops)?;
                RadialGradient {
                    start: (s1, s2),
                    end: (e1, e2),
                    radius,
                    stops,
                }
            }
            PaxValue::Option(o) => {
                if let Some(o) = *o {
                    RadialGradient::try_coerce(o)?
                } else {
                    return Err(format!("failed to coerce RadialGradient"));
                }
            }
            _ => {
                return Err(format!("failed to coerce RadialGradient"));
            }
        })
    }
}

impl CoercionRules for GradientStop {
    fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
        Ok(match pax_value {
            PaxValue::Object(map) => {
                let [position, color] = extract_options(["position", "color"], map)
                    .map_err(|e| format!("failed to convert to GradientStop: {e}"))?;
                let position = Size::try_coerce(position)?;
                let color = Color::try_coerce(color)?;
                GradientStop { position, color }
            }
            PaxValue::Option(o) => {
                if let Some(o) = *o {
                    GradientStop::try_coerce(o)?
                } else {
                    return Err(format!("failed to convert to GradientStop"));
                }
            }
            _ => {
                return Err(format!("failed to convert to GradientStop"));
            }
        })
    }
}

impl CoercionRules for Stroke {
    fn try_coerce(pax_value: PaxValue) -> Result<Self, String> {
        Ok(match pax_value {
            PaxValue::Color(color) => Stroke {
                color: Property::new(*color),
                width: Property::new(Size::Pixels(1.into())),
            },
            PaxValue::Object(map) => {
                let [color, width] = extract_options(["color", "width"], map)
                    .map_err(|e| format!("failed to convert to Stroke: {e}"))?;
                let color = Property::new(Color::try_coerce(color)?);
                let width = Property::new(Size::try_coerce(width)?);
                Stroke { color, width }
            }
            PaxValue::Option(o) => {
                if let Some(o) = *o {
                    Stroke::try_coerce(o)?
                } else {
                    return Err(format!("failed to convert to Stroke"));
                }
            }
            _ => return Err(format!("failed to convert to Stroke")),
        })
    }
}

impl CoercionRules for ColorChannel {
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        Ok(match value {
            PaxValue::Rotation(rot) => ColorChannel::Rotation(rot),
            PaxValue::Percent(perc) => ColorChannel::Percent(perc.0),
            PaxValue::Numeric(num) => ColorChannel::Integer(num.to_int().clamp(0, 255) as u8),
            PaxValue::Enum(contents) => {
                let (_, variant, args) = *contents;
                match variant.as_str() {
                    "Rotation" => {
                        let rot = Rotation::try_coerce(args.into_iter().next().unwrap())?;
                        ColorChannel::Rotation(rot)
                    }
                    "Integer" => {
                        let num = Numeric::try_coerce(args.into_iter().next().unwrap())?;
                        ColorChannel::Integer(num.to_int().clamp(0, 255) as u8)
                    }
                    "Percent" => {
                        let num = Numeric::try_coerce(args.into_iter().next().unwrap())?;
                        ColorChannel::Percent(num)
                    }
                    _ => {
                        return Err(format!(
                            "failed to convert to ColorChannel: unknown variant {:?}",
                            variant
                        ))
                    }
                }
            }
            PaxValue::Option(o) => {
                if let Some(o) = *o {
                    ColorChannel::try_coerce(o)?
                } else {
                    return Err(format!("failed to convert to ColorChannel"));
                }
            }
            _ => return Err(format!("failed to convert to ColorChannel")),
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
            PaxValue::Bool(b) => (b as i32).into(),
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

impl<T1: CoercionRules, T2: CoercionRules> CoercionRules for (T1, T2) {
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        match value {
            PaxValue::Vec(vec) => {
                let mut itr = vec.into_iter();
                let res: Result<T1, _> = T1::try_coerce(itr.next().unwrap());
                let res2: Result<T2, _> = T2::try_coerce(itr.next().unwrap());
                res.and_then(|v1| res2.map(|v2| (v1, v2)))
            }
            PaxValue::Option(opt) => {
                if let Some(p) = *opt {
                    <(T1, T2)>::try_coerce(p)
                } else {
                    return Err(format!("None can't be coerced into a Vec"));
                }
            }
            v => Err(format!(
                "{:?} can't be coerced into {:?}",
                v,
                std::any::type_name::<(T1, T2)>(),
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
            v => Some(T::try_coerce(v)).transpose(),
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
            PaxValue::Option(opt) => {
                if let Some(t) = *opt {
                    Transform2D::try_coerce(t)?
                } else {
                    return Err(format!("None can't be coerced into a Transform2D"));
                }
            }
            PaxValue::Object(map) => {
                let [previous, rotate, translate, anchor, scale, skew] = extract_options(
                    ["previous", "rotate", "translate", "anchor", "scale", "skew"],
                    map,
                )
                .map_err(|e| format!("failed to convert to Transform2D: {e}"))?;
                let previous = Option::<Box<Transform2D>>::try_coerce(previous)?;
                let rotate = Option::<Rotation>::try_coerce(rotate)?;
                let translate = match translate {
                    PaxValue::Option(opt) => {
                        if let Some(t) = *opt {
                            let t = Vec::<Size>::try_coerce(t)?;
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
                    _ => return Err(format!("translate can't be coerced into a Transform2D",)),
                };
                let anchor = match anchor {
                    PaxValue::Option(opt) => {
                        if let Some(a) = *opt {
                            let a = Vec::<Size>::try_coerce(a)?;
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
                    _ => return Err(format!("anchor can't be coerced into a Transform2D",)),
                };
                let scale = match scale {
                    PaxValue::Option(opt) => {
                        if let Some(s) = *opt {
                            let s = Vec::<Size>::try_coerce(s)?;
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
                    _ => return Err(format!("scale can't be coerced into a Transform2D",)),
                };
                let skew = match skew {
                    PaxValue::Option(opt) => {
                        if let Some(s) = *opt {
                            let s = Vec::<Rotation>::try_coerce(s)?;
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
                    _ => return Err(format!("skew can't be coerced into a Transform2D",)),
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

impl CoercionRules for Transform2 {
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        Ok(match value {
            PaxValue::Option(mut opt) => {
                if let Some(t) = opt.take() {
                    Transform2::try_coerce(t)?
                } else {
                    return Err(format!("None can't be coerced into a Transform2"));
                }
            }
            PaxValue::Object(map) => {
                let m = Vec::<f64>::try_coerce(
                    map.into_iter()
                        .find_map(|(n, v)| (n == "m").then_some(v))
                        .unwrap(),
                )?;
                if m.len() != 6 {
                    return Err(format!("expected 6 elements in coeffs, got {:?}", m.len()));
                }
                Transform2::new([m[0], m[1], m[2], m[3], m[4], m[5]])
            }
            _ => return Err(format!("{:?} can't be coerced into a Transform2", value)),
        })
    }
}

impl CoercionRules for Vector2 {
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        Ok(match value {
            PaxValue::Option(mut opt) => {
                if let Some(t) = opt.take() {
                    Vector2::try_coerce(t)?
                } else {
                    return Err(format!("None can't be coerced into a Vector2"));
                }
            }
            PaxValue::Object(map) => {
                let [x, y] = extract_options(["x", "y"], map)
                    .map_err(|e| format!("failed to convert to Vector2: {e}"))?;
                let x = f64::try_coerce(x)?;
                let y = f64::try_coerce(y)?;
                Vector2::new(x, y)
            }
            _ => return Err(format!("{:?} can't be coerced into a Vector2", value)),
        })
    }
}

pub fn extract_options<T, const N: usize>(
    keys: [&'static str; N],
    vec: Vec<(String, T)>,
) -> Result<[T; N], String> {
    // First create array of Options
    let mut intermediate: [Option<T>; N] = std::array::from_fn(|_| None);

    // Fill in the values we find
    for (k, v) in vec {
        if let Some(pos) = keys.iter().position(|&key| k == key) {
            intermediate[pos] = Some(v);
        }
    }

    // Convert to Result, ensuring all values were found
    let result: Result<[T; N], String> = intermediate
        .into_iter()
        .enumerate()
        .map(|(i, opt)| opt.ok_or_else(|| format!("missing field: {}", keys[i])))
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(|_| "Internal error converting vec to array".to_string());

    result
}
