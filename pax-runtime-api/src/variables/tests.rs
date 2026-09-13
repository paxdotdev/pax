use super::*;
use crate::pax_value::is_typed_binding_safe;

thread_local! {
    static COPIES: Cell<usize> = const { Cell::new(0) };
    static CONVERSIONS: Cell<usize> = const { Cell::new(0) };
}

#[derive(Default)]
struct Counted(Vec<Vec<f64>>);
impl Clone for Counted {
    fn clone(&self) -> Self {
        COPIES.with(|n| n.set(n.get() + 1));
        Self(self.0.clone())
    }
}
impl Interpolatable for Counted {}
impl ToPaxValue for Counted {
    fn to_pax_value(self) -> PaxValue {
        CONVERSIONS.with(|n| n.set(n.get() + 1));
        self.0.to_pax_value()
    }
}
impl CoercionRules for Counted {
    fn is_identity_roundtrip() -> bool {
        true
    }
    fn try_coerce(value: PaxValue) -> Result<Self, String> {
        Vec::<Vec<f64>>::try_coerce(value).map(Self)
    }
}

#[test]
fn typed_forwarding_clones_once_per_dirty_hop_without_dynamic_conversion() {
    let parent = Property::new(Counted(vec![vec![1.0; 512]; 8]));
    let first = Variable::new_from_typed_property(parent.clone())
        .try_typed_binding::<Counted>("first")
        .unwrap();
    let second = Variable::new_from_typed_property(first.clone())
        .try_typed_binding::<Counted>("second")
        .unwrap();
    COPIES.with(|n| n.set(0));
    CONVERSIONS.with(|n| n.set(0));
    for _ in 0..5 {
        second.read(|v| assert_eq!(v.0[0][0], 1.0));
    }
    assert_eq!(COPIES.with(Cell::get), 2);
    assert_eq!(CONVERSIONS.with(Cell::get), 0);
    parent.set(Counted(vec![vec![2.0; 512]; 8]));
    second.read(|v| assert_eq!(v.0[0][0], 2.0));
    assert_eq!(COPIES.with(Cell::get), 4);
    assert_eq!(CONVERSIONS.with(Cell::get), 0);
}

#[test]
fn exact_type_check_and_existing_dynamic_cache_are_preserved() {
    let parent = Property::new(Counted(vec![vec![3.0]]));
    let variable = Variable::new_from_typed_property(parent.clone());
    assert!(variable
        .try_typed_binding::<Vec<Vec<f64>>>("wrong")
        .is_none());
    CONVERSIONS.with(|n| n.set(0));
    for _ in 0..3 {
        assert_eq!(
            Vec::<Vec<f64>>::try_coerce(variable.get_as_pax_value()).unwrap(),
            vec![vec![3.0]]
        );
    }
    assert_eq!(CONVERSIONS.with(Cell::get), 1);
    parent.set(Counted(vec![vec![4.0]]));
    variable.get_as_pax_value();
    assert_eq!(CONVERSIONS.with(Cell::get), 2);
    assert!(Variable::new_from_typed_property(Property::new(1.0_f64))
        .try_typed_binding::<i64>("numeric mismatch")
        .is_none());
}

#[test]
fn mismatched_erased_adapter_cannot_create_a_typed_binding() {
    let source = Property::new(1.0_f64);
    let invalid_adapter = Variable::new::<Vec<f64>>(source.untyped());
    assert!(invalid_adapter
        .try_typed_binding::<Vec<f64>>("wrong storage")
        .is_none());
    assert!(invalid_adapter
        .try_typed_binding::<f64>("wrong adapter")
        .is_none());
}

#[test]
fn child_writes_and_easing_are_independent_from_parent() {
    use crate::properties::{register_millis, register_time};
    let clock = Property::new(0_u64);
    register_millis(&clock);
    register_time(&Property::new(0_u64));
    let parent = Property::new(10.0_f64);
    let child = Variable::new_from_typed_property(parent.clone())
        .try_typed_binding::<f64>("child")
        .unwrap();
    let variable = Variable::new_from_typed_property(parent.clone());
    let legacy = Property::computed(
        move || f64::try_coerce(variable.get_as_pax_value()).unwrap(),
        &[parent.untyped()],
    );
    assert_ne!(child.untyped().get_id(), parent.untyped().get_id());
    assert_eq!(child.get(), 10.0);
    assert_eq!(legacy.get(), 10.0);
    child.set(20.0);
    legacy.set(20.0);
    assert_eq!(parent.get(), 10.0);
    child.ease_to(
        40.0,
        Duration::Milliseconds(1000.into()),
        EasingCurve::Linear,
    );
    legacy.ease_to(
        40.0,
        Duration::Milliseconds(1000.into()),
        EasingCurve::Linear,
    );
    clock.set(500);
    // Easing a computed property currently leaves its evaluator authoritative.
    // This optimization must match that behavior, not change transition policy.
    assert_eq!(child.get(), legacy.get());
    assert_eq!(parent.get(), 10.0);
    child.cancel_transitions();
    parent.set(50.0);
    assert_eq!(child.get(), 50.0);
    child.replace_with(Property::new(20.0));
    child.ease_to(
        40.0,
        Duration::Milliseconds(1000.into()),
        EasingCurve::Linear,
    );
    clock.set(1000);
    assert_eq!(child.get(), 30.0);
    assert_eq!(parent.get(), 50.0);
}

#[test]
fn replacement_and_rebinding_follow_property_identity() {
    let parent = Property::new(1.0_f64);
    let input = Property::new(2.0_f64);
    let child = Variable::new_from_typed_property(parent.clone())
        .try_typed_binding::<f64>("child")
        .unwrap();
    assert_eq!(child.get(), 1.0);
    let read = input.clone();
    parent.replace_with(Property::computed(move || read.get(), &[input.untyped()]));
    assert_eq!(child.get(), 2.0);
    input.set(3.0);
    assert_eq!(child.get(), 3.0);
    let other_parent = Property::new(3.0_f64);
    child.replace_with(
        Variable::new_from_typed_property(other_parent.clone())
            .try_typed_binding::<f64>("new parent")
            .unwrap(),
    );
    input.set(4.0);
    assert_eq!(child.get(), 3.0);
    other_parent.set(5.0);
    assert_eq!(child.get(), 5.0);
}

#[test]
fn forwarded_float_bits_are_exact() {
    let parent = Property::new(0.0_f64);
    let child = Variable::new_from_typed_property(parent.clone())
        .try_typed_binding::<f64>("bits")
        .unwrap();
    for bits in [
        0,
        (-0.0_f64).to_bits(),
        1,
        1.0_f64.to_bits() + 1,
        0x7ff8_0000_0000_1234,
    ] {
        parent.set(f64::from_bits(bits));
        assert_eq!(child.get().to_bits(), bits);
    }
}

#[test]
fn custom_and_nested_custom_conversions_require_opt_in() {
    #[derive(Default, Clone)]
    struct Custom;
    impl Interpolatable for Custom {}
    impl ToPaxValue for Custom {
        fn to_pax_value(self) -> PaxValue {
            true.to_pax_value()
        }
    }
    impl CoercionRules for Custom {
        fn try_coerce(_: PaxValue) -> Result<Self, String> {
            Ok(Self)
        }
    }
    assert!(!is_typed_binding_safe::<Custom>());
    assert!(!is_typed_binding_safe::<Vec<Option<Custom>>>());
    assert!(Variable::new_from_typed_property(Property::new(Custom))
        .try_typed_binding::<Custom>("custom")
        .is_none());
}

#[test]
fn recursive_type_checks_fall_back_and_leave_no_guard_state() {
    struct Recursive;
    impl CoercionRules for Recursive {
        fn try_coerce(_: PaxValue) -> Result<Self, String> {
            Ok(Self)
        }
        fn is_identity_roundtrip() -> bool {
            is_typed_binding_safe::<Vec<Self>>()
        }
    }
    for _ in 0..3 {
        assert!(!is_typed_binding_safe::<Recursive>());
        assert!(is_typed_binding_safe::<Vec<Option<f64>>>());
    }
}

#[test]
fn typed_binding_churn_releases_all_properties() {
    use crate::properties::property_table_total_properties_count;
    let baseline = property_table_total_properties_count();
    for _ in 0..100 {
        let parent = Property::new(vec![1.0_f64; 512]);
        let variable = Variable::new_from_typed_property(parent);
        let child = variable.try_typed_binding::<Vec<f64>>("churn").unwrap();
        child.read(|v| assert_eq!(v.len(), 512));
    }
    assert_eq!(property_table_total_properties_count(), baseline);
}
