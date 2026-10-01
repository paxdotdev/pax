use super::*;
use std::{
    cell::Cell,
    rc::Rc,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Barrier,
    },
    thread,
};

#[test]
fn worker_publication_uses_shared_storage_and_only_owner_evaluates() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<Property<u32>>();
    let wakes = Arc::new(AtomicUsize::new(0));
    let signal = wakes.clone();
    let graph = PropertyGraph::new(move || {
        signal.fetch_add(1, Ordering::SeqCst);
    });
    let _entered = graph.enter();
    let shared = Property::new(7_u32);
    let local = shared.local();
    assert_eq!(shared.observed_revision(), Some(0));
    let source = local.clone();
    let owner = thread::current().id();
    let evaluations = Rc::new(Cell::new(0));
    let count = evaluations.clone();
    let computed = LocalProperty::computed(
        move || {
            assert_eq!(thread::current().id(), owner);
            count.set(count.get() + 1);
            source.get() * 2
        },
        &[local.untyped()],
    );
    assert_eq!(computed.get(), 14);
    let worker = shared.clone();
    thread::spawn(move || {
        let unrelated = LocalProperty::new(99_u32);
        assert_eq!(worker.get(), 7);
        for value in 8..=21 {
            worker.set(value);
        }
        assert_eq!(unrelated.get(), 99);
    })
    .join()
    .unwrap();
    assert_eq!(shared.get(), 21);
    assert_eq!(shared.observed_revision(), Some(0));
    assert_eq!(computed.get(), 14);
    assert_eq!(evaluations.get(), 1);
    assert_eq!(wakes.load(Ordering::SeqCst), 1);
    assert_eq!(graph.import(), 1);
    assert_eq!(shared.observed_revision(), Some(shared.revision()));
    assert_eq!(computed.get(), 42);
    assert_eq!(evaluations.get(), 2);
    shared.set(22);
    assert_eq!(wakes.load(Ordering::SeqCst), 2);
    graph.close();
    shared.set(23);
    assert_eq!(wakes.load(Ordering::SeqCst), 2);
    assert_eq!(graph.import(), 0);
}

#[test]
fn concurrent_updates_are_atomic_and_callbacks_run_once() {
    let value = Property::new(0_usize);
    let calls = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(Barrier::new(5));
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let value = value.clone();
            let calls = calls.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                for _ in 0..1000 {
                    value.update(|n| {
                        calls.fetch_add(1, Ordering::Relaxed);
                        *n += 1;
                    });
                }
            })
        })
        .collect();
    barrier.wait();
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(value.get(), 4000);
    assert_eq!(calls.load(Ordering::Relaxed), 4000);
    assert_eq!(value.revision(), 4000);
    assert!(!value.set_if_neq(4000));
    assert_eq!(value.revision(), 4000);
}

#[test]
fn read_snapshots_allow_reentrant_mutation_and_update_panic_rolls_back() {
    let value = Property::new(1_u32);
    value.read(|snapshot| {
        value.set(2);
        assert_eq!(*snapshot, 1);
        assert_eq!(value.get(), 2);
    });
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        value.update(|draft| {
            *draft = 99;
            panic!("stop");
        });
    }));
    assert!(failed.is_err());
    assert_eq!(value.get(), 2);
    value.update(|n| *n += 1);
    assert_eq!(value.get(), 3);
    let recursive = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        value.update(|_| value.set(999));
    }));
    assert!(recursive.is_err());
    assert_eq!(value.get(), 3);
    value.set(4);
    assert_eq!(value.get(), 4);
}

#[test]
fn local_alias_writes_and_computed_outputs_publish_back_to_shared_state() {
    let graph = PropertyGraph::new(|| {});
    let _entered = graph.enter();
    let source = Property::new(2_u32);
    let local = source.local();
    let alias = Property::<u32>::from_local(local.clone());
    local.set(3);
    assert_eq!(source.get(), 3);
    assert_eq!(alias.get(), 3);
    let read = local.clone();
    let computed = LocalProperty::computed(move || read.get() * 10, &[local.untyped()]);
    let output = Property::<u32>::from_local(computed);
    assert_eq!(output.get(), 30);
    source.set(4);
    assert_eq!(output.get(), 30);
    graph.import();
    drain_effects(100);
    assert_eq!(output.get(), 40);
    graph.close();
}

#[test]
fn literal_binding_publishes_on_settlement_without_an_animation() {
    let graph = PropertyGraph::new(|| {});
    let _entered = graph.enter();
    let label = Property::new(String::from("default"));
    label.replace_with(LocalProperty::new(String::from("authored")));
    drain_effects(100);
    assert_eq!(label.get(), "authored");
    graph.close();
}

#[test]
fn publication_during_evaluation_waits_for_next_import() {
    let graph = PropertyGraph::new(|| {});
    let _entered = graph.enter();
    let value = Property::new(1_u32);
    let local = value.local();
    let read = local.clone();
    let write = value.clone();
    let computed = LocalProperty::computed(
        move || {
            let before = read.get();
            write.set(before + 1);
            assert_eq!(read.get(), before);
            before
        },
        &[local.untyped()],
    );
    assert_eq!(computed.get(), 1);
    assert_eq!(value.get(), 2);
    assert_eq!(computed.get(), 1);
    graph.import();
    assert_eq!(computed.get(), 2);
    assert_eq!(value.get(), 3);
    graph.close();
}

#[test]
fn two_graphs_observe_independently_and_closed_graph_cannot_receive() {
    let value = Property::new(10_u32);
    let first = PropertyGraph::new(|| {});
    let a = {
        let _entered = first.enter();
        value.local()
    };
    let second = PropertyGraph::new(|| {});
    let b = {
        let _entered = second.enter();
        value.local()
    };
    value.set(11);
    first.import();
    assert_eq!(a.get(), 11);
    assert_eq!(b.get(), 10);
    first.close();
    value.set(12);
    second.import();
    assert_eq!(a.get(), 11);
    assert_eq!(b.get(), 12);
    second.close();
}

#[test]
fn dropped_shared_handles_release_owner_thread_graph_entries() {
    let before = property_table_total_properties_count();
    let graph = PropertyGraph::new(|| {});
    {
        let _entered = graph.enter();
        let value = Property::new(1_u32);
        let local = value.local();
        assert_eq!(local.get(), 1);
        thread::spawn(move || drop(value)).join().unwrap();
    }
    graph.collect();
    assert_eq!(property_table_total_properties_count(), before);
    graph.close();
}

#[test]
fn nested_conversion_uses_explicit_graph_snapshots() {
    use crate::{Size, Stroke, ToPaxValue};
    let graph = PropertyGraph::new(|| {});
    let _entered = graph.enter();
    let stroke = Stroke::default();
    stroke.width.set(Size::Pixels(2.into()));
    let frozen = stroke.clone().to_pax_value_in_graph();
    let width = stroke.width.clone();
    thread::spawn(move || width.set(Size::Pixels(5.into())))
        .join()
        .unwrap();
    assert_eq!(stroke.clone().to_pax_value_in_graph(), frozen);
    assert_ne!(stroke.clone().to_pax_value(), frozen);
    graph.import();
    assert_eq!(
        stroke.clone().to_pax_value_in_graph(),
        stroke.to_pax_value()
    );
    graph.close();
}

#[test]
fn shared_cutoff_outputs_publish_only_accepted_values() {
    let graph = PropertyGraph::new(|| {});
    let _entered = graph.enter();
    let source = Property::new(10_u32);
    let local = source.local();
    let read = local.clone();
    let computed = LocalProperty::computed_with_cutoff(
        move || read.get() / 10,
        &[local.untyped()],
        |a, b| a == b,
    );
    let result = Property::from_local(computed);
    assert_eq!(result.get(), 1);
    let revision = result.revision();
    source.set(11);
    graph.import();
    drain_effects(100);
    assert_eq!(result.revision(), revision);
    source.set(20);
    graph.import();
    drain_effects(100);
    assert_eq!(result.get(), 2);
    assert_eq!(result.revision(), revision + 1);
    graph.close();
}

#[test]
fn evaluator_keeps_its_graph_when_another_graph_is_entered() {
    let shared = Property::new(1_u32);
    let first = PropertyGraph::new(|| {});
    let computed = {
        let _entered = first.enter();
        let local = shared.local();
        let value = shared.clone();
        LocalProperty::computed(move || value.local().get(), &[local.untyped()])
    };
    let second = PropertyGraph::new(|| {});
    let _entered = second.enter();
    shared.set(2);
    assert_eq!(shared.local().get(), 2);
    assert_eq!(computed.get(), 1);
    first.import();
    assert_eq!(computed.get(), 2);
    first.close();
    second.close();
}

#[test]
fn draining_one_application_does_not_run_another_applications_effects() {
    let first = PropertyGraph::new(|| {});
    let second = PropertyGraph::new(|| {});
    let first_runs = Rc::new(Cell::new(0));
    let second_runs = Rc::new(Cell::new(0));
    let make_effect = |graph: &PropertyGraph, runs: Rc<Cell<u32>>| {
        let _entered = graph.enter();
        let effect = LocalProperty::computed(move || runs.set(runs.get() + 1), &[]);
        register_effect_property(&effect);
        effect
    };
    let _a = make_effect(&first, first_runs.clone());
    let _b = make_effect(&second, second_runs.clone());
    {
        let _entered = first.enter();
        drain_effects(100);
    }
    assert_eq!(first_runs.get(), 1);
    assert_eq!(second_runs.get(), 0);
    {
        let _entered = second.enter();
        drain_effects(100);
    }
    assert_eq!(second_runs.get(), 1);
}

#[test]
fn unmount_detaches_retained_data_and_remount_restores_existing_local_identity() {
    let wakes = Arc::new(AtomicUsize::new(0));
    let signal = wakes.clone();
    let graph = PropertyGraph::new(move || {
        signal.fetch_add(1, Ordering::SeqCst);
    });
    let _entered = graph.enter();
    let shared = Property::new(1_u32);
    let scope = graph.binding_scope();
    let local = {
        let _scope = scope.enter();
        shared.local()
    };
    let local_id = local.untyped().get_id();
    scope.close();
    let worker = shared.clone();
    thread::spawn(move || worker.set(2)).join().unwrap();
    assert_eq!(wakes.load(Ordering::SeqCst), 0);
    assert_eq!(graph.import(), 0);
    assert_eq!(local.get(), 1);
    scope.resume();
    let _scope = scope.enter();
    assert_eq!(shared.local().untyped().get_id(), local_id);
    assert_eq!(local.get(), 2);
    shared.set(3);
    graph.import();
    assert_eq!(local.get(), 3);
    scope.close();
    graph.close();
}

#[test]
fn unmount_releases_local_evaluator_when_only_shared_value_is_retained() {
    let before = property_table_total_properties_count();
    let graph = PropertyGraph::new(|| {});
    let _entered = graph.enter();
    let scope = graph.binding_scope();
    let captured = Rc::new(());
    let shared = {
        let _scope = scope.enter();
        let captured = captured.clone();
        Property::from_local(LocalProperty::computed(
            move || {
                let _ = &captured;
                42_u32
            },
            &[],
        ))
    };
    assert_eq!(Rc::strong_count(&captured), 2);
    scope.close();
    graph.collect();
    assert_eq!(Rc::strong_count(&captured), 1);
    assert_eq!(property_table_total_properties_count(), before);
    assert_eq!(shared.get(), 42);
    shared.set(43);
    assert!(!graph.has_pending());
    graph.close();
}

#[test]
fn unmount_does_not_detach_another_live_views_projection() {
    let graph = PropertyGraph::new(|| {});
    let _entered = graph.enter();
    let shared = Property::new(1_u32);
    let first = graph.binding_scope();
    let second = graph.binding_scope();
    let a = {
        let _scope = first.enter();
        shared.local()
    };
    let b = {
        let _scope = second.enter();
        shared.local()
    };
    assert_eq!(a.untyped().get_id(), b.untyped().get_id());
    first.close();
    shared.set(2);
    assert_eq!(graph.import(), 1);
    assert_eq!(b.get(), 2);
    drain_effects(100);
    second.close();
    shared.set(3);
    assert!(!graph.has_pending());
    graph.close();
}

#[test]
fn retained_computation_cannot_resubscribe_after_its_view_closes() {
    let graph = PropertyGraph::new(|| {});
    let _entered = graph.enter();
    let shared = Property::new(1_u32);
    let scope = graph.binding_scope();
    let (local, computed) = {
        let _scope = scope.enter();
        let local = shared.local();
        let read = shared.clone();
        let computed = LocalProperty::computed(move || read.local().get(), &[local.untyped()]);
        (local, computed)
    };
    assert_eq!(computed.get(), 1);
    scope.close();
    shared.set(2);
    local.set(3);
    assert_eq!(computed.get(), 3);
    shared.set(4);
    assert!(!graph.has_pending());
    scope.resume();
    assert_eq!(computed.get(), 4);
    graph.close();
}

#[test]
fn remount_recomputes_a_retained_derived_binding_from_new_shared_inputs() {
    let graph = PropertyGraph::new(|| {});
    let _entered = graph.enter();
    let scope = graph.binding_scope();
    let input = Property::new(2_u32);
    let (result, retained) = {
        let _scope = scope.enter();
        let read = input.local();
        let deps = [read.untyped()];
        let computed = LocalProperty::computed(move || read.get() * 10, &deps);
        (Property::from_local(computed.clone()), computed)
    };
    assert_eq!(result.get(), 20);
    scope.close();
    input.set(7);
    assert!(!graph.has_pending());
    scope.resume();
    drain_effects(100);
    assert_eq!(retained.get(), 70);
    assert_eq!(result.get(), 70);
    graph.close();
}

#[test]
fn retained_local_computations_remain_readable_without_reviving_a_view() {
    let graph = PropertyGraph::new(|| {});
    let _graph = graph.enter();
    let scope = graph.binding_scope();
    let input = LocalProperty::new(1_u32);
    let calls = Rc::new(Cell::new(0));
    let computed = LocalProperty::default();
    {
        let _scope = scope.enter();
        let source = input.clone();
        let count = calls.clone();
        computed.replace_with(LocalProperty::computed(
            move || {
                count.set(count.get() + 1);
                source.get() * 2
            },
            &[input.untyped()],
        ));
    }
    assert_eq!(computed.get(), 2);
    scope.close();
    input.set(3);
    assert_eq!(computed.get(), 6);
    assert_eq!(calls.get(), 2);
    scope.resume();
    assert_eq!(computed.get(), 6);
    assert_eq!(calls.get(), 2);
    graph.close();
}

#[test]
fn bounded_imports_are_fifo_and_request_continuation() {
    let wakes = Arc::new(AtomicUsize::new(0));
    let signal = wakes.clone();
    let graph = PropertyGraph::new(move || {
        signal.fetch_add(1, Ordering::SeqCst);
    });
    let _entered = graph.enter();
    let a = Property::new(0_u32);
    let b = Property::new(0_u32);
    let a_local = a.local();
    let b_local = b.local();
    a.set(1);
    b.set(2);
    a.set(3);
    assert_eq!(graph.import_batch(1), 1);
    assert_eq!(a_local.get(), 3);
    assert_eq!(b_local.get(), 0);
    assert_eq!(wakes.load(Ordering::SeqCst), 2);
    a.set(4);
    assert_eq!(graph.import_batch(1), 1);
    assert_eq!(b_local.get(), 2);
    assert_eq!(a_local.get(), 3);
    assert_eq!(graph.import_batch(1), 1);
    assert_eq!(a_local.get(), 4);
    assert!(!graph.has_pending());
    graph.close();
}

#[test]
fn drawing_snapshots_use_settled_nested_fields_until_next_import() {
    use crate::{Material, MaterialParams, Size, Stroke};
    let graph = PropertyGraph::new(|| {});
    let _entered = graph.enter();
    let stroke = Stroke::default();
    let params = MaterialParams::default();
    let material = Material::Lit(params.clone());
    let before_stroke = stroke.resolve_in_graph();
    let before_material = material.resolve_in_graph();
    let width = stroke.width.clone();
    std::thread::spawn(move || {
        width.set(Size::Pixels(9.into()));
        params.roughness.set(0.12);
    })
    .join()
    .unwrap();
    assert_eq!(stroke.resolve_in_graph(), before_stroke);
    assert_eq!(material.resolve_in_graph(), before_material);
    assert_ne!(stroke.snapshot(), before_stroke);
    graph.import();
    assert_eq!(stroke.resolve_in_graph(), stroke.snapshot());
    assert_eq!(material.resolve_in_graph(), material.snapshot());
}

#[test]
fn struct_publications_remain_coherent_during_concurrent_reads() {
    #[derive(Clone, Default)]
    struct Versioned {
        generation: usize,
        checksum: usize,
    }
    impl crate::Interpolatable for Versioned {}
    let value = Property::new(Versioned::default());
    let start = Arc::new(Barrier::new(2));
    let worker_start = start.clone();
    let worker_value = value.clone();
    let worker = thread::spawn(move || {
        worker_start.wait();
        for generation in 1..=2000 {
            worker_value.set(Versioned {
                generation,
                checksum: generation * 37,
            });
        }
    });
    start.wait();
    for _ in 0..2000 {
        value.read(|snapshot| assert_eq!(snapshot.checksum, snapshot.generation * 37));
    }
    worker.join().unwrap();
    assert_eq!(value.get().generation, 2000);
}
