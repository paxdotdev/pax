//! Representation equality for repeat payloads, not PAXEL's approximate equality.
//! Suppressing a write must preserve every value an expression can observe.

use pax_runtime_api::{Color, ColorChannel, Numeric, PathElement, PaxValue, Rotation, Size};

pub(super) fn same_value(a: &PaxValue, b: &PaxValue) -> bool {
    use PaxValue::*;
    match (a, b) {
        (Bool(a), Bool(b)) => a == b,
        (Numeric(a), Numeric(b)) => same_number(a, b),
        (String(a), String(b)) => a == b,
        (Size(a), Size(b)) => same_size(a, b),
        (Percent(a), Percent(b)) => same_number(&a.0, &b.0),
        (Color(a), Color(b)) => same_color(a, b),
        (Rotation(a), Rotation(b)) => same_rotation(a, b),
        (Duration(a), Duration(b)) => match (a, b) {
            (pax_runtime_api::Duration::Frames(a), pax_runtime_api::Duration::Frames(b))
            | (
                pax_runtime_api::Duration::Milliseconds(a),
                pax_runtime_api::Duration::Milliseconds(b),
            )
            | (pax_runtime_api::Duration::Seconds(a), pax_runtime_api::Duration::Seconds(b)) => {
                same_number(a, b)
            }
            _ => false,
        },
        (PathElement(a), PathElement(b)) => same_path_element(a, b),
        (Option(a), Option(b)) => match (a.as_ref(), b.as_ref()) {
            (Some(a), Some(b)) => same_value(a, b),
            (None, None) => true,
            _ => false,
        },
        (Vec(a), Vec(b)) => same_values(a, b),
        (Range(a0, a1), Range(b0, b1)) => same_value(a0, b0) && same_value(a1, b1),
        (Object(a), Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|((ak, av), (bk, bv))| ak == bk && same_value(av, bv))
        }
        (Enum(a), Enum(b)) => a.0 == b.0 && a.1 == b.1 && same_values(&a.2, &b.2),
        // New representations conservatively propagate until explicitly supported.
        _ => false,
    }
}

fn same_values(a: &[PaxValue], b: &[PaxValue]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same_value(a, b))
}

fn same_number(a: &Numeric, b: &Numeric) -> bool {
    use Numeric::*;
    match (a, b) {
        (I8(a), I8(b)) => a == b,
        (I16(a), I16(b)) => a == b,
        (I32(a), I32(b)) => a == b,
        (I64(a), I64(b)) => a == b,
        (U8(a), U8(b)) => a == b,
        (U16(a), U16(b)) => a == b,
        (U32(a), U32(b)) => a == b,
        (U64(a), U64(b)) => a == b,
        (ISize(a), ISize(b)) => a == b,
        (USize(a), USize(b)) => a == b,
        (F32(a), F32(b)) => a.to_bits() == b.to_bits(),
        (F64(a), F64(b)) => a.to_bits() == b.to_bits(),
        _ => false,
    }
}

fn same_size(a: &Size, b: &Size) -> bool {
    match (a, b) {
        (Size::Pixels(a), Size::Pixels(b)) | (Size::Percent(a), Size::Percent(b)) => {
            same_number(a, b)
        }
        (Size::Combined(a0, a1), Size::Combined(b0, b1)) => {
            same_number(a0, b0) && same_number(a1, b1)
        }
        _ => false,
    }
}

fn same_rotation(a: &Rotation, b: &Rotation) -> bool {
    match (a, b) {
        (Rotation::Degrees(a), Rotation::Degrees(b))
        | (Rotation::Radians(a), Rotation::Radians(b))
        | (Rotation::Percent(a), Rotation::Percent(b)) => same_number(a, b),
        _ => false,
    }
}

fn same_channel(a: &ColorChannel, b: &ColorChannel) -> bool {
    match (a, b) {
        (ColorChannel::Integer(a), ColorChannel::Integer(b)) => a == b,
        (ColorChannel::Percent(a), ColorChannel::Percent(b)) => same_number(a, b),
        (ColorChannel::Rotation(a), ColorChannel::Rotation(b)) => same_rotation(a, b),
        _ => false,
    }
}

fn same_color(a: &Color, b: &Color) -> bool {
    use Color::*;
    match (a, b) {
        (rgb(a0, a1, a2), rgb(b0, b1, b2)) => {
            same_channel(a0, b0) && same_channel(a1, b1) && same_channel(a2, b2)
        }
        (rgba(a0, a1, a2, a3), rgba(b0, b1, b2, b3)) => {
            same_channel(a0, b0)
                && same_channel(a1, b1)
                && same_channel(a2, b2)
                && same_channel(a3, b3)
        }
        (hsl(a0, a1, a2), hsl(b0, b1, b2)) => {
            same_rotation(a0, b0) && same_channel(a1, b1) && same_channel(a2, b2)
        }
        (hsla(a0, a1, a2, a3), hsla(b0, b1, b2, b3)) => {
            same_rotation(a0, b0)
                && same_channel(a1, b1)
                && same_channel(a2, b2)
                && same_channel(a3, b3)
        }
        (
            SLATE | GRAY | ZINC | NEUTRAL | STONE | RED | ORANGE | AMBER | YELLOW | LIME | GREEN
            | EMERALD | TEAL | CYAN | SKY | BLUE | INDIGO | VIOLET | PURPLE | FUCHSIA | PINK | ROSE
            | BLACK | WHITE | TRANSPARENT | NONE,
            _,
        ) => std::mem::discriminant(a) == std::mem::discriminant(b),
        _ => false,
    }
}

fn same_path_element(a: &PathElement, b: &PathElement) -> bool {
    use PathElement::*;
    match (a, b) {
        (Empty, Empty) | (Line, Line) | (Close, Close) => true,
        (Point(a0, a1), Point(b0, b1)) | (Quadratic(a0, a1), Quadratic(b0, b1)) => {
            same_size(a0, b0) && same_size(a1, b1)
        }
        (Cubic(a0, a1, a2, a3), Cubic(b0, b1, b2, b3)) => {
            same_size(a0, b0) && same_size(a1, b1) && same_size(a2, b2) && same_size(a3, b3)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pax_runtime_api::Duration;

    #[test]
    fn preserves_numeric_representation_and_small_changes() {
        let values = [
            Numeric::F64(0.0),
            Numeric::F64(-0.0),
            Numeric::F64(1e-8),
            Numeric::F32(0.0),
            Numeric::I64(0),
            Numeric::U64(0),
            Numeric::F64(f64::NAN),
            Numeric::F64(f64::from_bits(f64::NAN.to_bits() + 1)),
            Numeric::F64(f64::INFINITY),
        ];
        for (i, a) in values.iter().enumerate() {
            for (j, b) in values.iter().enumerate() {
                assert_eq!(
                    same_value(&PaxValue::Numeric(*a), &PaxValue::Numeric(*b)),
                    i == j
                );
            }
        }
    }

    fn payload(n: Numeric) -> PaxValue {
        let size = Size::Combined(n, 10.into());
        let rotation = Rotation::Degrees(n);
        let channel = ColorChannel::Percent(n);
        PaxValue::Object(vec![(
            "data".into(),
            PaxValue::Vec(vec![
                PaxValue::Size(size),
                PaxValue::Percent(pax_runtime_api::Percent(n)),
                PaxValue::Rotation(rotation),
                PaxValue::Duration(Duration::Milliseconds(n)),
                PaxValue::Color(Box::new(Color::hsla(
                    rotation,
                    channel.clone(),
                    channel.clone(),
                    channel,
                ))),
                PaxValue::PathElement(Box::new(PathElement::Cubic(size, size, size, size))),
                PaxValue::Option(Box::new(Some(PaxValue::Numeric(n)))),
                PaxValue::Range(
                    Box::new(PaxValue::Numeric(n)),
                    Box::new(PaxValue::Numeric(n)),
                ),
                PaxValue::Enum(Box::new((
                    "Test".into(),
                    "Variant".into(),
                    vec![PaxValue::Numeric(n)],
                ))),
            ]),
        )])
    }

    #[test]
    fn nested_values_are_exact_without_changing_language_equality() {
        let a = payload(Numeric::F64(1.0));
        let b = payload(Numeric::F64(1.0 + 1e-8));
        assert_eq!(a, b, "PAXEL equality remains approximate");
        assert!(same_value(&a, &a.clone()));
        assert!(!same_value(&a, &b));
        let (PaxValue::Object(a), PaxValue::Object(b)) = (a, b) else {
            unreachable!()
        };
        let (PaxValue::Vec(a), PaxValue::Vec(b)) = (&a[0].1, &b[0].1) else {
            unreachable!()
        };
        for (a, b) in a.iter().zip(b) {
            assert!(!same_value(a, b));
        }
    }
}
