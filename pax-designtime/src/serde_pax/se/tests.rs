#![allow(dead_code, unused_imports)]
use crate::to_pax;
use serde_derive::Serialize;

#[test]
fn test_number() {
    #[derive(Serialize, PartialEq, Debug)]
    pub enum Numeric {
        Integer(isize),
        Float(f64),
    }

    let expected = "10".to_string();
    let num_pax = Numeric::Integer(10);
    let v = to_pax(&num_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_pixels() {
    #[derive(Serialize, PartialEq, Debug)]
    pub enum Numeric {
        Integer(isize),
        Float(f64),
    }
    #[derive(Serialize, PartialEq, Debug)]
    pub enum Size {
        Pixels(Numeric),
        Percent(Numeric),
    }

    let expected = "10px".to_string();
    let pixels_pax = Size::Pixels(Numeric::Integer(10));
    let v = to_pax(&pixels_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_percent() {
    #[derive(Serialize, PartialEq, Debug)]
    pub enum Numeric {
        Integer(isize),
        Float(f64),
    }
    #[derive(Serialize, PartialEq, Debug)]
    pub enum Size {
        Pixels(Numeric),
        Percent(Numeric),
    }

    let expected = "10%".to_string();
    let percent_pax = Size::Percent(Numeric::Integer(10));
    let v = to_pax(&percent_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_degrees() {
    #[derive(Serialize, PartialEq, Debug)]
    pub enum Numeric {
        Integer(isize),
        Float(f64),
    }
    #[derive(Serialize, PartialEq, Debug)]
    pub enum Size {
        Pixels(Numeric),
        Percent(Numeric),
    }

    let expected = "10%".to_string();
    let percent_pax = Size::Percent(Numeric::Integer(10));
    let v = to_pax(&percent_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_radians() {
    #[derive(Serialize, PartialEq, Debug)]
    pub enum Numeric {
        Integer(isize),
        Float(f64),
    }
    #[derive(Serialize, PartialEq, Debug)]
    pub enum Rotation {
        Radians(Numeric),
        Degrees(Numeric),
        Percent(Numeric),
    }

    let expected = "10rad".to_string();
    let radians_pax = Rotation::Radians(Numeric::Integer(10));
    let v = to_pax(&radians_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_string_box() {
    let expected = "\"hello\"".to_string();
    let string_pax = "hello".to_string();
    let v = to_pax(&string_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_tuple() {
    #[derive(Serialize, PartialEq, Debug)]
    pub enum Numeric {
        Integer(isize),
        Float(f64),
    }

    let expected = "(\"hello\", 10)".to_string();
    let tuple_pax = ("hello".to_string(), Numeric::Integer(10));
    let v = to_pax(&tuple_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_enum() {
    #[derive(Serialize, PartialEq, Debug)]
    pub enum Numeric {
        Integer(isize),
        Float(f64),
    }
    #[derive(Serialize, PartialEq, Debug)]
    pub enum Color {
        Hlca(Numeric, Numeric, Numeric, Numeric),
        Rgba(Numeric, Numeric, Numeric, Numeric),
    }
    let expected = "Color::Hlca(0.0, 0.0, 1.0, 1.0)".to_string();
    let enum_pax = Color::Hlca(
        Numeric::Float(0.00),
        Numeric::Float(0.0),
        Numeric::Float(1.0),
        Numeric::Float(1.0),
    );
    let v = to_pax(&enum_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_boolean() {
    let expected = "true".to_string();
    let boolean_pax = true;
    let v = to_pax(&boolean_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_object() {
    #[derive(Serialize, PartialEq, Debug)]
    pub enum Numeric {
        Integer(isize),
        Float(f64),
    }
    #[derive(Serialize, PartialEq, Debug)]
    pub struct Example {
        pub x_px: Numeric,
        pub y_px: Numeric,
        pub width_px: Numeric,
        pub height_px: Numeric,
    }

    let expected = "Example: {x_px: 10.0, y_px: 10, width_px: 10.0, height_px: 10}".to_string();

    let object_pax = Example {
        x_px: Numeric::Float(10.0),
        y_px: Numeric::Integer(10),
        width_px: Numeric::Float(10.0),
        height_px: Numeric::Integer(10),
    };

    let v = to_pax(&object_pax).unwrap();
    assert_eq!(expected, v);
}
