
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct Color {
    fill: String,
}

#[test]
fn test_dyn_any_properties() {
    let any_properties = Rc::new(RefCell::new(Color{fill: "red".to_string()})) as Rc<RefCell<dyn Any>>;

    let properties_borrowed = any_properties.borrow();
    let unwrapped : &Color = properties_borrowed.downcast_ref().expect("Failed to downcast");
    assert_eq!(unwrapped.fill, "red");
}