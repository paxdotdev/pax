use super::*;
use pax_runtime_api::pax_value::ToFromPaxAny;
use pax_runtime_api::LocalProperty;

struct Fixture {
    engine: PaxEngine,
    root: Rc<ExpandedNode>,
    group: Rc<ExpandedNode>,
    conditional: Rc<ExpandedNode>,
    selected: LocalProperty<bool>,
    constructed: Rc<RefCell<Vec<(usize, u32)>>>,
}

impl Fixture {
    fn new(indexed: bool) -> Self {
        let mut engine = new_engine();
        let ctx = Rc::clone(&engine.runtime_context);
        ctx.configure_visible_groups(true);
        let _graph = ctx.property_graph.enter();
        let selected = LocalProperty::new(true);
        let constructed = Rc::new(RefCell::new(Vec::new()));
        let children = (0..7)
            .map(|i| {
                let mut args = output_args();
                let log = Rc::clone(&constructed);
                args.prototypical_properties =
                    crate::PropertiesInit::Factory(Box::new(move |_, node| {
                        if let Some(n) = node {
                            // A mount-time settings rebind can call this
                            // factory again for the same expanded node.
                            let id = n.id.to_u32();
                            let mut nodes = log.borrow_mut();
                            if !nodes.iter().any(|(_, previous)| *previous == id) {
                                nodes.push((i, id));
                            }
                            None
                        } else {
                            Some(Rc::new(RefCell::new(PaxAny::Builtin(Default::default()))))
                        }
                    }));
                args.handler_registry = component(vec![]).base().handler_registry.clone();
                args.component_template = Some(RefCell::new(vec![]));
                ComponentInstance::instantiate(args) as Rc<dyn InstanceNode>
            })
            .collect();
        let mut args = output_args();
        let predicate = selected.clone();
        args.children = Some(RefCell::new(children));
        args.prototypical_properties = crate::PropertiesInit::Factory(Box::new(move |_, node| {
            node.is_none().then(|| {
                Rc::new(RefCell::new(
                    crate::ConditionalProperties {
                        boolean_expression: predicate.clone(),
                        conditional_branches: vec![predicate.clone(), LocalProperty::new(true)],
                    }
                    .to_pax_any(),
                ))
            })
        }));
        let conditional = crate::ConditionalInstance::instantiate_with_branch_child_ranges(
            args,
            vec![0..5, 5..7],
        );
        let root = engine.mount_root_component(component(vec![
            box_at(0., vec![]),
            box_at(
                1200.,
                vec![
                    Output::<true>::instantiate(output_args()),
                    Output::<false>::instantiate(output_args()),
                    conditional,
                ],
            ),
        ]));
        assert!(reach_checkpoint(&mut engine).pending);
        assert!(engine.begin_background_mounting());
        if indexed {
            engine.enable_indexed_mounts(0.);
            engine.observe_mount_hosts(0., [0., 0., 320., 240.], &Default::default(), &[], 250.);
        }
        engine.prioritize_background_owners();
        engine.publish_presentation();
        let group = root.children.get()[1].clone();
        for _ in 0..250 {
            assert!(engine.service_background_mounts(0., || 0.));
            if let Some(id) = ctx.startup.borrow().preparing() {
                let conditional = ctx.get_expanded_node_by_eid(id).unwrap();
                assert!(constructed.borrow().is_empty());
                return Self {
                    engine,
                    root,
                    group,
                    conditional,
                    selected,
                    constructed,
                };
            }
            engine.tick();
        }
        panic!("conditional never entered the parked group's admission");
    }

    fn one_child(&mut self, expected: usize) {
        let service_counts = |engine: &PaxEngine| {
            let startup = engine.runtime_context.startup.borrow();
            startup.live_mounts.as_ref().map(|mounts| {
                (
                    mounts.counts.conditional_units,
                    mounts.counts.slices_all_maintenance,
                )
            })
        };
        let before = service_counts(&self.engine);
        assert!(self.engine.service_background_mounts(0., || 0.));
        if let Some((units, maintenance)) = before {
            assert_eq!(service_counts(&self.engine), Some((units + 1, maintenance)));
        }
        assert_eq!(self.constructed.borrow().len(), expected);
        self.engine.tick();
        assert_eq!(
            self.constructed.borrow().len(),
            expected,
            "foreground settlement forced the unfinished list"
        );
        assert!(!self
            .engine
            .runtime_context
            .ready_visible_groups()
            .iter()
            .any(|row| row.0 == self.group.id.to_u32()));
    }

    fn finish(&mut self) -> Vec<usize> {
        for _ in 0..500 {
            if !self.engine.service_background_mounts(0., || 0.) {
                break;
            }
            self.engine.tick();
        }
        assert!(!self.engine.startup_pending());
        assert!(self
            .conditional
            .active_children
            .borrow()
            .iter()
            .all(|n| n.attached.get() == 1));
        assert!(
            !self.conditional.active_children.borrow().is_empty(),
            "settlement lost the completed branch"
        );
        assert!(self
            .engine
            .runtime_context
            .ready_visible_groups()
            .iter()
            .any(|row| row.0 == self.group.id.to_u32()));
        let constructed = self.constructed.borrow();
        CALLS.with_borrow(|calls| {
            calls
                .iter()
                .filter(|(phase, _)| *phase == "mount")
                .filter_map(|(_, id)| {
                    constructed
                        .iter()
                        .find(|(_, child)| child == id)
                        .map(|(i, _)| *i)
                })
                .collect()
        })
    }
}

#[test]
fn parked_conditional_yields_per_child_and_preserves_mount_order() {
    for indexed in [false, true] {
        let mut fixture = Fixture::new(indexed);
        for n in 1..5 {
            fixture.one_child(n);
        }
        assert!(fixture.conditional.active_children.borrow().is_empty());
        assert_eq!(fixture.finish(), vec![0, 1, 2, 3, 4]);
    }
}

#[test]
fn a_two_millisecond_grant_checks_the_clock_between_conditional_children() {
    for indexed in [false, true] {
        let mut fixture = Fixture::new(indexed);
        let mut now = 0.0;
        assert!(fixture.engine.service_background_mounts(2.0, || {
            let observed = now;
            now += 0.75;
            observed
        }));
        assert_eq!(fixture.constructed.borrow().len(), 3);
        assert!(fixture.conditional.active_children.borrow().is_empty());
        assert_eq!(fixture.finish(), vec![0, 1, 2, 3, 4]);
    }
}

#[test]
fn synchronous_read_finishes_the_current_parked_selection() {
    for change_branch in [false, true] {
        let mut fixture = Fixture::new(true);
        fixture.one_child(1);
        fixture.one_child(2);
        let ctx = Rc::clone(&fixture.engine.runtime_context);
        let _graph = ctx.property_graph.enter();
        fixture
            .group
            .get_common_properties()
            .borrow()
            .width
            .set(Some(pax_runtime_api::Size::Pixels(76.into())));
        if change_branch {
            fixture.selected.set(false);
        }
        let selected = fixture.conditional.children.get();
        assert_eq!(selected.len(), if change_branch { 2 } else { 5 });
        assert_eq!(
            fixture.constructed.borrow().len(),
            if change_branch { 4 } else { 5 }
        );
        assert!(selected.iter().all(|child| child.attached.get() == 0));
        assert!(selected
            .iter()
            .all(|child| child.layout_bounds.get().0 == 76.0));
        assert!(selected
            .iter()
            .zip(fixture.conditional.children.get())
            .all(|(a, b)| Rc::ptr_eq(a, &b)));
        assert_eq!(
            fixture.finish(),
            if change_branch {
                vec![5, 6]
            } else {
                vec![0, 1, 2, 3, 4]
            }
        );
    }
}

#[test]
fn changing_a_condition_between_grants_only_mounts_the_latest_selection() {
    let mut fixture = Fixture::new(true);
    fixture.one_child(1);
    fixture.one_child(2);
    let ctx = Rc::clone(&fixture.engine.runtime_context);
    let _graph = ctx.property_graph.enter();
    fixture.selected.set(false);
    fixture.engine.tick();
    assert_eq!(fixture.constructed.borrow().len(), 2);
    fixture.one_child(3);
    assert_eq!(fixture.finish(), vec![5, 6]);
    assert_eq!(
        fixture
            .constructed
            .borrow()
            .iter()
            .map(|(index, _)| *index)
            .collect::<Vec<_>>(),
        vec![0, 1, 5, 6]
    );
}

#[test]
fn removing_a_partial_parked_conditional_cancels_preparation() {
    let mut fixture = Fixture::new(true);
    fixture.one_child(1);
    let ctx = Rc::clone(&fixture.engine.runtime_context);
    let _graph = ctx.property_graph.enter();
    let remaining = vec![fixture.root.children.get()[0].clone()];
    fixture
        .root
        .attach_children(remaining, &ctx, &fixture.root.parent_frame);
    assert_eq!(fixture.conditional.attached.get(), 0);
    for _ in 0..100 {
        if !fixture.engine.service_background_mounts(0., || 0.) {
            break;
        }
        fixture.engine.tick();
    }
    assert!(!fixture.engine.startup_pending());
    assert_eq!(fixture.constructed.borrow().len(), 1);
    let id = fixture.constructed.borrow()[0].1;
    CALLS.with_borrow(|calls| assert!(!calls.contains(&("mount", id))));
}
