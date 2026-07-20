use super::*;
use std::{cell::Cell, rc::Rc};

#[test]
fn test_literal_set_get() {
    let prop = Property::new(5);
    assert_eq!(prop.get(), 5);
    prop.set(2);
    assert_eq!(prop.get(), 2);
}

#[test]
fn test_computed_get() {
    let prop = Property::<i32>::computed(|| 42, &[]);
    assert_eq!(prop.get(), 42);
}

#[test]
fn test_computed_dependent_on_literal() {
    let prop_1 = Property::new_with_name(2, "p1");
    let p1 = prop_1.clone();
    let prop_2 =
        Property::<i32>::computed_with_name(move || p1.get() * 5, &[prop_1.untyped()], "p2");
    assert_eq!(prop_2.get(), 10);
    prop_1.set(3);
    assert_eq!(prop_2.get(), 15);
}

#[test]
fn test_property_replacement() {
    let prop_1 = Property::new(2);
    let p1 = prop_1.clone();
    let prop_2 = Property::computed(move || p1.get(), &[prop_1.untyped()]);

    let prop_3 = Property::new(6);
    let p3 = prop_3.clone();
    let prop_4 = Property::computed(move || p3.get(), &[prop_3.untyped()]);

    assert_eq!(prop_2.get(), 2);
    assert_eq!(prop_4.get(), 6);
    prop_3.replace_with(prop_2);
    assert_eq!(prop_4.get(), 2);
}

#[test]
fn test_property_replacement_with_self_is_noop() {
    let prop = Property::new(7);

    prop.replace_with(prop.clone());

    assert_eq!(prop.get(), 7);
}

#[test]
fn test_larger_network() {
    let prop_1 = Property::new(2);
    let prop_2 = Property::new(6);

    let p1 = prop_1.clone();
    let p2 = prop_2.clone();
    let prop_3 = Property::computed(
        move || p1.get() * p2.get(),
        &[prop_1.untyped(), prop_2.untyped()],
    );
    let p1 = prop_1.clone();
    let p3 = prop_3.clone();
    let prop_4 = Property::computed(
        move || p1.get() + p3.get(),
        &[prop_1.untyped(), prop_3.untyped()],
    );

    assert_eq!(prop_4.get(), 14);
    prop_1.set(1);
    assert_eq!(prop_4.get(), 7);
    prop_2.set(2);
    assert_eq!(prop_4.get(), 3);
}

#[test]
fn test_cleanup() {
    assert!(PROPERTY_TABLE.with(|t| t.property_map.borrow().is_empty()));
    let prop = Property::new(5);
    assert_eq!(PROPERTY_TABLE.with(|t| t.property_map.borrow().len()), 1);
    drop(prop);
    assert!(PROPERTY_TABLE.with(|t| t.property_map.borrow().is_empty()));
}

#[test]
fn test_recursive_props() {
    {
        let prop_of_prop = Property::new(Property::new(3));
        let prop_of_prop_clone = prop_of_prop.clone();
        prop_of_prop_clone.get().set(1);
        assert_eq!(prop_of_prop.get().get(), prop_of_prop_clone.get().get());
    }
    assert!(PROPERTY_TABLE.with(|t| t.property_map.borrow().is_empty()));
}

#[test]
fn test_registered_effect_drains_when_dirty() {
    let source = Property::new(1);
    let eval_count = Rc::new(Cell::new(0));
    let seen_value = Rc::new(Cell::new(0));

    let source_for_effect = source.clone();
    let eval_count_for_effect = Rc::clone(&eval_count);
    let seen_value_for_effect = Rc::clone(&seen_value);
    let effect = Property::computed(
        move || {
            eval_count_for_effect.set(eval_count_for_effect.get() + 1);
            seen_value_for_effect.set(source_for_effect.get());
        },
        &[source.untyped()],
    );
    register_effect_property(&effect);

    assert_eq!(eval_count.get(), 0);
    assert_eq!(drain_effects(10), 1);
    assert_eq!(eval_count.get(), 1);
    assert_eq!(seen_value.get(), 1);

    source.set(2);
    source.set(3);
    assert_eq!(eval_count.get(), 1);
    assert_eq!(drain_effects(10), 1);
    assert_eq!(eval_count.get(), 2);
    assert_eq!(seen_value.get(), 3);
    assert_eq!(drain_effects(10), 0);
}

#[test]
fn test_set_if_neq_skips_noop_write() {
    let source = Property::new(1);
    let eval_count = Rc::new(Cell::new(0));

    let source_for_effect = source.clone();
    let eval_count_for_effect = Rc::clone(&eval_count);
    let effect = Property::computed(
        move || {
            eval_count_for_effect.set(eval_count_for_effect.get() + 1);
            let _ = source_for_effect.get();
        },
        &[source.untyped()],
    );
    register_effect_property(&effect);

    assert_eq!(drain_effects(10), 1);
    assert_eq!(eval_count.get(), 1);

    assert!(!source.set_if_neq(1));
    assert_eq!(drain_effects(10), 0);
    assert_eq!(eval_count.get(), 1);

    assert!(source.set_if_neq(2));
    assert_eq!(drain_effects(10), 1);
    assert_eq!(eval_count.get(), 2);
}

#[test]
fn test_invalidate_preserves_pending_computed_update() {
    let source = Property::new(false);
    let source_for_computed = source.clone();
    let computed = Property::computed(move || source_for_computed.get(), &[source.untyped()]);

    assert!(!computed.get());
    source.set(true);
    computed.invalidate();

    assert!(computed.get());
}
