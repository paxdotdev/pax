use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use pax_core::with_properties_unwrapped;


#[derive(Default, Debug, PartialEq)]
struct Color {
    fill: String,
}

#[test]
fn test_with_properties_unwrapped() {
    let fully_wrapped : Rc<RefCell<dyn Any>> = Rc::new(RefCell::new(
        Color {fill: "blue".to_string()}
    ));
    with_properties_unwrapped!(&fully_wrapped, Color, |color : &mut Color| {
        color.fill = "red".to_string();
    });

    let pc_borrowed = (*fully_wrapped).borrow();

    if let Some(color) = pc_borrowed.downcast_ref::<Color>() {
        assert_eq!(color.fill, "red");
    } else {
        panic!("with_properties_unsafe failed to unpack the expected type")
    }
}
