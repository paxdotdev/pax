use std::cell::RefCell;
use std::rc::Rc;

use pax_runtime_api::pax_value::{ImplToFromPaxAny, PaxAny, ToFromPaxAny};

#[derive(Default)]
struct Color {
    fill: String,
}

impl ImplToFromPaxAny for Color {}

#[test]
fn test_dyn_any_properties() {
    let any_properties = Rc::new(RefCell::new(
        Color {
            fill: "red".to_string(),
        }
        .to_pax_any(),
    ));

    let properties_borrowed = any_properties.borrow();
    let unwrapped: &Color =
        Color::ref_from_pax_any(&properties_borrowed).expect("Failed to downcast");
    assert_eq!(unwrapped.fill, "red");
}

#[test]
fn downcast_repeat_properties_optional() {
    let wrapped: PaxAny = ((0 as isize)..(10 as isize)).to_pax_any();

    if let Ok(downcast_value) = <std::ops::Range<isize>>::ref_from_pax_any(&wrapped) {
        assert_eq!(downcast_value.start, 0);
        assert_eq!(downcast_value.end, 10);
    } else {
        panic!();
    }
}
