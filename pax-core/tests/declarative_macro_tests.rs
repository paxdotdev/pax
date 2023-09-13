use pax_core::unsafe_unwrap;

#[derive(Debug, PartialEq, Default)]
#[repr(C)]
enum Fruit {
    #[default]
    None,
    Apple(String),
    Banana(String),
}

//let new_value = unsafe_unwrap!(stroke_width, TypesCoproduct, pax_runtime_api::SizePixels);

#[test]
fn test_unwrap_apple() {
    let fruit = Fruit::Apple("green".to_string());
    let expected_color = "green".to_string();
    let unwrapped_color = unsafe_unwrap!(fruit, Fruit, String);
    assert_eq!(unwrapped_color, expected_color);
}

#[test]
#[should_panic(expected = "The size_of target_type must be less than the size_of enum_type.")]
fn test_unwrap_invalid_size() {
    let fruit = Fruit::Apple("red".to_string());
    let _unwrapped_fruit = unsafe_unwrap!(fruit, Fruit, Fruit);
}
