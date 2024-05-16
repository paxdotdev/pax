#![allow(dead_code, unused_imports)]

use pax_runtime_api::{
    pax_value::{CoercionRules, ToFromPaxAny},
    Color, ImplToFromPaxAny, Numeric, Rotation, Size,
};
use serde::Deserialize;

use crate::deserializer::from_pax;

#[test]
fn test_number() {
    let num_pax = "10".to_string();
    let expected = Numeric::I64(10);
    let v = Numeric::from_pax_any(from_pax::<Numeric>(&num_pax).unwrap()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_pixels() {
    let pixels_pax = "10px".to_string();
    let expected = Size::Pixels(Numeric::I64(10));
    let v = Size::from_pax_any(from_pax::<Size>(&pixels_pax).unwrap()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_percent() {
    let percent_pax = "10.21%".to_string();
    let expected = Size::Percent(Numeric::F64(10.21));
    let v = Size::from_pax_any(from_pax::<Size>(&percent_pax).unwrap()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_degrees() {
    let deg_pax = "10deg".to_string();
    let expected = Rotation::Degrees(Numeric::I64(10));
    let v = Rotation::from_pax_any(from_pax::<Rotation>(&deg_pax).unwrap()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_radians() {
    let radians_pax = "10rad".to_string();
    let expected = Rotation::Radians(Numeric::I64(10));
    let v = Rotation::from_pax_any(from_pax::<Rotation>(&radians_pax).unwrap()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_string() {
    let string_pax = "\"hello\"".to_string();
    let expected = String::from("hello");
    let v = String::from_pax_any(from_pax::<String>(&string_pax).unwrap()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_tuple() {
    let tuple_pax = "(\"hello\", 10)".to_string();
    let expected = (String::from("hello"), Numeric::I64(10));
    let v = <(String, Numeric)>::from_pax_any(from_pax::<(String, Numeric)>(&tuple_pax).unwrap())
        .unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_enum() {
    let enum_pax = "rgba(0, 0, 1, 1)".to_string();
    let expected = Color::rgba(
        Numeric::I64(0).into(),
        Numeric::I64(0).into(),
        Numeric::I64(1).into(),
        Numeric::I64(1).into(),
    );
    let v = Color::from_pax_any(from_pax::<Color>(&enum_pax).unwrap()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_boolean() {
    let boolean_pax = "true".to_string();
    let expected = true;
    let v = bool::from_pax_any(from_pax::<bool>(&boolean_pax).unwrap()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_object() {
    #[derive(Deserialize, PartialEq, Debug, Clone)]
    pub struct Example {
        pub x_px: Numeric,
        pub y_px: Numeric,
        pub width_px: Numeric,
        pub height_px: Numeric,
    }

    impl ImplToFromPaxAny for Example {}

    let object_pax = "{x_px: 10.0, y_px: 10, width_px: 10.0, height_px: 10}".to_string();

    let expected = Example {
        x_px: Numeric::F64(10.0),
        y_px: Numeric::I64(10),
        width_px: Numeric::F64(10.0),
        height_px: Numeric::I64(10),
    };

    let v = Example::from_pax_any(from_pax::<Example>(&object_pax).unwrap()).unwrap();
    assert_eq!(expected, v);
}
