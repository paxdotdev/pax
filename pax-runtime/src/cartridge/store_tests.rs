use super::*;
use crate::api::{NodeContext, Store, StoreError};

struct SelectionStore {
    selected: LocalProperty<f64>,
}
impl Store for SelectionStore {}
struct OtherStore;
impl Store for OtherStore {}

fn provide(props: Rc<RefCell<PaxAny>>, ctx: &NodeContext, _: Option<PaxAny>) {
    let props = props.as_ref().borrow();
    let p = Probe::ref_from_pax_any(&props).unwrap();
    ctx.provide_store(SelectionStore {
        selected: p.value.clone(),
    })
    .unwrap();
}

fn provider_args(template: Vec<Rc<dyn InstanceNode>>) -> InstantiationArgs {
    let mut args = args();
    let mut registry = HandlerRegistry::default();
    registry.handlers.insert(
        "mount".into(),
        vec![Handler::new_component_handler(provide)],
    );
    args.handler_registry = Some(Rc::new(RefCell::new(registry)));
    args.component_template = Some(RefCell::new(template));
    args
}

fn provider(template: Vec<Rc<dyn InstanceNode>>) -> Rc<ComponentInstance> {
    ComponentInstance::instantiate(provider_args(template))
}

fn mount(
    parent: &Rc<ExpandedNode>,
    engine: &PaxEngine,
    template: Rc<dyn InstanceNode>,
) -> Rc<ExpandedNode> {
    parent
        .generate_children(
            [(template, parent.stack.clone())],
            &engine.runtime_context,
            &parent.parent_frame,
            true,
        )
        .remove(0)
}

fn selection(node: &Rc<ExpandedNode>, engine: &PaxEngine) -> LocalProperty<f64> {
    node.get_node_context(&engine.runtime_context)
        .with_store(|s: &mut SelectionStore| s.selected.clone())
        .unwrap()
}

fn slot() -> Rc<dyn InstanceNode> {
    let mut args = args();
    args.properties_scope = PropertiesScopeInit::None;
    args.prototypical_properties = PropertiesInit::Factory(Box::new(|_, node| {
        node.is_none().then(|| {
            Rc::new(RefCell::new(
                Slot {
                    is_remainder: LocalProperty::new(true),
                    ..Default::default()
                }
                .to_pax_any(),
            ))
        })
    }));
    SlotInstance::instantiate(args)
}

fn repeat(
    source: LocalProperty<PaxValue>,
    keyed: bool,
    children: Vec<Rc<dyn InstanceNode>>,
) -> Rc<dyn InstanceNode> {
    let mut args = args();
    args.children = Some(RefCell::new(children));
    args.properties_scope = PropertiesScopeInit::None;
    args.prototypical_properties = PropertiesInit::Factory(Box::new(move |_, node| {
        node.is_none().then(|| {
            Rc::new(RefCell::new(
                RepeatProperties {
                    source_expression: source.clone(),
                    iterator_i_symbol: LocalProperty::new(Some("i".into())),
                    iterator_elem_symbol: LocalProperty::new(Some("item".into())),
                    repeat_key_expression: keyed.then(|| {
                        ExpressionInfo::new(pax_language::parse_pax_expression("item").unwrap())
                    }),
                }
                .to_pax_any(),
            ))
        })
    }));
    RepeatInstance::instantiate(args)
}

#[test]
fn siblings_publish_into_distinct_owners_without_upward_leakage() {
    let (engine, root) = fixture();
    let template = provider(vec![Leaf::instantiate(args())]);
    let siblings = root.generate_children(
        [
            (template.clone() as Rc<dyn InstanceNode>, root.stack.clone()),
            (template, root.stack.clone()),
        ],
        &engine.runtime_context,
        &root.parent_frame,
        true,
    );
    siblings[0].with_properties_unwrapped(|p: &mut Probe| p.value.set(1.0));
    siblings[1].with_properties_unwrapped(|p: &mut Probe| p.value.set(2.0));
    assert!(Rc::ptr_eq(&siblings[0].stack, &siblings[1].stack));
    assert_eq!(
        selection(&siblings[0].children.get()[0], &engine).get(),
        1.0
    );
    assert_eq!(
        selection(&siblings[1].children.get()[0], &engine).get(),
        2.0
    );
    assert!(matches!(
        root.get_node_context(&engine.runtime_context)
            .with_store(|_: &mut SelectionStore| ()),
        Err(StoreError::Missing { .. })
    ));
    siblings[1].clone().recurse_unmount(&engine.runtime_context);
    assert_eq!(selection(&siblings[0], &engine).get(), 1.0);
}

#[test]
fn nearest_provider_shadows_and_unrelated_types_pass_through() {
    let (engine, root) = fixture();
    let outer = mount(
        &root,
        &engine,
        provider(vec![provider(vec![Leaf::instantiate(args())])]),
    );
    let inner = outer.children.get()[0].clone();
    let leaf = inner.children.get()[0].clone();
    outer.with_properties_unwrapped(|p: &mut Probe| p.value.set(1.0));
    inner.with_properties_unwrapped(|p: &mut Probe| p.value.set(2.0));
    outer
        .get_node_context(&engine.runtime_context)
        .provide_store(OtherStore)
        .unwrap();
    assert_eq!(selection(&leaf, &engine).get(), 2.0);
    leaf.get_node_context(&engine.runtime_context)
        .with_store(|_: &mut OtherStore| ())
        .unwrap();
    assert_eq!(selection(&outer, &engine).get(), 1.0);
}

#[test]
fn replacement_preserves_old_explicit_handles_and_borrows_are_checked_per_type() {
    let (engine, root) = fixture();
    let node = mount(&root, &engine, provider(vec![]));
    let ctx = node.get_node_context(&engine.runtime_context);
    let original = selection(&node, &engine);
    ctx.provide_store(SelectionStore {
        selected: LocalProperty::new(99.0),
    })
    .unwrap();
    original.set(12.0);
    assert_eq!(selection(&node, &engine).get(), 99.0);
    ctx.provide_store(OtherStore).unwrap();
    ctx.with_store(|_: &mut SelectionStore| {
        assert!(matches!(
            ctx.with_store(|_: &mut SelectionStore| ()),
            Err(StoreError::BorrowConflict { .. })
        ));
        assert!(matches!(
            ctx.provide_store(SelectionStore {
                selected: LocalProperty::new(0.0)
            }),
            Err(StoreError::BorrowConflict { .. })
        ));
        ctx.with_store(|_: &mut OtherStore| ()).unwrap();
    })
    .unwrap();
}

#[test]
fn final_unmount_drops_owned_state_and_remount_expires_saved_contexts() {
    struct LifetimeStore {
        _token: Rc<()>,
    }
    impl Store for LifetimeStore {}
    let (engine, root) = fixture();
    engine.activate_application();
    let node = mount(&root, &engine, provider(vec![]));
    let old_ctx = node.get_node_context(&engine.runtime_context);
    let old_scope = old_ctx.async_scope().unwrap();
    let old_properties = Rc::downgrade(&node.properties.borrow());
    let old_selected = selection(&node, &engine);
    old_selected.set(42.0);
    let token = Rc::new(());
    let weak = Rc::downgrade(&token);
    old_ctx
        .provide_store(LifetimeStore { _token: token })
        .unwrap();
    node.clone().recurse_unmount(&engine.runtime_context);
    assert!(old_scope.is_closed());
    assert!(weak.upgrade().is_none());
    assert!(old_properties.upgrade().is_none());
    assert_eq!(
        old_ctx.with_store(|_: &mut SelectionStore| ()),
        Err(StoreError::ExpiredScope)
    );
    assert_eq!(
        old_ctx.provide_store(OtherStore),
        Err(StoreError::ExpiredScope)
    );
    let closed_ctx = node.get_node_context(&engine.runtime_context);
    node.recurse_mount(&engine.runtime_context);
    assert!(matches!(
        old_ctx.async_scope(),
        Err(crate::api::AsyncError::Closed)
    ));
    assert!(matches!(
        closed_ctx.async_scope(),
        Err(crate::api::AsyncError::Closed)
    ));
    assert!(node
        .get_node_context(&engine.runtime_context)
        .async_scope()
        .is_ok());
    assert_eq!(selection(&node, &engine).get(), 7.0);
    assert_eq!(
        old_ctx.with_store(|_: &mut SelectionStore| ()),
        Err(StoreError::ExpiredScope)
    );
    assert_eq!(
        closed_ctx.with_store(|_: &mut SelectionStore| ()),
        Err(StoreError::ExpiredScope)
    );
    old_selected.set(88.0);
    assert_eq!(selection(&node, &engine).get(), 7.0);
}

#[test]
fn remount_rebuilds_measurement_effects_against_fresh_provider_state() {
    let (engine, root) = fixture();
    let node = mount(&root, &engine, provider(vec![]));
    let observations = Rc::new(RefCell::new(Vec::new()));
    let bind = |node: &Rc<ExpandedNode>, token: Rc<()>| {
        let ctx = node.get_node_context(&engine.runtime_context);
        let selected = selection(node, &engine);
        let deps = [selected.untyped()];
        let observations = observations.clone();
        crate::bind_content_measurement_effect(
            node,
            &ctx,
            "store measurement",
            crate::ContentMeasurementGeometry::Placed,
            &deps,
            move |_, ctx| {
                let _keep_alive = &token;
                let value = ctx
                    .with_store(|s: &mut SelectionStore| s.selected.get())
                    .unwrap();
                observations.borrow_mut().push(value);
            },
        );
        engine.runtime_context.drain_node_effects();
    };
    let token = Rc::new(());
    let old_effect = Rc::downgrade(&token);
    let old_common = Rc::downgrade(&node.common_properties.borrow());
    bind(&node, token);
    selection(&node, &engine).set(42.0);
    engine.runtime_context.drain_node_effects();
    assert_eq!(observations.as_ref().borrow().last(), Some(&42.0));

    node.clone().recurse_unmount(&engine.runtime_context);
    assert!(old_effect.upgrade().is_none());
    assert!(old_common.upgrade().is_none());
    node.recurse_mount(&engine.runtime_context);
    bind(&node, Rc::new(()));
    assert_eq!(observations.as_ref().borrow().last(), Some(&7.0));
    selection(&node, &engine).set(11.0);
    engine.runtime_context.drain_node_effects();
    assert_eq!(observations.as_ref().borrow().last(), Some(&11.0));
}

#[test]
fn unmount_handlers_can_still_read_the_provider_scope() {
    thread_local! { static READS: Cell<usize> = const { Cell::new(0) }; }
    fn read_on_unmount(_: Rc<RefCell<PaxAny>>, ctx: &NodeContext, _: Option<PaxAny>) {
        ctx.with_store(|_: &mut SelectionStore| READS.with(|n| n.set(n.get() + 1)))
            .unwrap();
    }
    let (engine, root) = fixture();
    READS.with(|n| n.set(0));
    let mut child_args = args();
    let mut registry = HandlerRegistry::default();
    registry.handlers.insert(
        "unmount".into(),
        vec![Handler::new_component_handler(read_on_unmount)],
    );
    child_args.handler_registry = Some(Rc::new(RefCell::new(registry)));
    let mut args = provider_args(vec![Leaf::instantiate(child_args)]);
    args.handler_registry
        .as_mut()
        .unwrap()
        .borrow_mut()
        .handlers
        .insert(
            "unmount".into(),
            vec![Handler::new_component_handler(read_on_unmount)],
        );
    let node = mount(&root, &engine, ComponentInstance::instantiate(args));
    node.recurse_unmount(&engine.runtime_context);
    assert_eq!(READS.with(Cell::get), 2);
}

#[test]
fn projection_uses_receiving_provider_but_keeps_caller_expressions_and_forwarding() {
    let (engine, root) = fixture();
    let caller_value = LocalProperty::new(42.0);
    let env = root.stack.push(HashMap::from([(
        "value".into(),
        Variable::new_from_typed_property(caller_value.clone()),
    )]));
    // Receiver's slot is forwarded through a private provider component.
    let mut wrapper = provider_args(vec![slot()]);
    wrapper.children = Some(RefCell::new(vec![slot()]));
    let mut receiver = provider_args(vec![ComponentInstance::instantiate(wrapper)]);
    receiver.children = Some(RefCell::new(vec![template(&plan(
        vec![setting("value", expression("self.value"))],
        None,
        None,
    ))]));
    let node = root
        .generate_children(
            [(
                ComponentInstance::instantiate(receiver) as Rc<dyn InstanceNode>,
                env,
            )],
            &engine.runtime_context,
            &root.parent_frame,
            true,
        )
        .remove(0);
    engine.runtime_context.drain_node_effects();
    let projected = node.expanded_projected_children.borrow().as_ref().unwrap()[0].clone();
    assert_eq!(values(&projected).0, 42.0);
    let private_wrapper = node.children.get()[0].clone();
    node.with_properties_unwrapped(|p: &mut Probe| p.value.set(11.0));
    private_wrapper.with_properties_unwrapped(|p: &mut Probe| p.value.set(22.0));
    assert_eq!(selection(&projected, &engine).get(), 11.0);
    caller_value.set(43.0);
    assert_eq!(values(&projected).0, 43.0);
    assert_ne!(
        projected.render_parent.borrow().upgrade().unwrap().id,
        node.id
    );
}

#[test]
fn projected_repeat_has_transparent_unmounted_scopes() {
    let (engine, root) = fixture();
    let source = LocalProperty::new(vec![1usize, 2].to_pax_value());
    let mut receiver = provider_args(vec![slot()]);
    receiver.children = Some(RefCell::new(vec![repeat(
        source.clone(),
        true,
        vec![Leaf::instantiate(args())],
    )]));
    let node = mount(&root, &engine, ComponentInstance::instantiate(receiver));
    engine.runtime_context.drain_node_effects();
    let repeated = node.expanded_projected_children.borrow().as_ref().unwrap()[0].clone();
    assert_eq!(repeated.attached.get(), 0);
    for leaf in repeated.children.get() {
        assert_eq!(selection(&leaf, &engine).get(), 7.0);
    }
    source.set(vec![2usize, 1, 3].to_pax_value());
    engine.runtime_context.drain_node_effects();
    for leaf in repeated.children.get() {
        assert_eq!(selection(&leaf, &engine).get(), 7.0);
    }
}

#[test]
fn repeated_providers_follow_keyed_identity_and_unkeyed_positions() {
    for keyed in [true, false] {
        let (engine, root) = fixture();
        let source = LocalProperty::new(vec![1usize, 2].to_pax_value());
        let repeated = mount(
            &root,
            &engine,
            repeat(
                source.clone(),
                keyed,
                vec![provider(vec![]), provider(vec![])],
            ),
        );
        engine.runtime_context.drain_node_effects();
        let before = repeated.children.get();
        for (i, node) in before.iter().enumerate() {
            selection(node, &engine).set(i as f64);
        }
        source.set(vec![2usize, 1].to_pax_value());
        engine.runtime_context.drain_node_effects();
        let after = repeated.children.get();
        let order = if keyed { [2, 3, 0, 1] } else { [0, 1, 2, 3] };
        for (node, old) in after.iter().zip(order) {
            assert_eq!(node.id, before[old].id);
            assert_eq!(selection(node, &engine).get(), old as f64);
        }
        source.set(Vec::<usize>::new().to_pax_value());
        engine.runtime_context.drain_node_effects();
        for node in &after {
            assert_eq!(
                node.get_node_context(&engine.runtime_context)
                    .with_store(|_: &mut SelectionStore| ()),
                Err(StoreError::ExpiredScope)
            );
        }
        source.set(vec![1usize].to_pax_value());
        engine.runtime_context.drain_node_effects();
        for node in repeated.children.get() {
            assert_eq!(selection(&node, &engine).get(), 7.0);
        }
    }
}

#[test]
fn nested_keyed_repeats_keep_each_provider_identity() {
    let (engine, root) = fixture();
    let outer_source = LocalProperty::new(vec![1usize, 2].to_pax_value());
    let inner_source = LocalProperty::new(vec![10usize, 20].to_pax_value());
    let outer = mount(
        &root,
        &engine,
        repeat(
            outer_source.clone(),
            true,
            vec![repeat(inner_source.clone(), true, vec![provider(vec![])])],
        ),
    );
    engine.runtime_context.drain_node_effects();
    let rows = outer.children.get();
    for (row, inner) in rows.iter().enumerate() {
        for (column, node) in inner.children.get().iter().enumerate() {
            selection(node, &engine).set((10 * row + column) as f64);
        }
    }
    outer_source.set(vec![2usize, 1].to_pax_value());
    inner_source.set(vec![20usize, 10].to_pax_value());
    engine.runtime_context.drain_node_effects();
    let values: Vec<Vec<f64>> = outer
        .children
        .get()
        .iter()
        .map(|inner| {
            inner
                .children
                .get()
                .iter()
                .map(|node| selection(node, &engine).get())
                .collect()
        })
        .collect();
    assert_eq!(values, vec![vec![11.0, 10.0], vec![1.0, 0.0]]);
}

#[test]
fn render_reparent_keeps_provider_ancestry_and_cannot_reconnect_after_ancestor_remount() {
    let (engine, root) = fixture();
    let siblings = root.generate_children(
        [
            (
                provider(vec![Leaf::instantiate(args())]) as Rc<dyn InstanceNode>,
                root.stack.clone(),
            ),
            (provider(vec![]) as Rc<dyn InstanceNode>, root.stack.clone()),
        ],
        &engine.runtime_context,
        &root.parent_frame,
        true,
    );
    let first = &siblings[0];
    let second = &siblings[1];
    selection(first, &engine).set(1.0);
    selection(second, &engine).set(2.0);
    let child = first.children.get()[0].clone();
    second.attach_children(
        vec![child.clone()],
        &engine.runtime_context,
        &second.parent_frame,
    );
    assert_eq!(selection(&child, &engine).get(), 1.0);
    first.clone().recurse_unmount(&engine.runtime_context);
    assert_eq!(
        child
            .get_node_context(&engine.runtime_context)
            .with_store(|_: &mut SelectionStore| ()),
        Err(StoreError::ExpiredScope)
    );
    first.recurse_mount(&engine.runtime_context);
    assert_eq!(
        child
            .get_node_context(&engine.runtime_context)
            .with_store(|_: &mut SelectionStore| ()),
        Err(StoreError::ExpiredScope)
    );
}

#[test]
fn reload_retains_stable_handles_and_remounts_changed_binding_aliases() {
    let (engine, root) = fixture();
    let a = LocalProperty::new(10.0);
    let b = LocalProperty::new(20.0);
    let env = root.stack.push(HashMap::from([
        ("a".into(), Variable::new_from_typed_property(a.clone())),
        ("b".into(), Variable::new_from_typed_property(b.clone())),
    ]));
    let make = |name: &str| {
        let plan = plan(
            vec![setting(
                "value",
                ValueDefinition::DoubleBinding(PaxIdentifier::new(name)),
            )],
            None,
            None,
        );
        let mut args = provider_args(vec![Leaf::instantiate(args())]);
        args.prototypical_properties = PropertiesInit::Template {
            descriptor: &ERASED,
            plan,
        };
        ComponentInstance::instantiate(args)
    };
    let node = root
        .generate_children(
            [(make("a") as Rc<dyn InstanceNode>, env)],
            &engine.runtime_context,
            &root.parent_frame,
            true,
        )
        .remove(0);
    let ctx = node.get_node_context(&engine.runtime_context);
    let old_child = node.children.get()[0].clone();
    node.recreate_with_new_data(make("a"), &engine.runtime_context);
    assert!(ctx.with_store(|_: &mut SelectionStore| ()).is_ok());
    assert_eq!(node.children.get()[0].id, old_child.id);
    node.recreate_with_new_data(make("b"), &engine.runtime_context);
    assert_eq!(
        ctx.with_store(|_: &mut SelectionStore| ()),
        Err(StoreError::ExpiredScope)
    );
    selection(&node.children.get()[0], &engine).set(33.0);
    assert_eq!(a.get(), 10.0);
    assert_eq!(b.get(), 33.0);
    assert_ne!(node.children.get()[0].id, old_child.id);
}

#[test]
fn exit_retention_preserves_store_until_rescue_or_final_unmount() {
    let (engine, root) = fixture();
    let source = LocalProperty::new(vec![1usize].to_pax_value());
    let mut args = provider_args(vec![]);
    args.transition_config = pax_manifest::cartridge_generation::ComponentTransitionConfig {
        has_exit: true,
        exit_frame_count: 30,
        timeout_ms: 5_000,
        ..Default::default()
    };
    let repeated = mount(
        &root,
        &engine,
        repeat(
            source.clone(),
            true,
            vec![ComponentInstance::instantiate(args)],
        ),
    );
    engine.runtime_context.drain_node_effects();
    let node = repeated.children.get()[0].clone();
    let ctx = node.get_node_context(&engine.runtime_context);
    selection(&node, &engine).set(42.0);
    source.set(Vec::<usize>::new().to_pax_value());
    engine.runtime_context.drain_node_effects();
    assert_eq!(selection(&node, &engine).get(), 42.0);
    source.set(vec![1usize].to_pax_value());
    engine.runtime_context.drain_node_effects();
    assert_eq!(repeated.children.get()[0].id, node.id);
    assert_eq!(selection(&node, &engine).get(), 42.0);
    source.set(Vec::<usize>::new().to_pax_value());
    engine.runtime_context.drain_node_effects();
    engine.runtime_context.globals().elapsed_frames.set(100);
    engine.runtime_context.globals().elapsed_millis.set(6_000);
    engine.runtime_context.drain_node_effects();
    root.recurse_update(&engine.runtime_context);
    assert_eq!(
        ctx.with_store(|_: &mut SelectionStore| ()),
        Err(StoreError::ExpiredScope)
    );
}

#[test]
fn subtree_recreation_discards_providers_without_touching_siblings() {
    let (engine, root) = fixture();
    let template = provider(vec![provider(vec![])]);
    let siblings = root.generate_children(
        [
            (template.clone() as Rc<dyn InstanceNode>, root.stack.clone()),
            (template.clone(), root.stack.clone()),
        ],
        &engine.runtime_context,
        &root.parent_frame,
        true,
    );
    let first = &siblings[0];
    let second = &siblings[1];
    selection(first, &engine).set(10.0);
    selection(second, &engine).set(20.0);
    let nested = first.children.get()[0].clone();
    let old_ctx = nested.get_node_context(&engine.runtime_context);
    selection(&nested, &engine).set(30.0);
    first.fully_recreate_with_new_data(template, &engine.runtime_context);
    assert_eq!(
        old_ctx.with_store(|_: &mut SelectionStore| ()),
        Err(StoreError::ExpiredScope)
    );
    assert_eq!(selection(first, &engine).get(), 7.0);
    assert_eq!(selection(&first.children.get()[0], &engine).get(), 7.0);
    assert_eq!(selection(second, &engine).get(), 20.0);
}
