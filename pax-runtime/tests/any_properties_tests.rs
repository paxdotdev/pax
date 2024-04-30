use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use pax_runtime_api::pax_value::{PaxValue, ToFromPaxValue};

#[derive(Default)]
struct Color {
    fill: String,
}

#[test]
fn test_dyn_any_properties() {
    let any_properties = Rc::new(RefCell::new(
        Color {
            fill: "red".to_string(),
        }
        .to_pax_value(),
    ));

    let properties_borrowed = any_properties.borrow();
    let unwrapped: &Color =
        Color::ref_from_pax_value(&properties_borrowed).expect("Failed to downcast");
    assert_eq!(unwrapped.fill, "red");
}

#[test]
fn downcast_repeat_properties_optional() {
    let wrapped: PaxValue = ((0 as isize)..(10 as isize)).to_pax_value();

    if let Ok(downcast_value) = <std::ops::Range<isize>>::ref_from_pax_value(&wrapped) {
        assert_eq!(downcast_value.start, 0);
        assert_eq!(downcast_value.end, 10);
    } else {
        panic!();
    }
}
