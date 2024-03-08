#![allow(dead_code, unused_imports)]

use serde::Deserialize;

use crate::deserializer::from_pax;

#[test]
fn test_number() {
    #[derive(Deserialize, PartialEq, Debug)]
    pub enum Numeric {
        Integer(isize),
        Float(f64),
    }

    let num_pax = "10".to_string();
    let expected = Numeric::Integer(10);
    let v = from_pax(&num_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_pixels() {
    #[derive(Deserialize, PartialEq, Debug)]
    pub enum Numeric {
        Integer(isize),
        Float(f64),
    }
    #[derive(Deserialize, PartialEq, Debug)]
    pub enum Size {
        Pixels(Numeric),
        Percent(Numeric),
    }

    let pixels_pax = "10px".to_string();
    let expected = Size::Pixels(Numeric::Integer(10));
    let v = from_pax(&pixels_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_percent() {
    #[derive(Deserialize, PartialEq, Debug)]
    pub enum Numeric {
        Integer(isize),
        Float(f64),
    }
    #[derive(Deserialize, PartialEq, Debug)]
    pub enum Size {
        Pixels(Numeric),
        Percent(Numeric),
    }

    let percent_pax = "10%".to_string();
    let expected = Size::Percent(Numeric::Integer(10));
    let v = from_pax(&percent_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_degrees() {
    #[derive(Deserialize, PartialEq, Debug)]
    pub enum Numeric {
        Integer(isize),
        Float(f64),
    }
    #[derive(Deserialize, PartialEq, Debug)]
    pub enum Rotation {
        Radians(Numeric),
        Degrees(Numeric),
        Percent(Numeric),
    }

    let radians_pax = "10deg".to_string();
    let expected = Rotation::Degrees(Numeric::Integer(10));
    let v = from_pax(&radians_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_radians() {
    #[derive(Deserialize, PartialEq, Debug)]
    pub enum Numeric {
        Integer(isize),
        Float(f64),
    }
    #[derive(Deserialize, PartialEq, Debug)]
    pub enum Rotation {
        Radians(Numeric),
        Degrees(Numeric),
        Percent(Numeric),
    }

    let radians_pax = "10rad".to_string();
    let expected = Rotation::Radians(Numeric::Integer(10));
    let v = from_pax(&radians_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_string_box() {
    #[derive(Deserialize, PartialEq, Debug)]
    pub struct StringBox {
        pub string: String,
    }

    let string_box_pax = "\"hello\"".to_string();
    let expected = StringBox {
        string: "hello".to_string(),
    };
    let v = from_pax(&string_box_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_tuple() {
    #[derive(Deserialize, PartialEq, Debug)]
    pub struct StringBox {
        pub string: String,
    }
    #[derive(Deserialize, PartialEq, Debug)]
    pub enum Numeric {
        Integer(isize),
        Float(f64),
    }

    let tuple_pax = "(\"hello\", 10)".to_string();
    let expected = (
        StringBox {
            string: "hello".to_string(),
        },
        Numeric::Integer(10),
    );
    let v = from_pax(&tuple_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_enum() {
    #[derive(Deserialize, PartialEq, Debug)]
    pub enum Numeric {
        Integer(isize),
        Float(f64),
    }
    #[derive(Deserialize, PartialEq, Debug)]
    pub enum Color {
        Rgba(Numeric, Numeric, Numeric, Numeric),
    }

    let enum_pax = "Color::Rgba(0.0, 0.0, 1.0, 1.0)".to_string();
    let expected = Color::Rgba(
        Numeric::Float(0.0),
        Numeric::Float(0.0),
        Numeric::Float(1.0),
        Numeric::Float(1.0),
    );
    let v = from_pax(&enum_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_boolean() {
    let boolean_pax = "true".to_string();
    let expected = true;
    let v: bool = from_pax(&boolean_pax).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_object() {
    #[derive(Deserialize, PartialEq, Debug)]
    pub enum Numeric {
        Integer(isize),
        Float(f64),
    }
    #[derive(Deserialize, PartialEq, Debug)]
    pub struct Example {
        pub x_px: Numeric,
        pub y_px: Numeric,
        pub width_px: Numeric,
        pub height_px: Numeric,
    }

    let object_pax = "{x_px: 10.0, y_px: 10, width_px: 10.0, height_px: 10}".to_string();

    let expected = Example {
        x_px: Numeric::Float(10.0),
        y_px: Numeric::Integer(10),
        width_px: Numeric::Float(10.0),
        height_px: Numeric::Integer(10),
    };

    let v = from_pax(&object_pax).unwrap();
    assert_eq!(expected, v);
}
