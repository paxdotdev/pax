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
    let parent = LocalProperty::new(Counted(vec![vec![1.0; 512]; 8]));
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
    let parent = LocalProperty::new(Counted(vec![vec![3.0]]));
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
    assert!(
        Variable::new_from_typed_property(LocalProperty::new(1.0_f64))
            .try_typed_binding::<i64>("numeric mismatch")
            .is_none()
    );
}

#[test]
fn mismatched_erased_adapter_cannot_create_a_typed_binding() {
    let source = LocalProperty::new(1.0_f64);
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
    let clock = LocalProperty::new(0_u64);
    register_millis(&clock);
    register_time(&LocalProperty::new(0_u64));
    let parent = LocalProperty::new(10.0_f64);
    let child = Variable::new_from_typed_property(parent.clone())
        .try_typed_binding::<f64>("child")
        .unwrap();
    let variable = Variable::new_from_typed_property(parent.clone());
    let legacy = LocalProperty::computed(
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
    child.replace_with(LocalProperty::new(20.0));
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
    let parent = LocalProperty::new(1.0_f64);
    let input = LocalProperty::new(2.0_f64);
    let child = Variable::new_from_typed_property(parent.clone())
        .try_typed_binding::<f64>("child")
        .unwrap();
    assert_eq!(child.get(), 1.0);
    let read = input.clone();
    parent.replace_with(LocalProperty::computed(
        move || read.get(),
        &[input.untyped()],
    ));
    assert_eq!(child.get(), 2.0);
    input.set(3.0);
    assert_eq!(child.get(), 3.0);
    let other_parent = LocalProperty::new(3.0_f64);
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
    let parent = LocalProperty::new(0.0_f64);
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
    assert!(
        Variable::new_from_typed_property(LocalProperty::new(Custom))
            .try_typed_binding::<Custom>("custom")
            .is_none()
    );
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
        let parent = LocalProperty::new(vec![1.0_f64; 512]);
        let variable = Variable::new_from_typed_property(parent);
        let child = variable.try_typed_binding::<Vec<f64>>("churn").unwrap();
        child.read(|v| assert_eq!(v.len(), 512));
    }
    assert_eq!(property_table_total_properties_count(), baseline);
}

#[test]
fn unused_dynamic_adapters_allocate_nothing_and_clones_share_first_read() {
    use crate::properties::property_table_total_properties_count as count;
    let before = count();
    {
        let source = LocalProperty::new(7.0_f64);
        let source_count = count();
        let variable = Variable::new_from_typed_property(source.clone());
        let copy = variable.clone();
        assert_eq!(count(), source_count);
        let typed = copy.try_typed_binding::<f64>("typed").unwrap();
        assert_eq!(count(), source_count + 1);
        assert_eq!(typed.get(), 7.0);
        source.set(8.0);
        assert_eq!(f64::try_coerce(variable.get_as_pax_value()).unwrap(), 8.0);
        let converted_count = count();
        assert_eq!(
            converted_count,
            source_count + 2,
            "only the typed binding and converter are needed for a scalar"
        );
        assert_eq!(f64::try_coerce(copy.get_as_pax_value()).unwrap(), 8.0);
        assert_eq!(count(), converted_count);
        source.set(9.0);
        assert_eq!(f64::try_coerce(copy.get_as_pax_value()).unwrap(), 9.0);
    }
    assert_eq!(count(), before);
}

#[test]
fn prepare_common_scope_evaluates_values_without_allocating_conversion_adapters() {
    use crate::properties::property_table_total_properties_count as count;
    let before = count();
    {
        let common = CommonProperties::default();
        let input = LocalProperty::new(40.0);
        let evaluations = Rc::new(Cell::new(0));
        let reads = evaluations.clone();
        let source = input.clone();
        common.width.replace_with(LocalProperty::computed(
            move || {
                reads.set(reads.get() + 1);
                Some(Size::Pixels(source.get().into()))
            },
            &[input.untyped()],
        ));
        let scope = common.retrieve_property_scope();
        let width = scope["width"].clone();
        let slots = count();
        for _ in 0..2 {
            for variable in scope.values() {
                variable.prepare();
            }
        }
        assert_eq!(evaluations.get(), 1, "prepare reuses a clean typed value");
        assert_eq!(
            count(),
            slots,
            "no common-property conversion graph is needed"
        );

        input.set(60.0);
        width.prepare();
        assert_eq!(evaluations.get(), 2);
        assert_eq!(count(), slots);
        input.set(80.0);
        assert_eq!(
            Option::<Size>::try_coerce(width.get_as_pax_value()).unwrap(),
            Some(Size::Pixels(80.0.into()))
        );
        assert_eq!(count(), slots + 1);
        assert_eq!(evaluations.get(), 3);
        assert_eq!(
            scope["width"].get_as_pax_value(),
            width.get_as_pax_value(),
            "clones share the first converted read"
        );
        assert_eq!(count(), slots + 1);

        let replacement = LocalProperty::new(100.0);
        let source = replacement.clone();
        common.width.replace_with(LocalProperty::computed(
            move || Some(Size::Pixels(source.get().into())),
            &[replacement.untyped()],
        ));
        let slots = count();
        width.prepare();
        assert_eq!(count(), slots, "replacing a binding preserves preparation");
        replacement.set(120.0);
        assert_eq!(
            Option::<Size>::try_coerce(width.get_as_pax_value()).unwrap(),
            Some(Size::Pixels(120.0.into()))
        );
    }
    assert_eq!(count(), before);
}

#[test]
fn prepare_preserves_custom_conversion_and_reuses_its_cache() {
    let source = LocalProperty::new(Counted(vec![vec![1.0]]));
    let variable = Variable::new_from_typed_property(source.clone());
    CONVERSIONS.with(|n| n.set(0));
    variable.prepare();
    variable.prepare();
    assert_eq!(CONVERSIONS.with(Cell::get), 1);
    source.set(Counted(vec![vec![2.0]]));
    variable.prepare();
    assert_eq!(CONVERSIONS.with(Cell::get), 2);
    assert_eq!(
        Vec::<Vec<f64>>::try_coerce(variable.get_as_pax_value()).unwrap(),
        vec![vec![2.0]]
    );
    assert_eq!(CONVERSIONS.with(Cell::get), 2);
}

#[test]
fn prepare_preserves_nested_observation_for_typed_consumers() {
    use crate::properties::{drain_effects, PropertyGraph};
    #[derive(Clone, Default)]
    struct Nested(Property<f64>);
    impl Interpolatable for Nested {}
    impl ToPaxValue for Nested {
        fn nested_properties(&self) -> Vec<UntypedProperty> {
            vec![self.0.local().untyped()]
        }
        fn to_pax_value(self) -> PaxValue {
            self.0.local().get().to_pax_value()
        }
    }
    let graph = PropertyGraph::new(|| {});
    let _entered = graph.enter();
    let scope = graph.binding_scope();
    let value = Property::new(1.0_f64);
    let (variable, reader) = {
        let _scope = scope.enter();
        let source = LocalProperty::new(Nested(value.clone()));
        let variable = Variable::new_from_typed_property(source.clone());
        let deps = [source.untyped()];
        let reader = LocalProperty::computed(move || source.get().0.local().get(), &deps);
        (variable, reader)
    };
    variable.prepare();
    assert_eq!(reader.get(), 1.0);
    value.set(2.0);
    graph.import();
    drain_effects(100);
    assert_eq!(
        reader.get(),
        2.0,
        "prepare retains nested change propagation"
    );
    scope.close();
    graph.import();
    graph.collect();
    value.set(3.0);
    assert!(!graph.has_pending());
    graph.close();
}

#[test]
fn first_dynamic_read_keeps_original_graph_and_restores_reader_context() {
    use crate::properties::PropertyGraph;
    let owner = PropertyGraph::new(|| {});
    let reader = PropertyGraph::new(|| {});
    let source = Property::new(1.0_f64);
    let variable = {
        let _entered = owner.enter();
        Variable::new_from_typed_property(source.clone())
    };
    {
        let _entered = reader.enter();
        assert_eq!(f64::try_coerce(variable.get_as_pax_value()).unwrap(), 1.0);
        // The initializer must restore the active reader after constructing its cache.
        let independent = Property::new(3.0_f64);
        let reader_local = independent.local();
        independent.set(4.0);
        reader.import();
        assert_eq!(reader_local.get(), 4.0);
    }
    source.set(2.0);
    {
        let _entered = reader.enter();
        reader.import();
    }
    assert_eq!(f64::try_coerce(variable.get_as_pax_value()).unwrap(), 1.0);
    {
        let _entered = owner.enter();
        owner.import();
    }
    assert_eq!(f64::try_coerce(variable.get_as_pax_value()).unwrap(), 2.0);
    owner.close();
    reader.close();
}

#[test]
fn lazy_nested_conversion_uses_original_scope_and_keeps_nested_reactivity() {
    use crate::properties::{drain_effects, PropertyGraph};
    #[derive(Clone, Default)]
    struct Nested(Property<f64>);
    impl Interpolatable for Nested {}
    impl ToPaxValue for Nested {
        fn nested_properties(&self) -> Vec<UntypedProperty> {
            vec![self.0.local().untyped()]
        }
        fn to_pax_value(self) -> PaxValue {
            self.0.local().get().to_pax_value()
        }
    }
    let graph = PropertyGraph::new(|| {});
    let _entered = graph.enter();
    let scope = graph.binding_scope();
    let value = Property::new(1.0_f64);
    let variable = {
        let _scope = scope.enter();
        Variable::new_from_typed_property(LocalProperty::new(Nested(value.clone())))
    };
    assert_eq!(f64::try_coerce(variable.get_as_pax_value()).unwrap(), 1.0);
    value.set(2.0);
    graph.import();
    drain_effects(100);
    assert_eq!(f64::try_coerce(variable.get_as_pax_value()).unwrap(), 2.0);
    scope.close();
    graph.import();
    graph.collect();
    value.set(3.0);
    assert!(
        !graph.has_pending(),
        "first conversion must belong to its creation scope"
    );
    scope.resume();
    drain_effects(100);
    assert_eq!(f64::try_coerce(variable.get_as_pax_value()).unwrap(), 3.0);
    graph.close();
}

#[test]
fn nested_edits_update_the_snapshot_without_reconstructing_the_container() {
    use crate::properties::{drain_effects, PropertyGraph};
    #[derive(Clone, Default)]
    struct Nested(Property<f64>);
    impl Interpolatable for Nested {}
    impl ToPaxValue for Nested {
        fn nested_properties(&self) -> Vec<UntypedProperty> {
            vec![self.0.local().untyped()]
        }
        fn to_pax_value(self) -> PaxValue {
            self.0.local().get().to_pax_value()
        }
    }
    let graph = PropertyGraph::new(|| {});
    let _entered = graph.enter();
    let constructions = Rc::new(Cell::new(0));
    let counted = constructions.clone();
    let source = LocalProperty::computed(move || {
        counted.set(counted.get() + 1);
        Nested(Property::new(1.0))
    }, &[]);
    let variable = Variable::new_from_typed_property(source.clone());
    assert_eq!(f64::try_coerce(variable.get_as_pax_value()).unwrap(), 1.0);
    source.get().0.set(2.0);
    graph.import();
    drain_effects(100);
    assert_eq!(f64::try_coerce(variable.get_as_pax_value()).unwrap(), 2.0);
    assert_eq!(constructions.get(), 1, "nested edits must preserve container identity");
    graph.close();
}
