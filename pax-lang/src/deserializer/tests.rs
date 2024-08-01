#![allow(dead_code, unused_imports)]

use pax_runtime_api::{
    pax_value::{CoercionRules, ToFromPaxAny},
    Color, ColorChannel, ImplToFromPaxAny, Numeric, PaxValue, Percent, Rotation, Size,
};
use serde::Deserialize;

use crate::deserializer::from_pax;

#[test]
fn test_number() {
    let num_pax = "10".to_string();
    let expected = PaxValue::Numeric(Numeric::I64(10));
    let v = from_pax(&num_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_pixels() {
    let pixels_pax = "10px".to_string();
    let expected = PaxValue::Size(Size::Pixels(Numeric::I64(10)));
    let v = from_pax(&pixels_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_percent() {
    let percent_pax = "10.21%".to_string();
    let expected = PaxValue::Percent(Percent(Numeric::F64(10.21)));
    let v = from_pax(&percent_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_degrees() {
    let deg_pax = "10deg".to_string();
    let expected = PaxValue::Rotation(Rotation::Degrees(Numeric::I64(10)));
    let v = from_pax(&deg_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_radians() {
    let radians_pax = "10rad".to_string();
    let expected = PaxValue::Rotation(Rotation::Radians(Numeric::I64(10)));
    let v = from_pax(&radians_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_string() {
    let string_pax = "\"hello\"".to_string();
    let expected = PaxValue::String("hello".to_string());
    let v = from_pax(&string_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_boolean() {
    let boolean_pax = "true".to_string();
    let expected = PaxValue::Bool(true);
    let v = from_pax(&boolean_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_option() {
    let option_pax = "Some(10)".to_string();
    let expected = PaxValue::Option(Box::new(Some(PaxValue::Numeric(Numeric::I64(10)))));
    let v = from_pax(&option_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_color() {
    let color_pax = "rgba(100%, 180deg, 100, 100%)".to_string();
    let expected = PaxValue::Color(Color::rgba(
        ColorChannel::Percent(Numeric::I64(100)),
        ColorChannel::Rotation(Rotation::Degrees(Numeric::I64(180))),
        ColorChannel::Integer(Numeric::I64(100)),
        ColorChannel::Percent(Numeric::I64(100)),
    ));
    let v = from_pax(&color_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_vec() {
    let vec_pax = "[10, 20, 30]".to_string();
    let expected = PaxValue::Vec(vec![
        PaxValue::Numeric(Numeric::I64(10)),
        PaxValue::Numeric(Numeric::I64(20)),
        PaxValue::Numeric(Numeric::I64(30)),
    ]);
    let v = from_pax(&vec_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_enum() {
    let enum_pax = "Test::Enum(10, 20, 30)".to_string();
    let expected = PaxValue::Enum("Enum".to_string(), vec![
        PaxValue::Numeric(Numeric::I64(10)),
        PaxValue::Numeric(Numeric::I64(20)),
        PaxValue::Numeric(Numeric::I64(30)),
    ]);
    let v = from_pax(&enum_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_object() {
    let object_pax = "{ a: 10, b: 20 }".to_string();
    let expected = PaxValue::Object(
        vec![
            ("a".to_string(), PaxValue::Numeric(Numeric::I64(10))),
            ("b".to_string(), PaxValue::Numeric(Numeric::I64(20))),
        ]
        .into_iter()
        .collect(),
    );
    let v = from_pax(&object_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_complex_vec() {
    let vec_pax = "[10, 20, 30, [40, 50], { a: 60 }]".to_string();
    let expected = PaxValue::Vec(vec![
        PaxValue::Numeric(Numeric::I64(10)),
        PaxValue::Numeric(Numeric::I64(20)),
        PaxValue::Numeric(Numeric::I64(30)),
        PaxValue::Vec(vec![PaxValue::Numeric(Numeric::I64(40)), PaxValue::Numeric(Numeric::I64(50))]),
        PaxValue::Object(
            vec![("a".to_string(), PaxValue::Numeric(Numeric::I64(60)))].into_iter().collect()
        )]);
    let v = from_pax(&vec_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_complex_enum() {
    let enum_pax = "Test::Enum(10, 20, 30, [40, 50], { a: 60 })".to_string();
    let expected = PaxValue::Enum("Enum".to_string(), vec![
        PaxValue::Numeric(Numeric::I64(10)),
        PaxValue::Numeric(Numeric::I64(20)),
        PaxValue::Numeric(Numeric::I64(30)),
        PaxValue::Vec(vec![PaxValue::Numeric(Numeric::I64(40)), PaxValue::Numeric(Numeric::I64(50))]),
        PaxValue::Object(
            vec![("a".to_string(), PaxValue::Numeric(Numeric::I64(60)))].into_iter().collect()
        )]);
    let v = from_pax(&enum_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_complex_object() {
    let object_pax = "{ a: 10.0, b: 20, c: [30, 40], d: { e: 50 } }".to_string();
    let expected = PaxValue::Object(
        vec![
            ("a".to_string(), PaxValue::Numeric(Numeric::F64(10.0))),
            ("b".to_string(), PaxValue::Numeric(Numeric::I64(20))),
            ("c".to_string(), PaxValue::Vec(vec![PaxValue::Numeric(Numeric::I64(30)), PaxValue::Numeric(Numeric::I64(40))])),
            ("d".to_string(), PaxValue::Object(
                vec![("e".to_string(), PaxValue::Numeric(Numeric::I64(50)))].into_iter().collect()
            )),
        ]
        .into_iter()
        .collect(),
    );
    let v = from_pax(&object_pax).unwrap();
    assert_eq!(expected, v);
}