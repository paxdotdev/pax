use crate::FontWeight;
use pax_engine::api::functions::print_all_functions;
use pax_engine::{CoercionRules, Property, ToPaxValue};

use crate::TextStyle;

#[test]
fn test_font_style_to_pax_value() {
    let mut expected = TextStyle::default();

    let pax_value = expected.clone().to_pax_value();
    println!("pax_value: {:?}", pax_value);
    let translated = TextStyle::try_coerce(pax_value).unwrap();
    let expected_str = format!("{:?}", expected);
    let translated_str = format!("{:?}", translated);
    // println!("expected: {:?}", expected_str);
    println!("translated: {:?}", translated_str);
    // assert_eq!(translated_str, expected_str);
}

#[test]
fn test_helper() {
    // let pv = pax_lang::compute_paxel("increase({})".to_string()).unwrap();
    // let fw = FontWeight::try_coerce(pv).unwrap();
    print_all_functions();
}
