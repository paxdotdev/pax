use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct Color {
    fill: String,
}

#[test]
fn test_dyn_any_properties() {
    let any_properties = Rc::new(RefCell::new(Color {
        fill: "red".to_string(),
    })) as Rc<RefCell<dyn Any>>;

    let properties_borrowed = any_properties.borrow();
    let unwrapped: &Color = properties_borrowed
        .downcast_ref()
        .expect("Failed to downcast");
    assert_eq!(unwrapped.fill, "red");
}

#[test]
fn downcast_repeat_properties_optional() {
    let wrapped: Box<dyn Any> = Box::new((0 as isize)..(10 as isize));

    if let Ok(downcast_value) = wrapped.downcast::<std::ops::Range<isize>>() {
        assert_eq!(downcast_value.start, 0);
        assert_eq!(downcast_value.end, 10);
    } else {
        panic!();
    }
}
