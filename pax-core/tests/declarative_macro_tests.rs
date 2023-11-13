use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, PartialEq, Default)]
#[repr(C)]
enum PropertiesCoproductTest {
    #[default]
    None,
    Color(Color),
}

#[derive(Default, Debug, PartialEq)]
#[repr(C)]
struct Color {
    fill: String,
}

#[test]
fn test_unsafe_unwrap() {
    let wrapped = PropertiesCoproductTest::Color(Color { fill: "green".to_string() } );
    let expected_color = "green".to_string();
    let unwrapped_color = unsafe_unwrap!(wrapped, PropertiesCoproductTest, Color);
    assert_eq!(unwrapped_color.fill, expected_color);
}

#[test]
#[should_panic(expected = "The size_of target_type must be less than the size_of enum_type.")]
fn test_unsafe_unwrap_invalid_size() {
    let red = PropertiesCoproductTest::Color(Color { fill: "red".to_string() } );
    let _unwrapped = unsafe_unwrap!(red, PropertiesCoproductTest, PropertiesCoproductTest);
}

#[test]
fn test_with_properties_unsafe() {
    let fully_wrapped : Rc<RefCell<PropertiesCoproductTest>> = Rc::new(RefCell::new(PropertiesCoproductTest::Color(Color {fill: "blue".to_string()})));
    with_properties_unsafe!(&fully_wrapped, PropertiesCoproductTest, Color, |color : &mut Color| {
        color.fill = "red".to_string();
    });

    let pc_borrowed = (*fully_wrapped).borrow();
    if let PropertiesCoproductTest::Color(ref color) = *pc_borrowed {
        assert_eq!(color.fill, "red");
    } else {
        panic!("with_properties_unsafe failed to unpack the expected type")
    }
}

#[test]
fn test_unsafe_wrap() {
    let unwrapped = Color {fill: "orange".to_string()};
    let wrapped = unsafe_wrap!(unwrapped,PropertiesCoproductTest, Color);
    if let PropertiesCoproductTest::Color(ref color) = wrapped {
        println!("{:?}", color);
        assert_eq!(color.fill, "orange");
    } else {
        panic!("unsafe_wrap failed to pack the expected type")
    }
}