#![cfg(not(feature = "designtime"))]

use pax_runtime::api::*;
use pax_runtime::*;
use pax_runtime_api::pax_value::{ImplToFromPaxAny, PaxAny, ToFromPaxAny};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

type CallLog = Rc<RefCell<Vec<Call>>>;

type Call = (&'static str, ExpandedNodeIdentifier);

#[derive(Default)]
struct Owner {
    calls: CallLog,
}
impl ImplToFromPaxAny for Owner {}

#[derive(Default)]
struct Child {
    calls: CallLog,
}
impl ImplToFromPaxAny for Child {}

fn args<T: ToFromPaxAny + Default>() -> InstantiationArgs {
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Default,
        prototypical_properties: PropertiesInit::Factory(Box::new(|_, node| {
            node.is_none()
                .then(|| Rc::new(RefCell::new(T::default().to_pax_any())))
        })),
        handler_registry: None,
        children: None,
        component_template: None,
        component_settings: None,
        template_node_identifier: None,
        template_node_type_id: None,
        template_node_selector_info: None,
        transition_config: Default::default(),
        properties_scope: PropertiesScopeInit::None,
    }
}

macro_rules! handler {
    ($name:ident, $state:ty, $phase:literal) => {
        fn $name(properties: Rc<RefCell<PaxAny>>, ctx: &NodeContext, event: Option<PaxAny>) {
            assert!(event.is_none());
            let target = ctx.expanded_node.upgrade().unwrap();
            assert!(Rc::ptr_eq(&ctx.expression_stack, &target.stack));
            assert_eq!(
                ctx.bounds_self.get(),
                target.transform_and_bounds.get().bounds
            );
            assert_eq!(
                ctx.node_transform_and_bounds.bounds,
                target.transform_and_bounds.get().bounds
            );
            let mut properties = properties.borrow_mut();
            let state = <$state>::mut_from_pax_any(&mut properties).expect(concat!(
                "handler receives ",
                stringify!($state),
                " state"
            ));
            state.calls.borrow_mut().push(($phase, target.id));
        }
    };
}

handler!(owner_mount, Owner, "mount");
handler!(owner_tick, Owner, "tick");
handler!(owner_pre_render, Owner, "pre_render");
handler!(owner_unmount, Owner, "unmount");
handler!(child_mount, Child, "mount");
handler!(child_tick, Child, "tick");
handler!(child_pre_render, Child, "pre_render");
handler!(child_unmount, Child, "unmount");

fn child_resize_mount(properties: Rc<RefCell<PaxAny>>, ctx: &NodeContext, event: Option<PaxAny>) {
    child_mount(properties, ctx, event);
    // The following inline handler must see a fresh target context.
    ctx.expanded_node
        .upgrade()
        .unwrap()
        .get_common_properties()
        .borrow_mut()
        .width
        .set(Some(Size::Pixels(240.into())));
}

const OWNER_HANDLERS: &[HandlerDescriptor] = &[
    HandlerDescriptor::new("mount", owner_mount),
    HandlerDescriptor::new("tick", owner_tick),
    HandlerDescriptor::new("pre_render", owner_pre_render),
    HandlerDescriptor::new("unmount", owner_unmount),
];
const CHILD_HANDLERS: &[HandlerDescriptor] = &[
    HandlerDescriptor::new("mount", child_resize_mount),
    HandlerDescriptor::new("tick", child_tick),
    HandlerDescriptor::new("pre_render", child_pre_render),
    HandlerDescriptor::new("unmount", child_unmount),
];
const PHASES: [&str; 4] = ["mount", "tick", "pre_render", "unmount"];

fn child_template() -> Rc<ComponentInstance> {
    let mut child = args::<Child>();
    let registry = build_component_handler_registry(
        CHILD_HANDLERS,
        PHASES
            .iter()
            .map(|name| (name.to_string(), vec![name.to_string()]))
            .collect(),
    );
    child.handler_registry = Some(add_inline_handlers_from_descriptors(
        OWNER_HANDLERS,
        PHASES
            .iter()
            .map(|name| (name.to_string(), name.to_string()))
            .collect(),
        registry,
    ));
    ComponentInstance::instantiate(child)
}

fn owner_template() -> Rc<ComponentInstance> {
    let mut owner = args::<Owner>();
    owner.component_template = Some(RefCell::new(vec![child_template()]));
    ComponentInstance::instantiate(owner)
}

fn engine() -> PaxEngine {
    PaxEngine::new_empty(
        (320.0, 240.0),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    )
}

fn owner_calls(node: &Rc<ExpandedNode>) -> CallLog {
    node.with_properties_unwrapped(|owner: &mut Owner| owner.calls.clone())
}

fn child_calls(node: &Rc<ExpandedNode>) -> CallLog {
    node.with_properties_unwrapped(|child: &mut Child| child.calls.clone())
}

#[test]
fn inline_mount_uses_authored_owner_and_child_context() {
    let mut child = args::<Child>();
    child.handler_registry = Some(Rc::new(RefCell::new(HandlerRegistry {
        handlers: [(
            "mount".to_owned(),
            vec![Handler::new_inline_handler(owner_mount)],
        )]
        .into_iter()
        .collect(),
    })));
    let mut owner = args::<Owner>();
    owner.component_template = Some(RefCell::new(vec![ComponentInstance::instantiate(child)]));
    let mut engine = engine();
    let root = engine.mount_root_component(ComponentInstance::instantiate(owner));
    let child = root.children.get()[0].clone();
    assert_eq!(*owner_calls(&root).borrow(), vec![("mount", child.id)]);
    engine.unmount();
}

#[test]
fn all_lifecycle_phases_keep_inline_and_component_state_separate_across_instances() {
    let owner = owner_template();
    let mut root_args = args::<()>();
    root_args.component_template = Some(RefCell::new(vec![owner.clone(), owner]));
    let mut engine = engine();
    let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
    let owners = root.children.get();
    let children: Vec<_> = owners
        .iter()
        .map(|owner| owner.children.get()[0].clone())
        .collect();
    let owner_logs: Vec<_> = owners.iter().map(owner_calls).collect();
    let child_logs: Vec<_> = children.iter().map(child_calls).collect();
    engine.tick();
    // Suspension suppresses tick/pre-render, but must not suppress final unmount.
    children[0].suspended.set(true);
    engine.tick();
    engine.unmount();
    for (index, child) in children.iter().enumerate() {
        let mut expected = vec![
            ("mount", child.id),
            ("tick", child.id),
            ("pre_render", child.id),
        ];
        if index == 1 {
            expected.extend([("tick", child.id), ("pre_render", child.id)]);
        }
        expected.push(("unmount", child.id));
        assert_eq!(*owner_logs[index].borrow(), expected);
        assert_eq!(*child_logs[index].borrow(), expected);
    }
    engine.tick();
    assert_eq!(owner_logs[0].borrow().len(), 4);
    assert_eq!(owner_logs[1].borrow().len(), 6);
}

#[test]
fn root_component_settings_use_root_state_and_context() {
    let mut owner = args::<Owner>();
    owner.handler_registry = Some(build_component_handler_registry(
        OWNER_HANDLERS,
        PHASES
            .iter()
            .map(|name| (name.to_string(), vec![name.to_string()]))
            .collect(),
    ));
    let mut engine = engine();
    let root = engine.mount_root_component(ComponentInstance::instantiate(owner));
    let calls = owner_calls(&root);
    engine.tick();
    engine.unmount();
    assert_eq!(*calls.borrow(), PHASES.map(|name| (name, root.id)));
}

#[test]
fn projected_child_handlers_belong_to_author_not_slot_host() {
    let mut host = args::<Child>();
    host.children = Some(RefCell::new(vec![child_template()]));
    host.component_template = Some(RefCell::new(vec![
        SlotInstance::instantiate(args::<Slot>()),
    ]));
    let mut owner = args::<Owner>();
    owner.component_template = Some(RefCell::new(vec![ComponentInstance::instantiate(host)]));
    let mut engine = engine();
    let root = engine.mount_root_component(ComponentInstance::instantiate(owner));
    let host = root.children.get()[0].clone();
    let projected = host.expanded_projected_children.borrow().as_ref().unwrap()[0].clone();
    engine.tick();
    assert_eq!(
        projected.containing_component.upgrade().unwrap().id,
        root.id
    );
    assert_ne!(
        projected.render_parent.borrow().upgrade().unwrap().id,
        root.id
    );
    let owner_log = owner_calls(&root);
    let child_log = child_calls(&projected);
    assert!(child_calls(&host).borrow().is_empty());
    engine.unmount();
    assert_eq!(*owner_log.borrow(), PHASES.map(|name| (name, projected.id)));
    assert_eq!(*child_log.borrow(), PHASES.map(|name| (name, projected.id)));
}

#[test]
fn reload_uses_current_owner_state_and_unmounts_old_children_first() {
    let mut engine = engine();
    let root = engine.mount_root_component(owner_template());
    let first_child = root.children.get()[0].clone();
    let original_log = owner_calls(&root);
    let first_child_log = child_calls(&first_child);
    // Partial reload retains state; full reload replaces it after old unmount handlers.
    first_child.recreate_with_new_data(child_template(), &engine.runtime_context);
    engine.tick();
    root.fully_recreate_with_new_data(owner_template(), &engine.runtime_context);
    let second_child = root.children.get()[0].clone();
    assert_ne!(first_child.id, second_child.id);
    assert_eq!(
        *original_log.borrow(),
        PHASES.map(|name| (name, first_child.id)),
    );
    assert_eq!(
        *first_child_log.borrow(),
        PHASES.map(|name| (name, first_child.id))
    );
    let current_log = owner_calls(&root);
    let second_child_log = child_calls(&second_child);
    engine.tick();
    engine.unmount();
    assert_eq!(
        *current_log.borrow(),
        PHASES.map(|name| (name, second_child.id))
    );
    assert_eq!(
        *second_child_log.borrow(),
        PHASES.map(|name| (name, second_child.id))
    );
}

#[test]
fn render_reparent_and_remount_do_not_change_authored_owner() {
    let mut engine = engine();
    let root = engine.mount_root_component(owner_template());
    let child = root.children.get()[0].clone();
    let mut containers = root.create_children_detached(
        [(
            ComponentInstance::instantiate(args::<Child>()) as Rc<dyn InstanceNode>,
            root.stack.clone(),
        )],
        &engine.runtime_context,
        &Rc::downgrade(&root),
    );
    let container = containers.remove(0);
    let children = root.attach_children(
        vec![child.clone(), container.clone()],
        &engine.runtime_context,
        &root.parent_frame,
    );
    root.children.set(children);
    let children = container.attach_children(
        vec![child.clone()],
        &engine.runtime_context,
        &container.parent_frame,
    );
    container.children.set(children);
    assert_eq!(
        child.render_parent.borrow().upgrade().unwrap().id,
        container.id
    );
    assert_eq!(child.containing_component.upgrade().unwrap().id, root.id);
    engine.tick();
    let first_child_log = child_calls(&child);
    let children =
        container.attach_children(vec![], &engine.runtime_context, &container.parent_frame);
    container.children.set(children);
    child.suspended.set(true);
    // Explicit remount also calls mount even while suspended.
    child.recurse_mount(&engine.runtime_context);
    let remounted_log = child_calls(&child);
    child.recurse_mount(&engine.runtime_context);
    child.clone().recurse_unmount(&engine.runtime_context);
    child.clone().recurse_unmount(&engine.runtime_context);
    let expected = vec![
        ("mount", child.id),
        ("tick", child.id),
        ("pre_render", child.id),
        ("unmount", child.id),
        ("mount", child.id),
        ("unmount", child.id),
    ];
    assert_eq!(*owner_calls(&root).borrow(), expected);
    assert_eq!(*first_child_log.borrow(), expected[..4]);
    assert_eq!(*remounted_log.borrow(), expected[4..]);
    engine.unmount();
}

#[test]
fn mounted_mask_source_handlers_keep_authored_owner_and_source_context() {
    let mut engine = engine();
    let root = engine.mount_root_component(ComponentInstance::instantiate(args::<Owner>()));
    let sources = root.create_children_detached(
        [(child_template() as Rc<dyn InstanceNode>, root.stack.clone())],
        &engine.runtime_context,
        &Rc::downgrade(&root),
    );
    let sources =
        root.attach_sidecar_children(sources, &engine.runtime_context, &root.parent_frame);
    let source = &sources[0];
    // Mask sources have a logical mount lifetime without native presentation.
    source.mount_as_render_source(&root, &engine.runtime_context);
    source.recurse_control_flow_expansion(&engine.runtime_context);
    let owner_log = owner_calls(&root);
    let child_log = child_calls(source);
    assert_eq!(source.containing_component.upgrade().unwrap().id, root.id);
    assert!(source.is_render_source());
    engine.tick();
    // MaskInstance tears down its mounted source when its owner unmounts.
    source.clone().recurse_unmount(&engine.runtime_context);
    engine.unmount();
    assert_eq!(*owner_log.borrow(), PHASES.map(|name| (name, source.id)));
    assert_eq!(*child_log.borrow(), PHASES.map(|name| (name, source.id)));
}

#[test]
fn inline_subscription_lifetime_belongs_to_target_and_final_unmount_releases_owner() {
    thread_local! {
        static SIGNAL: Property<usize> = Property::new(0);
        static CALLS: Rc<Cell<usize>> = Rc::new(Cell::new(0));
        static UNMOUNTS: Cell<usize> = const { Cell::new(0) };
        static SCOPE: RefCell<Option<AsyncScope>> = const { RefCell::new(None) };
    }
    fn unmount(properties: Rc<RefCell<PaxAny>>, ctx: &NodeContext, event: Option<PaxAny>) {
        owner_unmount(properties, ctx, event);
        SCOPE.with(|scope| assert!(scope.borrow().as_ref().unwrap().is_closed()));
        UNMOUNTS.with(|calls| calls.set(calls.get() + 1));
    }
    fn subscribe(properties: Rc<RefCell<PaxAny>>, ctx: &NodeContext, event: Option<PaxAny>) {
        owner_mount(properties, ctx, event);
        SCOPE.with(|scope| *scope.borrow_mut() = Some(ctx.async_scope().unwrap()));
        SIGNAL.with(|signal| {
            CALLS.with(|calls| {
                let calls = calls.clone();
                ctx.subscribe(&[signal.untyped()], move || calls.set(calls.get() + 1));
            })
        });
    }
    let mut target = args::<Child>();
    target.handler_registry = Some(Rc::new(RefCell::new(HandlerRegistry {
        handlers: [
            (
                "mount".to_owned(),
                vec![Handler::new_inline_handler(subscribe)],
            ),
            (
                "unmount".to_owned(),
                vec![Handler::new_inline_handler(unmount)],
            ),
        ]
        .into_iter()
        .collect(),
    })));
    let mut owner = args::<Owner>();
    owner.component_template = Some(RefCell::new(vec![ComponentInstance::instantiate(target)]));
    let mut engine = engine();
    engine.activate_application();
    let root = engine.mount_root_component(ComponentInstance::instantiate(owner));
    let target = root.children.get()[0].clone();
    engine.runtime_context.drain_node_effects();
    SIGNAL.with(|signal| signal.set(1));
    engine.runtime_context.drain_node_effects();
    let before = CALLS.with(|calls| calls.get());
    assert!(before > 0);
    let weak_owner = Rc::downgrade(&root);
    let weak_state = Rc::downgrade(&root.properties.borrow());
    drop(root);
    engine.unmount();
    // No retained owner Rc is needed for the child's final unmount callback.
    assert_eq!(UNMOUNTS.with(|calls| calls.get()), 1);
    assert!(weak_owner.upgrade().is_none());
    assert!(weak_state.upgrade().is_none());
    assert!(target.subscriptions.borrow().is_empty());
    SIGNAL.with(|signal| signal.set(2));
    engine.runtime_context.drain_node_effects();
    assert_eq!(CALLS.with(|calls| calls.get()), before);
}

#[test]
fn inline_lifecycle_bindings_live_until_authored_owner_unmounts() {
    use pax_runtime_api::properties::PropertyBinding;

    #[derive(Default)]
    struct BindingOwner {
        source: Property<u32>,
        bound: Option<Property<u32>>,
        token: Rc<()>,
    }
    impl ImplToFromPaxAny for BindingOwner {}

    fn bind(properties: Rc<RefCell<PaxAny>>, _: &NodeContext, _: Option<PaxAny>) {
        let mut properties = properties.borrow_mut();
        let owner = BindingOwner::mut_from_pax_any(&mut properties).unwrap();
        let source = owner.source.clone();
        let dependencies = [source.untyped()];
        let token = owner.token.clone();
        owner.bound = Some(Property::from_local(LocalProperty::computed(
            move || {
                let _ = &token;
                source.get() + 1
            },
            &dependencies,
        )));
    }

    for phase in PHASES {
        let mut target = args::<Child>();
        target.handler_registry = Some(Rc::new(RefCell::new(HandlerRegistry {
            handlers: [(phase.to_owned(), vec![Handler::new_inline_handler(bind)])]
                .into_iter()
                .collect(),
        })));
        let mut owner = args::<BindingOwner>();
        owner.component_template = Some(RefCell::new(vec![ComponentInstance::instantiate(target)]));
        let mut engine = engine();
        let root = engine.mount_root_component(ComponentInstance::instantiate(owner));
        let target = root.children.get()[0].clone();
        engine.tick();
        target.clone().recurse_unmount(&engine.runtime_context);
        let (source, bound, token) = root.with_properties_unwrapped(|owner: &mut BindingOwner| {
            (
                owner.source.clone(),
                owner.bound.clone().unwrap(),
                Rc::downgrade(&owner.token),
            )
        });
        source.set(7);
        engine.runtime_context.drain_node_effects();
        assert_eq!(
            bound.get(),
            8,
            "{phase} binding must survive target unmount"
        );
        engine.unmount();
        engine.runtime_context.property_graph.collect();
        assert!(
            token.upgrade().is_none(),
            "{phase} binding must release its closure"
        );
        source.set(9);
        engine.runtime_context.drain_node_effects();
        assert_eq!(
            bound.get(),
            8,
            "{phase} binding must stop with authored owner"
        );
    }
}
