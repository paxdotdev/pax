use super::*;

// #[test]
// fn test_literal_set_get() {
//     let prop = Property::literal(5);
//     assert_eq!(prop.get(), 5);
//     prop.set(2);
//     assert_eq!(prop.get(), 2);
// }

// #[test]
// fn test_computed_get() {
//     let prop = Property::<i32>::computed(|| 42, &vec![]);
//     assert_eq!(prop.get(), 42);
// }

#[test]
fn test_computed_dependent_on_literal() {
    let prop_1 = Property::new_with_name(2, "p1");
    let p1 = prop_1.clone();
    let prop_2 = Property::<i32>::computed_with_name(
        move || p1.get() * 5,
        &vec![&prop_1.as_untyped()],
        "p2",
    );
    println!("this is fine UNTIL NOW");
    drop(prop_2);
    println!("NOT FINE");

    // assert_eq!(prop_2.get(), 10);
    // prop_1.set(3);
    // assert_eq!(prop_2.get(), 15);
}

// #[test]
// fn test_property_replacement() {
//     let prop_1 = Property::literal(2);
//     let p1 = prop_1.clone();
//     let prop_2 = Property::computed(move || p1.get(), &vec![&prop_1.as_untyped()]);

//     let prop_3 = Property::literal(6);
//     let p3 = prop_3.clone();
//     let prop_4 = Property::computed(move || p3.get(), &vec![&prop_3.as_untyped()]);

//     assert_eq!(prop_2.get(), 2);
//     assert_eq!(prop_4.get(), 6);
//     prop_3.replace_with(prop_2);
//     assert_eq!(prop_4.get(), 2);
// }

// #[test]
// fn test_larger_network() {
//     let prop_1 = Property::literal(2);
//     let prop_2 = Property::literal(6);

//     let p1 = prop_1.clone();
//     let p2 = prop_2.clone();
//     let prop_3 = Property::computed(
//         move || p1.get() * p2.get(),
//         &vec![&prop_1.as_untyped(), &prop_2.as_untyped()],
//     );
//     let p1 = prop_1.clone();
//     let p3 = prop_3.clone();
//     let prop_4 = Property::computed(
//         move || p1.get() + p3.get(),
//         &vec![&prop_1.as_untyped(), &prop_3.as_untyped()],
//     );

//     assert_eq!(prop_4.get(), 14);
//     prop_1.set(1);
//     assert_eq!(prop_4.get(), 7);
//     prop_2.set(2);
//     assert_eq!(prop_4.get(), 3);
// }

// #[test]
// fn test_cleanup() {
//     assert!(PROPERTY_TABLE.with(|t| t.entries.borrow().is_empty()));
//     let prop = Property::literal(5);
//     assert_eq!(PROPERTY_TABLE.with(|t| t.entries.borrow().len()), 1);
//     drop(prop);
//     assert!(PROPERTY_TABLE.with(|t| t.entries.borrow().is_empty()));
// }

// #[test]
// fn test_recursive_props() {
//     {
//         let prop_of_prop = Property::literal(Property::literal(3));
//         let prop_of_prop_clone = prop_of_prop.clone();
//         prop_of_prop_clone.get().set(1);
//         assert_eq!(prop_of_prop.get().get(), prop_of_prop_clone.get().get());
//     }
//     assert!(PROPERTY_TABLE.with(|t| t.entries.borrow().is_empty()));
// }
