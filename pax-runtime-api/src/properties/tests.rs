use super::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[test]
fn test_literal_set_get() {
    let prop = LocalProperty::new(5);
    assert_eq!(prop.get(), 5);
    prop.set(2);
    assert_eq!(prop.get(), 2);
}

#[test]
fn test_cancel_transitions_freezes_value_and_clears_queue() {
    let frames = LocalProperty::new(0_u64);
    let millis = LocalProperty::new(0_u64);
    register_time(&frames);
    register_millis(&millis);

    let prop = LocalProperty::new(0.0);
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
    let prop = LocalProperty::<i32>::computed(|| 42, &[]);
    assert_eq!(prop.get(), 42);
}

#[test]
fn test_computed_dependent_on_literal() {
    let prop_1 = LocalProperty::new_with_name(2, "p1");
    let p1 = prop_1.clone();
    let prop_2 =
        LocalProperty::<i32>::computed_with_name(move || p1.get() * 5, &[prop_1.untyped()], "p2");
    assert_eq!(prop_2.get(), 10);
    prop_1.set(3);
    assert_eq!(prop_2.get(), 15);
}

#[test]
fn optional_inputs_follow_the_evaluated_slot_after_replacement() {
    let active = LocalProperty::new(false);
    let input = LocalProperty::new(1);
    let computation = {
        let (active, input) = (active.clone(), input.clone());
        LocalProperty::computed_with_optional_inputs(
            &[active.untyped()],
            [input.untyped()],
            move |[select_input]| {
                let enabled = active.get();
                select_input(enabled);
                if enabled {
                    input.get()
                } else {
                    0
                }
            },
        )
    };
    assert_eq!(computation.get(), 0);
    let copied = LocalProperty::new(-1);
    copied.replace_with(computation.clone());
    active.set(true);
    assert_eq!(copied.get(), 1);
    input.set(2);
    assert_eq!(copied.get(), 2);
    assert_eq!(computation.get(), 2);
    active.set(false);
    assert_eq!(computation.get(), 0);
    drop(computation);
    active.set(true);
    input.set(3);
    assert_eq!(copied.get(), 3);
    input.set(4);
    assert_eq!(copied.get(), 4);
}

#[test]
fn test_property_replacement() {
    let prop_1 = LocalProperty::new(2);
    let p1 = prop_1.clone();
    let prop_2 = LocalProperty::computed(move || p1.get(), &[prop_1.untyped()]);

    let prop_3 = LocalProperty::new(6);
    let p3 = prop_3.clone();
    let prop_4 = LocalProperty::computed(move || p3.get(), &[prop_3.untyped()]);

    assert_eq!(prop_2.get(), 2);
    assert_eq!(prop_4.get(), 6);
    prop_3.replace_with(prop_2);
    assert_eq!(prop_4.get(), 2);
}

#[test]
fn test_property_replacement_with_self_is_noop() {
    let prop = LocalProperty::new(7);

    prop.replace_with(prop.clone());

    assert_eq!(prop.get(), 7);
}

#[test]
fn test_property_replacement_disconnects_previous_dependencies() {
    let old_source = LocalProperty::new(1);
    let old_source_for_computed = old_source.clone();
    let slot = LocalProperty::computed(
        move || old_source_for_computed.get(),
        &[old_source.untyped()],
    );
    let effect_runs = Rc::new(Cell::new(0));
    let effect_runs_for_effect = Rc::clone(&effect_runs);
    let slot_for_effect = slot.clone();
    let effect = LocalProperty::computed(
        move || {
            let _ = slot_for_effect.get();
            effect_runs_for_effect.set(effect_runs_for_effect.get() + 1);
        },
        &[slot.untyped()],
    );
    register_effect_property(&effect);
    drain_effects(10);

    slot.replace_with(LocalProperty::new(0));
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
    let a = LocalProperty::new(1);
    let b = LocalProperty::new(2);
    let c = LocalProperty::new(3);
    let read_a = a.clone();
    let slot = LocalProperty::computed(
        move || read_a.get(),
        &[a.untyped(), b.untyped(), c.untyped()],
    );
    let read_slot = slot.clone();
    let dependent = LocalProperty::computed(move || read_slot.get() * 10, &[slot.untyped()]);
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
            .map(LocalProperty::untyped)
            .collect::<Vec<_>>();
        let reads = dependencies.clone();
        let replacement = LocalProperty::computed(
            move || reads.iter().map(LocalProperty::get).sum::<i32>(),
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
    let source = LocalProperty::new(1);
    let read_source = source.clone();
    let first = LocalProperty::computed(move || read_source.get(), &[source.untyped()]);
    let read_source = source.clone();
    let sibling = LocalProperty::computed(move || read_source.get(), &[source.untyped()]);
    let read_source = source.clone();
    let replacement = LocalProperty::computed(move || read_source.get() + 10, &[source.untyped()]);

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
fn direct_rebinding_preserves_synchronous_reads_order_and_cutoff_propagation() {
    let source = LocalProperty::new(3);
    let old = LocalProperty::new(99);
    let previous = old.clone();
    let slot = LocalProperty::computed(move || previous.get(), &[old.untyped()]);
    let sibling = LocalProperty::computed(|| 0, &[source.untyped()]);
    let read = slot.clone();
    let runs = Rc::new(Cell::new(0));
    let count = runs.clone();
    let effect = LocalProperty::computed(
        move || {
            read.get();
            count.set(count.get() + 1);
        },
        &[slot.untyped()],
    );
    register_effect_property(&effect);
    drain_effects(10);

    let input = source.clone();
    slot.replace_with_computed(
        move || input.get() + 10,
        &[source.untyped(), source.untyped()],
        None,
    );
    assert_eq!(slot.get(), 13);
    PROPERTY_TABLE.with(|table| {
        assert!(table.with_property_data(old.untyped.id, |p| p.outbound.is_empty()));
        assert_eq!(
            table.with_property_data(source.untyped.id, |p| p.outbound.clone()),
            vec![sibling.untyped.id, slot.untyped.id, slot.untyped.id]
        );
    });
    let input = source.clone();
    slot.replace_with_computed_cutoff(
        move || input.get() % 2,
        &[source.untyped()],
        i32::eq,
        "parity",
    );
    assert_eq!(slot.get(), 1);
    drain_effects(10);
    let before = runs.get();
    source.set(5);
    drain_effects(10);
    assert_eq!(runs.get(), before);
    source.set(6);
    assert_eq!(slot.get(), 0);
    drain_effects(10);
    assert_eq!(runs.get(), before + 1);
}

#[test]
fn test_larger_network() {
    let prop_1 = LocalProperty::new(2);
    let prop_2 = LocalProperty::new(6);

    let p1 = prop_1.clone();
    let p2 = prop_2.clone();
    let prop_3 = LocalProperty::computed(
        move || p1.get() * p2.get(),
        &[prop_1.untyped(), prop_2.untyped()],
    );
    let p1 = prop_1.clone();
    let p3 = prop_3.clone();
    let prop_4 = LocalProperty::computed(
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
    let prop = LocalProperty::new(5);
    assert_eq!(PROPERTY_TABLE.with(|t| t.property_map.borrow().len()), 1);
    drop(prop);
    assert!(PROPERTY_TABLE.with(|t| t.property_map.borrow().is_empty()));
}

#[test]
fn test_recursive_props() {
    {
        let prop_of_prop = LocalProperty::new(LocalProperty::new(3));
        let prop_of_prop_clone = prop_of_prop.clone();
        prop_of_prop_clone.get().set(1);
        assert_eq!(prop_of_prop.get().get(), prop_of_prop_clone.get().get());
    }
    assert!(PROPERTY_TABLE.with(|t| t.property_map.borrow().is_empty()));
}

#[test]
fn test_registered_effect_drains_when_dirty() {
    let source = LocalProperty::new(1);
    let eval_count = Rc::new(Cell::new(0));
    let seen_value = Rc::new(Cell::new(0));

    let source_for_effect = source.clone();
    let eval_count_for_effect = Rc::clone(&eval_count);
    let seen_value_for_effect = Rc::clone(&seen_value);
    let effect = LocalProperty::computed(
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
    let source = LocalProperty::new(1);
    let eval_count = Rc::new(Cell::new(0));

    let source_for_effect = source.clone();
    let eval_count_for_effect = Rc::clone(&eval_count);
    let effect = LocalProperty::computed(
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
    let source = LocalProperty::new(false);
    let source_for_computed = source.clone();
    let computed = LocalProperty::computed(move || source_for_computed.get(), &[source.untyped()]);

    assert!(!computed.get());
    source.set(true);
    computed.invalidate();

    assert!(computed.get());
}

#[test]
fn test_cutoff_suppresses_equivalent_output_propagation() {
    let source = LocalProperty::new(1);
    let source_for_cutoff = source.clone();
    let cutoff = LocalProperty::computed_with_cutoff(
        move || source_for_cutoff.get() % 2,
        &[source.untyped()],
        i32::eq,
    );
    let effect_runs = Rc::new(Cell::new(0));
    let seen = Rc::new(Cell::new(0));
    let cutoff_for_effect = cutoff.clone();
    let effect_runs_for_effect = Rc::clone(&effect_runs);
    let seen_for_effect = Rc::clone(&seen);
    let effect = LocalProperty::computed(
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
    let source = LocalProperty::new(0.0);
    let source_for_cutoff = source.clone();
    let cutoff = LocalProperty::computed_with_cutoff(
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
    let source = LocalProperty::new(1);
    let source_for_first = source.clone();
    let first = LocalProperty::computed_with_cutoff(
        move || source_for_first.get() % 2,
        &[source.untyped()],
        i32::eq,
    );
    let first_for_second = first.clone();
    let second = LocalProperty::computed_with_cutoff(
        move || first_for_second.get() * 10,
        &[first.untyped()],
        i32::eq,
    );
    let effect_runs = Rc::new(Cell::new(0));
    let seen = Rc::new(Cell::new(0));
    let second_for_effect = second.clone();
    let effect_runs_for_effect = Rc::clone(&effect_runs);
    let seen_for_effect = Rc::clone(&seen);
    let effect = LocalProperty::computed(
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
    let source = LocalProperty::new(1);
    let source_for_cutoff = source.clone();
    let cutoff_branch = LocalProperty::computed_with_cutoff(
        move || source_for_cutoff.get() * 2,
        &[source.untyped()],
        i32::eq,
    );
    let source_for_eager = source.clone();
    let eager_branch =
        LocalProperty::computed(move || source_for_eager.get() * 3, &[source.untyped()]);
    let effect_runs = Rc::new(Cell::new(0));
    let seen = Rc::new(Cell::new((0, 0)));
    let cutoff_for_effect = cutoff_branch.clone();
    let eager_for_effect = eager_branch.clone();
    let effect_runs_for_effect = Rc::clone(&effect_runs);
    let seen_for_effect = Rc::clone(&seen);
    let effect = LocalProperty::computed(
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
    let source = LocalProperty::new(1);
    let source_for_cutoff = source.clone();
    let cutoff = LocalProperty::computed_with_cutoff(
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
    let source = LocalProperty::new(1);
    let source_for_cutoff = source.clone();
    let cutoff = LocalProperty::computed_with_cutoff(
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
    let source = LocalProperty::new(1);
    let source_for_cutoff = source.clone();
    let cutoff = LocalProperty::computed_with_cutoff(
        move || source_for_cutoff.get(),
        &[source.untyped()],
        i32::eq,
    );
    let effect_runs = Rc::new(Cell::new(0));
    let effect_runs_for_effect = Rc::clone(&effect_runs);
    let effect = LocalProperty::computed(
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
    let source = LocalProperty::new(1);
    let source_for_cutoff = source.clone();
    let source_to_invalidate = source.clone();
    let cutoff = LocalProperty::computed_with_cutoff(
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
    let effect = LocalProperty::computed(
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
    let source = LocalProperty::new(1);
    let source_for_cutoff = source.clone();
    let cutoff = LocalProperty::computed_with_cutoff(
        move || source_for_cutoff.get() % 2,
        &[source.untyped()],
        i32::eq,
    );
    let slot = LocalProperty::new(99);
    let effect_runs = Rc::new(Cell::new(0));
    let seen = Rc::new(Cell::new(0));
    let slot_for_effect = slot.clone();
    let effect_runs_for_effect = Rc::clone(&effect_runs);
    let seen_for_effect = Rc::clone(&seen);
    let effect = LocalProperty::computed(
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
fn paint_cutoffs_settle_after_layout_but_read_the_current_pose_immediately() {
    let source = LocalProperty::new(0);
    let next = LocalProperty::new(0);
    let samples = Rc::new(RefCell::new(Vec::new()));
    let placement = LocalProperty::new(0);
    let paint = LocalProperty::computed(
        {
            let placement = placement.clone();
            move || {
                placement.get();
            }
        },
        &[placement.untyped()],
    );
    let cutoff = LocalProperty::computed_with_cutoff(
        {
            let source = source.clone();
            let samples = samples.clone();
            move || {
                let value = source.get();
                samples.borrow_mut().push(value);
                value
            }
        },
        &[source.untyped()],
        i32::eq,
    );
    placement.replace_with_paint_cutoff(cutoff, &paint, false);
    register_effect_property(&paint);
    let layout = LocalProperty::computed(
        {
            let next = next.clone();
            let source = source.clone();
            move || source.set(next.get())
        },
        &[next.untyped()],
    );
    register_effect_property(&layout);
    drain_effects(10);
    samples.borrow_mut().clear();

    source.set(1);
    next.set(2);
    let report = drain_effects_with_report(1);
    assert_eq!((report.ran, report.remaining_cutoffs), (1, 1));
    assert!(samples.borrow().is_empty());
    assert_eq!(drain_effects(10), 2);
    assert_eq!(&*samples.borrow(), &[2]);

    source.set(3);
    assert_eq!(placement.get(), 3);
    source.set(4);
    drain_effects(10);
    assert_eq!(&*samples.borrow(), &[2, 3, 4]);
}

#[test]
fn presentation_effect_coalesces_layout_writes_and_replacement_restores_normal_order() {
    let source = LocalProperty::new(0);
    let layout_target = LocalProperty::new(0);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let effect = LocalProperty::new(());
    let clip = LocalProperty::computed_with_cutoff(
        {
            let source = source.clone();
            move || source.get()
        },
        &[source.untyped()],
        i32::eq,
    )
    .for_presentation_listener(&effect);
    effect.replace_with_presentation_effect(
        {
            let clip = clip.clone();
            let seen = seen.clone();
            move || seen.borrow_mut().push(clip.get())
        },
        &[source.untyped(), clip.untyped()],
        None,
    );
    register_effect_property(&effect);
    let layout = LocalProperty::computed(
        {
            let source = source.clone();
            let target = layout_target.clone();
            move || source.set(target.get())
        },
        &[layout_target.untyped()],
    );
    register_effect_property(&layout);
    drain_effects(20);
    seen.borrow_mut().clear();

    source.set(1);
    assert_eq!(clip.get(), 1);
    layout_target.set(2);
    assert_eq!(drain_effects_with_report(1).remaining, 1);
    drain_effects(20);
    assert_eq!(&*seen.borrow(), &[2]);

    source.set(3);
    effect.get();
    assert_eq!(&*seen.borrow(), &[2, 3]);
    source.set(4);
    effect.replace_with_computed(
        {
            let source = source.clone();
            let seen = seen.clone();
            move || seen.borrow_mut().push(source.get())
        },
        &[source.untyped()],
        None,
    );
    layout_target.set(5);
    drain_effects(20);
    assert_eq!(&*seen.borrow(), &[2, 3, 4, 5]);
}

#[test]
fn unused_paint_is_pulled_on_read_or_on_its_first_subscription() {
    let source = LocalProperty::new(0);
    let placement = LocalProperty::new(0);
    let paint = LocalProperty::new(());
    let reads = Rc::new(Cell::new(0));
    let cutoff = LocalProperty::computed_with_cutoff(
        {
            let source = source.clone();
            let reads = reads.clone();
            move || {
                reads.set(reads.get() + 1);
                source.get()
            }
        },
        &[source.untyped()],
        i32::eq,
    );
    placement.replace_with_paint_cutoff(cutoff, &paint, false);
    drain_effects(10);
    reads.set(0);
    for value in [1, 2] {
        source.set(value);
        drain_effects(10);
    }
    assert_eq!(reads.get(), 0);
    assert_eq!(placement.get(), 2);
    source.set(3);
    let observed = LocalProperty::computed(
        {
            let placement = placement.clone();
            move || placement.get()
        },
        &[placement.untyped()],
    );
    drain_effects(10);
    assert_eq!(reads.get(), 2);
    assert_eq!(observed.get(), 3);
    source.set(4);
    drain_effects(10);
    assert_eq!(reads.get(), 3);
    assert_eq!(observed.get(), 4);

    drop(observed);
    source.set(5);
    drain_effects(10);
    assert_eq!(reads.get(), 3);
    let events = DependencyEventQueue::default();
    let _watch = events.watch(&[placement.untyped()], 7);
    assert_eq!(events.pop(), Some(7));
    assert_eq!(placement.get(), 5);
    source.set(6);
    drain_effects(10);
    assert_eq!(events.pop(), Some(7));
    assert_eq!(reads.get(), 5);
}

#[test]
fn adding_an_ordinary_observer_promotes_already_queued_paint() {
    let source = LocalProperty::new(0);
    let placement = LocalProperty::new(0);
    let paint = LocalProperty::new(());
    let cutoff = LocalProperty::computed_with_cutoff(
        {
            let source = source.clone();
            move || source.get()
        },
        &[source.untyped()],
        i32::eq,
    );
    placement.replace_with_paint_cutoff(cutoff, &paint, false);
    drain_effects(10);
    source.set(1);
    let regular = LocalProperty::computed(
        {
            let placement = placement.clone();
            move || placement.get()
        },
        &[placement.untyped()],
    );
    let effect = LocalProperty::computed(|| {}, &[]);
    register_effect_property(&effect);
    let report = drain_effects_with_report(1);
    assert_eq!(
        (
            report.cutoffs_evaluated,
            report.ran,
            report.remaining_cutoffs
        ),
        (1, 0, 0)
    );
    assert_eq!(regular.get(), 1);
    drain_effects(10);
    source.set(2);
    effect.invalidate();
    assert_eq!(drain_effects_with_report(1).cutoffs_evaluated, 1);
    assert_eq!(regular.get(), 2);
    drain_effects(10);
}

#[test]
fn test_cutoff_work_obeys_drain_budget() {
    let source = LocalProperty::new(1);
    let source_for_cutoff = source.clone();
    let cutoff = LocalProperty::computed_with_cutoff(
        move || source_for_cutoff.get(),
        &[source.untyped()],
        i32::eq,
    );
    let effect_runs = Rc::new(Cell::new(0));
    let cutoff_for_effect = cutoff.clone();
    let effect_runs_for_effect = Rc::clone(&effect_runs);
    let effect = LocalProperty::computed(
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

#[test]
fn disconnect_temporary_and_duplicate_dependencies_preserves_other_edge_order() {
    let source = LocalProperty::new(1);
    let dependencies = [source.untyped(), source.untyped()];
    let first = LocalProperty::computed(|| 1, &dependencies);
    let middle = LocalProperty::computed(|| 2, &dependencies);
    let last = LocalProperty::computed(|| 3, &dependencies);
    let outbound = || {
        properties_table::PROPERTY_TABLE.with(|table| {
            table.with_property_data(source.untyped().get_id(), |data| data.outbound.clone())
        })
    };
    let original = outbound();
    assert_eq!(original.len(), 6);
    drop(last);
    assert_eq!(outbound(), original[..4]);
    drop(first);
    assert_eq!(outbound(), original[2..4]);
    source.set(4);
    assert_eq!(middle.get(), 2);
    drop(middle);
    assert!(outbound().is_empty());
}

#[test]
fn applications_ease_against_their_own_clock() {
    fn create() -> (PropertyGraph, LocalProperty<u64>, LocalProperty<f64>) {
        let graph = PropertyGraph::new(|| {});
        let _entered = graph.enter();
        register_time(&LocalProperty::new(0));
        let millis = LocalProperty::new(0);
        register_millis(&millis);
        let value = LocalProperty::new(0.0);
        value.ease_to(
            10.0,
            Duration::Milliseconds(100.into()),
            EasingCurve::Linear,
        );
        drop(_entered);
        (graph, millis, value)
    }
    let (loader, loader_clock, loader_value) = create();
    let (app, app_clock, app_value) = create();
    {
        let _entered = loader.enter();
        loader_clock.set(50);
        assert_eq!(loader_value.get(), 5.0);
    }
    {
        let _entered = app.enter();
        assert_eq!(app_value.get(), 0.0);
        app_clock.set(20);
        assert_eq!(app_value.get(), 2.0);
    }
    let _entered = loader.enter();
    loader_clock.set(80);
    assert_eq!(loader_value.get(), 8.0);
}
