use super::*;
use crate::api::NodeContext;
use crate::{ConditionalInstance, ConditionalProperties, Handler, HandlerRegistry};
use pax_runtime_api::{Click, Event, MouseButton, MouseEventArgs};

#[derive(Default)]
struct Calls {
    inside: usize,
    on_click: Option<Box<dyn FnOnce()>>,
    outside: usize,
}
impl pax_runtime_api::pax_value::ImplToFromPaxAny for Calls {}

fn observe(properties: Rc<RefCell<PaxAny>>, _: &NodeContext, event: Option<PaxAny>) {
    let event = Event::<Click>::from_pax_any(event.unwrap()).unwrap();
    assert_eq!(event.mouse.x, 12.0);
    assert!(!event.cancelled() && !event.propagation_stopped());
    event.stop_propagation();
    event.prevent_default();
    Calls::mut_from_pax_any(&mut properties.borrow_mut())
        .unwrap()
        .outside += 1;
}

fn activate(properties: Rc<RefCell<PaxAny>>, _: &NodeContext, event: Option<PaxAny>) {
    Event::<Click>::from_pax_any(event.unwrap())
        .unwrap()
        .stop_propagation();
    let action = {
        let mut properties = properties.borrow_mut();
        let calls = Calls::mut_from_pax_any(&mut properties).unwrap();
        calls.inside += 1;
        calls.on_click.take()
    };
    if let Some(action) = action {
        action();
    }
}

fn listener(children: Vec<Rc<dyn InstanceNode>>) -> Rc<dyn InstanceNode> {
    let mut args = component_args(Some(children));
    args.prototypical_properties = crate::PropertiesInit::Factory(Box::new(|_, node| {
        node.is_none()
            .then(|| Rc::new(RefCell::new(Calls::default().to_pax_any())))
    }));
    args.handler_registry = Some(Rc::new(RefCell::new(HandlerRegistry {
        handlers: [
            (
                "click_outside".into(),
                vec![Handler::new_component_handler(observe)],
            ),
            (
                "click".into(),
                vec![Handler::new_component_handler(activate)],
            ),
        ]
        .into_iter()
        .collect(),
    })));
    ComponentInstance::instantiate(args)
}

fn click() -> Event<Click> {
    Event::new(Click {
        mouse: MouseEventArgs {
            x: 12.0,
            y: 24.0,
            button: MouseButton::Left,
            modifiers: vec![],
        },
    })
}

fn counts(node: &ExpandedNode) -> (usize, usize) {
    node.with_properties_unwrapped(|s: &mut Calls| (s.inside, s.outside))
}

#[test]
fn captured_activation_notifies_only_displayed_outside_listeners() {
    let items = LocalProperty::new(source(&["a", "b"]));
    let repeat = RepeatInstance::instantiate(repeat_args(
        items.clone(),
        vec![listener(vec![listener(vec![])])],
    ));
    let mut engine = crate::test_support::empty_engine(
        (100., 100.),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    let root =
        engine.mount_root_component(ComponentInstance::instantiate(component_args(Some(vec![
            repeat,
        ]))));
    engine.tick();
    let ctx = engine.runtime_context.clone();
    let front = crate::properties::input_capture::InputPublisher::default()
        .capture(&ctx)
        .unwrap();
    items.set(source(&["a", "b", "c"]));
    engine.tick();
    let rows = root.children.get()[0].children.get();
    let child = rows[0].children.get()[0].clone();
    let _scope = front.enter(&ctx, &Default::default()).unwrap();
    child.dispatch_click(click(), &ctx.globals(), &ctx);
    child.dispatch_tap(click(), &ctx.globals(), &ctx);
    assert_eq!(counts(&child), (2, 0));
    assert_eq!(counts(&rows[0]), (0, 0));
    assert_eq!(counts(&rows[1]), (0, 2));
    assert_eq!(counts(&rows[2]), (0, 0));
    rows[2].dispatch_click(click(), &ctx.globals(), &ctx);
    assert_eq!(counts(&rows[1]), (0, 2));
}

#[test]
fn click_outside_tracks_conditional_and_repeated_subtree_lifetimes() {
    for keyed in [false, true] {
        let items = LocalProperty::new(source(&["a", "b", "c"]));
        let shown = LocalProperty::new(true);
        let row = listener(vec![listener(vec![])]);
        let repeat = RepeatInstance::instantiate(repeat_args_with_key(
            items.clone(),
            vec![row.clone()],
            keyed,
        ));
        let mut conditional = component_args(None);
        let condition = shown.clone();
        conditional.prototypical_properties =
            crate::PropertiesInit::Factory(Box::new(move |_, node| {
                node.is_none().then(|| {
                    Rc::new(RefCell::new(
                        ConditionalProperties {
                            boolean_expression: condition.clone(),
                            conditional_branches: vec![],
                        }
                        .to_pax_any(),
                    ))
                })
            }));
        conditional.children = Some(RefCell::new(vec![repeat]));
        let ctx = Rc::new(crate::test_support::runtime_context(test_globals()));
        let root = ExpandedNode::initialize_root(
            ComponentInstance::instantiate(component_args(Some(vec![
                ConditionalInstance::instantiate(conditional),
            ]))),
            &ctx,
        );
        root.recurse_update(&ctx);
        let repeat = root.children.get()[0].children.get()[0].clone();
        let rows = repeat.children.get();
        let child = rows[0].children.get()[0].clone();
        assert!(!child.dispatch_click(click(), &ctx.globals(), &ctx));
        assert_eq!(counts(&child), (1, 0));
        assert_eq!(counts(&rows[0]), (0, 0)); // A stopped click still counts as inside its parent.
        assert_eq!(counts(&rows[1]), (0, 1));
        assert_eq!(counts(&rows[2]), (0, 1));
        let before = ctx.outside_clicks.borrow().snapshot(&root).unwrap();
        let removed = if keyed { &rows[1] } else { &rows[2] };
        let removed_state = removed.properties.borrow().clone();
        items.set(source(&["c", "a"]));
        root.recurse_update(&ctx);
        let live = repeat.children.get();
        assert_eq!(removed.attached.get(), 0);
        if keyed {
            assert_eq!(live[0].id, rows[2].id);
        }
        // Replace a handler registry in place between capture and delivery.
        let previous = counts(&live[0]);
        live[0].recreate_with_new_data(row.clone(), &ctx);
        ctx.notify_click_outside(before, &click().args);
        assert_eq!(counts(&live[0]), previous);
        assert_eq!(
            Calls::ref_from_pax_any(&removed_state.borrow())
                .unwrap()
                .outside,
            1
        );
        assert!(!root.dispatch_tap(click(), &ctx.globals(), &ctx));
        assert_eq!(counts(&live[0]).1, previous.1 + 1);

        let before = ctx.outside_clicks.borrow().snapshot(&root).unwrap();
        shown.set(false);
        root.recurse_update(&ctx);
        assert!(ctx.outside_clicks.borrow().snapshot(&root).is_none());
        shown.set(true);
        root.recurse_update(&ctx);
        ctx.notify_click_outside(before, &click().args);
        let reopened = root.children.get()[0].children.get()[0].children.get();
        assert_eq!(counts(&reopened[0]), (0, 0));
        assert!(!root.dispatch_click(click(), &ctx.globals(), &ctx));
        assert_eq!(counts(&reopened[0]), (0, 1));
    }
}

#[test]
fn click_outside_skips_retained_row_and_resumes_once_when_key_returns() {
    let items = LocalProperty::new(source(&["a", "b"]));
    let mut animated_row = component_args(Some(vec![listener(vec![])]));
    animated_row.transition_config = ComponentTransitionConfig {
        has_exit: true,
        exit_frame_count: 30,
        timeout_ms: 5_000,
        ..Default::default()
    };
    let repeat = RepeatInstance::instantiate(repeat_args(
        items.clone(),
        vec![ComponentInstance::instantiate(animated_row)],
    ));
    let ctx = Rc::new(crate::test_support::runtime_context(test_globals()));
    let root = ExpandedNode::initialize_root(
        ComponentInstance::instantiate(component_args(Some(vec![repeat]))),
        &ctx,
    );
    root.recurse_update(&ctx);
    let repeat = root.children.get()[0].clone();
    let row_a = repeat.children.get()[0].clone();
    let listener_a = row_a.children.get()[0].clone();
    let before = ctx.outside_clicks.borrow().snapshot(&root).unwrap();
    items.set(source(&["b"]));
    root.recurse_update(&ctx);
    assert_eq!(row_a.attached.get(), 1);
    assert!(repeat
        .exiting_children
        .borrow()
        .iter()
        .any(|r| r.id == row_a.id));
    root.dispatch_tap(click(), &ctx.globals(), &ctx);
    assert_eq!(counts(&listener_a), (0, 0));
    items.set(source(&["b", "a"]));
    root.recurse_update(&ctx);
    assert_eq!(repeat.children.get()[1].id, row_a.id);
    ctx.notify_click_outside(before, &click().args);
    assert_eq!(counts(&listener_a), (0, 0));
    root.dispatch_click(click(), &ctx.globals(), &ctx);
    assert_eq!(counts(&listener_a), (0, 1));
}

#[test]
fn click_outside_freezes_recipients_before_the_target_handler_changes_the_loop() {
    let items = LocalProperty::new(source(&["keep", "remove"]));
    let repeat = RepeatInstance::instantiate(repeat_args(items.clone(), vec![listener(vec![])]));
    let ctx = Rc::new(crate::test_support::runtime_context(test_globals()));
    let root = ExpandedNode::initialize_root(
        ComponentInstance::instantiate(component_args(Some(vec![listener(vec![]), repeat]))),
        &ctx,
    );
    root.recurse_update(&ctx);
    let button = root.children.get()[0].clone();
    let repeat = root.children.get()[1].clone();
    let removed = repeat.children.get()[1].properties.borrow().clone();
    button.with_properties_unwrapped(|s: &mut Calls| {
        let (root, ctx) = (root.clone(), ctx.clone());
        s.on_click = Some(Box::new(move || {
            items.set(source(&["new", "keep"]));
            root.recurse_update(&ctx);
        }));
    });
    assert!(!button.dispatch_click(click(), &ctx.globals(), &ctx));
    let rows = repeat.children.get();
    assert_eq!(counts(&rows[0]), (0, 0));
    assert_eq!(counts(&rows[1]), (0, 1));
    assert_eq!(
        Calls::ref_from_pax_any(&removed.borrow()).unwrap().outside,
        0
    );
}
