use pax_core::unsafe_unwrap;

#[derive(Debug, PartialEq)]
#[repr(C)]
enum TypesCoproduct {
    Apple(String),
    Banana(String),
}

#[test]
fn test_unwrap_apple() {
    let fruit = TypesCoproduct::Apple("green".to_string());
    let expected_color = "green".to_string();
    let (unwrapped_color, _ptr) =
        safe_unwrap!(fruit, Apple);
    assert_eq!(unwrapped_color, expected_color);
}
