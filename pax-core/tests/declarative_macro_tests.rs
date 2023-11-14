use std::cell::RefCell;
use std::rc::Rc;
use pax_core::with_properties_unwrapped;

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
fn test_with_properties_unwrapped() {
    let fully_wrapped : Rc<RefCell<PropertiesCoproductTest>> = Rc::new(RefCell::new(PropertiesCoproductTest::Color(Color {fill: "blue".to_string()})));
    with_properties_unwrapped!(&fully_wrapped, PropertiesCoproductTest, Color, |color : &mut Color| {
        color.fill = "red".to_string();
    });

    let pc_borrowed = (*fully_wrapped).borrow();
    if let PropertiesCoproductTest::Color(ref color) = *pc_borrowed {
        assert_eq!(color.fill, "red");
    } else {
        panic!("with_properties_unsafe failed to unpack the expected type")
    }
}
