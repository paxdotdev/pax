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
fn test_cancel_transitions_freezes_value_and_clears_queue() {
    let frames = Property::new(0_u64);
    let millis = Property::new(0_u64);
    register_time(&frames);
    register_millis(&millis);

    let prop = Property::new(0.0);
    prop.ease_to(
        10.0,
        Duration::Milliseconds(100.into()),
        EasingCurve::Linear,
    );
    prop.ease_to_later(
        20.0,
        Duration::Milliseconds(100.into()),
        EasingCurve::Linear,
    );

    millis.set(40);
    assert_eq!(prop.get(), 4.0);
    prop.cancel_transitions();

    millis.set(180);
    assert_eq!(prop.get(), 4.0);

    prop.set(7.5);
    millis.set(220);
    assert_eq!(prop.get(), 7.5);
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
fn test_property_replacement_disconnects_previous_dependencies() {
    let old_source = Property::new(1);
    let old_source_for_computed = old_source.clone();
    let slot = Property::computed(
        move || old_source_for_computed.get(),
        &[old_source.untyped()],
    );
    let effect_runs = Rc::new(Cell::new(0));
    let effect_runs_for_effect = Rc::clone(&effect_runs);
    let slot_for_effect = slot.clone();
    let effect = Property::computed(
        move || {
            let _ = slot_for_effect.get();
            effect_runs_for_effect.set(effect_runs_for_effect.get() + 1);
        },
        &[slot.untyped()],
    );
    register_effect_property(&effect);
    drain_effects(10);

    slot.replace_with(Property::new(0));
    drain_effects(10);
    let runs_after_replacement = effect_runs.get();

    old_source.set(2);
    assert_eq!(drain_effects(10), 0);
    assert_eq!(effect_runs.get(), runs_after_replacement);
    assert_eq!(slot.get(), 0);
}

#[test]
fn test_replacement_reuses_dependency_storage_without_reusing_binding_state() {
    let initial_count = property_table_total_properties_count();
    let a = Property::new(1);
    let b = Property::new(2);
    let c = Property::new(3);
    let read_a = a.clone();
    let slot = Property::computed(
        move || read_a.get(),
        &[a.untyped(), b.untyped(), c.untyped()],
    );
    let read_slot = slot.clone();
    let dependent = Property::computed(move || read_slot.get() * 10, &[slot.untyped()]);
    assert_eq!(dependent.get(), 10);
    let storage = PROPERTY_TABLE
        .with(|table| table.with_property_data(slot.untyped.id, |data| data.inbound.as_ptr()));

    // Reuse capacity across shrinking, empty, duplicate, and reordered inputs.
    // The evaluator, value, graph edges, and dirty propagation must still change.
    for dependencies in [
        vec![c.clone(), b.clone()],
        vec![],
        vec![b.clone(), b.clone(), a.clone()],
    ] {
        let inputs = dependencies
            .iter()
            .map(Property::untyped)
            .collect::<Vec<_>>();
        let reads = dependencies.clone();
        let replacement = Property::computed(
            move || reads.iter().map(Property::get).sum::<i32>(),
            &inputs,
        );
        let expected = replacement.get(); // Also cover replacing from a clean property.
        slot.replace_with(replacement);
        assert_eq!(dependent.get(), expected * 10);
        PROPERTY_TABLE.with(|table| {
            let (pointer, inbound) = table.with_property_data(slot.untyped.id, |data| {
                (data.inbound.as_ptr(), data.inbound.clone())
            });
            assert_eq!(pointer, storage);
            assert_eq!(
                inbound,
                inputs.iter().map(|input| input.id).collect::<Vec<_>>()
            );
            for input in [&a, &b, &c] {
                let connections = table.with_property_data(input.untyped.id, |data| {
                    data.outbound
                        .iter()
                        .filter(|id| **id == slot.untyped.id)
                        .count()
                });
                assert_eq!(
                    connections,
                    inputs.iter().filter(|id| id.id == input.untyped.id).count()
                );
            }
        });
    }
    a.set(7);
    assert_eq!(dependent.get(), 110);
    b.set(4);
    assert_eq!(dependent.get(), 150);
    c.set(100);
    assert_eq!(dependent.get(), 150);
    drop(dependent);
    drop(slot);
    drop([a, b, c]);
    assert_eq!(property_table_total_properties_count(), initial_count);
}

#[test]
fn test_replacement_preserves_dependency_connection_order_and_shared_target() {
    let source = Property::new(1);
    let read_source = source.clone();
    let first = Property::computed(move || read_source.get(), &[source.untyped()]);
    let read_source = source.clone();
    let sibling = Property::computed(move || read_source.get(), &[source.untyped()]);
    let read_source = source.clone();
    let replacement = Property::computed(move || read_source.get() + 10, &[source.untyped()]);

    first.replace_with(replacement.clone());
    PROPERTY_TABLE.with(|table| {
        let outbound = table.with_property_data(source.untyped.id, |data| data.outbound.clone());
        // Even identical dependencies reconnect at the end: dirtification
        // traverses this order when scheduling dependent effects.
        assert_eq!(
            outbound,
            vec![sibling.untyped.id, replacement.untyped.id, first.untyped.id]
        );
    });
    source.set(3);
    assert_eq!(first.get(), 13);
    assert_eq!(replacement.get(), 13);
    drop(replacement);
    source.set(4);
    assert_eq!(first.get(), 14);
    assert_eq!(sibling.get(), 4);
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

#[test]
fn test_cutoff_suppresses_equivalent_output_propagation() {
    let source = Property::new(1);
    let source_for_cutoff = source.clone();
    let cutoff = Property::computed_with_cutoff(
        move || source_for_cutoff.get() % 2,
        &[source.untyped()],
        i32::eq,
    );
    let effect_runs = Rc::new(Cell::new(0));
    let seen = Rc::new(Cell::new(0));
    let cutoff_for_effect = cutoff.clone();
    let effect_runs_for_effect = Rc::clone(&effect_runs);
    let seen_for_effect = Rc::clone(&seen);
    let effect = Property::computed(
        move || {
            effect_runs_for_effect.set(effect_runs_for_effect.get() + 1);
            seen_for_effect.set(cutoff_for_effect.get());
        },
        &[cutoff.untyped()],
    );
    register_effect_property(&effect);

    drain_effects(10);
    assert_eq!(effect_runs.get(), 1);
    assert_eq!(seen.get(), 1);

    source.set(3);
    drain_effects(10);
    assert_eq!(effect_runs.get(), 1);
    assert_eq!(cutoff.get(), 1);

    source.set(4);
    drain_effects(10);
    assert_eq!(effect_runs.get(), 2);
    assert_eq!(seen.get(), 0);
}

#[test]
fn test_cutoff_retains_last_accepted_value() {
    let source = Property::new(0.0);
    let source_for_cutoff = source.clone();
    let cutoff = Property::computed_with_cutoff(
        move || source_for_cutoff.get(),
        &[source.untyped()],
        |previous: &f64, candidate: &f64| (previous - candidate).abs() < 1.0,
    );

    assert_eq!(cutoff.get(), 0.0);

    source.set(0.6);
    assert_eq!(cutoff.get(), 0.0);

    source.set(1.2);
    assert_eq!(cutoff.get(), 1.2);
}

#[test]
fn test_chained_cutoffs_settle_before_effects() {
    let source = Property::new(1);
    let source_for_first = source.clone();
    let first = Property::computed_with_cutoff(
        move || source_for_first.get() % 2,
        &[source.untyped()],
        i32::eq,
    );
    let first_for_second = first.clone();
    let second = Property::computed_with_cutoff(
        move || first_for_second.get() * 10,
        &[first.untyped()],
        i32::eq,
    );
    let effect_runs = Rc::new(Cell::new(0));
    let seen = Rc::new(Cell::new(0));
    let second_for_effect = second.clone();
    let effect_runs_for_effect = Rc::clone(&effect_runs);
    let seen_for_effect = Rc::clone(&seen);
    let effect = Property::computed(
        move || {
            effect_runs_for_effect.set(effect_runs_for_effect.get() + 1);
            seen_for_effect.set(second_for_effect.get());
        },
        &[second.untyped()],
    );
    register_effect_property(&effect);

    drain_effects(10);
    assert_eq!(effect_runs.get(), 1);
    assert_eq!(seen.get(), 10);

    source.set(3);
    drain_effects(10);
    assert_eq!(effect_runs.get(), 1);

    source.set(4);
    drain_effects(10);
    assert_eq!(effect_runs.get(), 2);
    assert_eq!(seen.get(), 0);
}

#[test]
fn test_diamond_effect_observes_settled_cutoff_once() {
    let source = Property::new(1);
    let source_for_cutoff = source.clone();
    let cutoff_branch = Property::computed_with_cutoff(
        move || source_for_cutoff.get() * 2,
        &[source.untyped()],
        i32::eq,
    );
    let source_for_eager = source.clone();
    let eager_branch = Property::computed(move || source_for_eager.get() * 3, &[source.untyped()]);
    let effect_runs = Rc::new(Cell::new(0));
    let seen = Rc::new(Cell::new((0, 0)));
    let cutoff_for_effect = cutoff_branch.clone();
    let eager_for_effect = eager_branch.clone();
    let effect_runs_for_effect = Rc::clone(&effect_runs);
    let seen_for_effect = Rc::clone(&seen);
    let effect = Property::computed(
        move || {
            effect_runs_for_effect.set(effect_runs_for_effect.get() + 1);
            seen_for_effect.set((cutoff_for_effect.get(), eager_for_effect.get()));
        },
        &[cutoff_branch.untyped(), eager_branch.untyped()],
    );
    register_effect_property(&effect);

    drain_effects(10);
    assert_eq!(effect_runs.get(), 1);
    assert_eq!(seen.get(), (2, 3));

    source.set(2);
    drain_effects(10);
    assert_eq!(effect_runs.get(), 2);
    assert_eq!(seen.get(), (4, 6));
}

#[test]
fn test_get_settles_queued_cutoff_before_drain() {
    let source = Property::new(1);
    let source_for_cutoff = source.clone();
    let cutoff = Property::computed_with_cutoff(
        move || source_for_cutoff.get(),
        &[source.untyped()],
        i32::eq,
    );

    assert_eq!(cutoff.get(), 1);
    source.set(2);
    assert_eq!(cutoff.get(), 2);
    assert_eq!(drain_effects(10), 0);
}

#[test]
fn test_dropped_queued_cutoff_is_ignored() {
    let source = Property::new(1);
    let source_for_cutoff = source.clone();
    let cutoff = Property::computed_with_cutoff(
        move || source_for_cutoff.get(),
        &[source.untyped()],
        i32::eq,
    );

    assert_eq!(cutoff.get(), 1);
    source.set(2);
    drop(cutoff);

    assert_eq!(drain_effects(10), 0);
}

#[test]
fn test_cutoff_initializes_without_being_pulled_by_effect() {
    let source = Property::new(1);
    let source_for_cutoff = source.clone();
    let cutoff = Property::computed_with_cutoff(
        move || source_for_cutoff.get(),
        &[source.untyped()],
        i32::eq,
    );
    let effect_runs = Rc::new(Cell::new(0));
    let effect_runs_for_effect = Rc::clone(&effect_runs);
    let effect = Property::computed(
        move || effect_runs_for_effect.set(effect_runs_for_effect.get() + 1),
        &[cutoff.untyped()],
    );
    register_effect_property(&effect);

    drain_effects(10);
    assert_eq!(effect_runs.get(), 1);

    source.set(2);
    drain_effects(10);
    assert_eq!(effect_runs.get(), 2);
}

#[test]
fn test_cutoff_requeues_when_invalidated_during_evaluation() {
    let source = Property::new(1);
    let source_for_cutoff = source.clone();
    let source_to_invalidate = source.clone();
    let cutoff = Property::computed_with_cutoff(
        move || {
            let value = source_for_cutoff.get();
            if value == 2 {
                source_to_invalidate.set(3);
            }
            value
        },
        &[source.untyped()],
        i32::eq,
    );
    let seen = Rc::new(Cell::new(0));
    let cutoff_for_effect = cutoff.clone();
    let seen_for_effect = Rc::clone(&seen);
    let effect = Property::computed(
        move || seen_for_effect.set(cutoff_for_effect.get()),
        &[cutoff.untyped()],
    );
    register_effect_property(&effect);

    drain_effects(10);
    assert_eq!(seen.get(), 1);

    source.set(2);
    assert_eq!(drain_effects(10), 3);
    assert_eq!(cutoff.get(), 3);
    assert_eq!(seen.get(), 3);
}

#[test]
fn test_replacement_preserves_cutoff_semantics() {
    let source = Property::new(1);
    let source_for_cutoff = source.clone();
    let cutoff = Property::computed_with_cutoff(
        move || source_for_cutoff.get() % 2,
        &[source.untyped()],
        i32::eq,
    );
    let slot = Property::new(99);
    let effect_runs = Rc::new(Cell::new(0));
    let seen = Rc::new(Cell::new(0));
    let slot_for_effect = slot.clone();
    let effect_runs_for_effect = Rc::clone(&effect_runs);
    let seen_for_effect = Rc::clone(&seen);
    let effect = Property::computed(
        move || {
            effect_runs_for_effect.set(effect_runs_for_effect.get() + 1);
            seen_for_effect.set(slot_for_effect.get());
        },
        &[slot.untyped()],
    );
    register_effect_property(&effect);
    drain_effects(10);

    slot.replace_with(cutoff);
    drain_effects(10);
    assert_eq!(seen.get(), 1);
    let baseline_runs = effect_runs.get();

    source.set(3);
    drain_effects(10);
    assert_eq!(effect_runs.get(), baseline_runs);

    source.set(4);
    drain_effects(10);
    assert_eq!(effect_runs.get(), baseline_runs + 1);
    assert_eq!(seen.get(), 0);
}

#[test]
fn test_cutoff_work_obeys_drain_budget() {
    let source = Property::new(1);
    let source_for_cutoff = source.clone();
    let cutoff = Property::computed_with_cutoff(
        move || source_for_cutoff.get(),
        &[source.untyped()],
        i32::eq,
    );
    let effect_runs = Rc::new(Cell::new(0));
    let cutoff_for_effect = cutoff.clone();
    let effect_runs_for_effect = Rc::clone(&effect_runs);
    let effect = Property::computed(
        move || {
            let _ = cutoff_for_effect.get();
            effect_runs_for_effect.set(effect_runs_for_effect.get() + 1);
        },
        &[cutoff.untyped()],
    );
    register_effect_property(&effect);

    let report = drain_effects_with_report(1);
    assert_eq!(report.cutoffs_evaluated, 1);
    assert_eq!(report.ran, 0);
    assert!(report.budget_exhausted);
    assert_eq!(report.remaining, 1);

    let report = drain_effects_with_report(1);
    assert_eq!(report.cutoffs_evaluated, 0);
    assert_eq!(report.ran, 1);
    assert_eq!(effect_runs.get(), 1);
    assert!(!report.budget_exhausted);
}
