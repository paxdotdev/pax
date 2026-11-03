#![cfg(not(feature = "designtime"))]

use pax_engine::api::pax_value::{ImplToFromPaxAny, PaxAny, ToFromPaxAny};
use pax_engine::api::*;
use pax_runtime::*;
use pax_std::{
    core::group::{Group, GroupInstance},
    forms::{
        button::{Button, ButtonInstance},
        textbox::{Textbox, TextboxInstance},
    },
};
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
struct State {
    calls: Vec<String>,
    text: Property<String>,
    origins: Vec<u32>,
    triggers: Vec<SubmitTrigger>,
    stop_submit: bool,
    cancel_submit: bool,
    stop_click: bool,
    cancel_click: bool,
    custom_submit: bool,
}
impl ImplToFromPaxAny for State {}

fn args<T: ToFromPaxAny + 'static>(value: T) -> InstantiationArgs {
    let properties = Rc::new(RefCell::new(value.to_pax_any()));
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Default,
        prototypical_properties: PropertiesInit::Factory(Box::new(move |_, node| {
            node.is_none().then(|| properties.clone())
        })),
        properties_scope: PropertiesScopeInit::None,
        handler_registry: None,
        children: None,
        component_template: None,
        component_settings: None,
        component_camera: None,
        template_node_identifier: None,
        template_node_type_id: None,
        template_node_selector_info: None,
        transition_config: Default::default(),
    }
}

fn inner_submit(properties: Rc<RefCell<PaxAny>>, _: &NodeContext, event: Option<PaxAny>) {
    let event = Event::<Submit>::from_pax_any(event.unwrap()).unwrap();
    let mut properties = properties.borrow_mut();
    let state = State::mut_from_pax_any(&mut properties).unwrap();
    let text = state.text.get();
    state.calls.push(format!("inner:{text}"));
    state.origins.push(event.origin);
    state.triggers.push(event.trigger);
    if state.stop_submit {
        event.stop_propagation();
    }
    if state.cancel_submit {
        event.prevent_default();
    }
}

fn same_node_observer(properties: Rc<RefCell<PaxAny>>, _: &NodeContext, _: Option<PaxAny>) {
    State::mut_from_pax_any(&mut properties.borrow_mut())
        .unwrap()
        .calls
        .push("observer".into());
}

fn outer_submit(properties: Rc<RefCell<PaxAny>>, _: &NodeContext, _: Option<PaxAny>) {
    State::mut_from_pax_any(&mut properties.borrow_mut())
        .unwrap()
        .calls
        .push("outer".into());
}

fn inner_click(properties: Rc<RefCell<PaxAny>>, ctx: &NodeContext, event: Option<PaxAny>) {
    let event = Event::<ButtonClick>::from_pax_any(event.unwrap()).unwrap();
    let mut properties = properties.borrow_mut();
    let state = State::mut_from_pax_any(&mut properties).unwrap();
    state.calls.push("click".into());
    if state.stop_click {
        event.stop_propagation();
    }
    if state.cancel_click {
        event.prevent_default();
    }
    if state.custom_submit {
        ctx.submit();
    }
}

fn pointer(properties: Rc<RefCell<PaxAny>>, _: &NodeContext, event: Option<PaxAny>) {
    let event = Event::<Click>::from_pax_any(event.unwrap()).unwrap();
    event.stop_propagation();
    State::mut_from_pax_any(&mut properties.borrow_mut())
        .unwrap()
        .calls
        .push("pointer".into());
}

fn fixture(textbox: bool) -> (PaxEngine, Rc<ExpandedNode>, Rc<ExpandedNode>) {
    let mut engine = PaxEngine::new_empty(
        (320.0, 240.0),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    let graph = engine.runtime_context.property_graph.clone();
    let _graph = graph.enter();
    let state = State::default();
    let leaf: Rc<dyn InstanceNode> = if textbox {
        TextboxInstance::instantiate(args(Textbox {
            text: state.text.clone(),
            ..Textbox::default()
        }))
    } else {
        ButtonInstance::instantiate(args(Button {
            submit: Property::new(true),
            ..Button::default()
        }))
    };
    let mut transparent = args(Group::default());
    transparent.children = Some(RefCell::new(vec![leaf]));
    let mut group = args(Group::default());
    group.children = Some(RefCell::new(vec![GroupInstance::instantiate(transparent)]));
    group.handler_registry = Some(Rc::new(RefCell::new(HandlerRegistry {
        handlers: [
            (
                "submit".into(),
                vec![
                    Handler::new_inline_handler(inner_submit),
                    Handler::new_inline_handler(same_node_observer),
                ],
            ),
            (
                "button_click".into(),
                vec![Handler::new_inline_handler(inner_click)],
            ),
            (
                "click".into(),
                vec![
                    Handler::new_inline_handler(pointer),
                    Handler::new_inline_handler(same_node_observer),
                ],
            ),
            (
                "tap".into(),
                vec![
                    Handler::new_inline_handler(pointer),
                    Handler::new_inline_handler(same_node_observer),
                ],
            ),
        ]
        .into_iter()
        .collect(),
    })));
    let mut root = args(state);
    root.component_template = Some(RefCell::new(vec![GroupInstance::instantiate(group)]));
    root.handler_registry = Some(Rc::new(RefCell::new(HandlerRegistry {
        handlers: ["submit", "click", "tap"]
            .into_iter()
            .map(|key| {
                (
                    key.into(),
                    vec![Handler::new_component_handler(outer_submit)],
                )
            })
            .collect(),
    })));
    let root = engine.mount_root_component(ComponentInstance::instantiate(root));
    engine.activate_application();
    engine.tick();
    let leaf = root.children.get()[0].children.get()[0].children.get()[0].clone();
    (engine, root, leaf)
}

fn calls(root: &ExpandedNode) -> Vec<String> {
    root.with_properties_unwrapped(|s: &mut State| s.calls.clone())
}

#[test]
fn nested_submission_stops_ancestors_but_keeps_same_node_observers() {
    let (engine, root, leaf) = fixture(true);
    root.with_properties_unwrapped(|s: &mut State| s.stop_submit = true);
    assert!(!leaf.emit_submit(SubmitTrigger::Return, &engine.runtime_context));
    assert_eq!(calls(&root), ["inner:", "observer"]);
    root.with_properties_unwrapped(|s: &mut State| assert_eq!(s.origins, [leaf.id.to_u32()]));
}

#[test]
fn default_prevention_is_shared_but_does_not_stop_bubbling() {
    let (engine, root, leaf) = fixture(true);
    root.with_properties_unwrapped(|s: &mut State| s.cancel_submit = true);
    assert!(leaf.emit_submit(SubmitTrigger::Return, &engine.runtime_context));
    assert_eq!(calls(&root), ["inner:", "observer", "outer"]);
}

#[test]
fn textbox_final_value_reaches_binding_before_submission() {
    let (engine, root, leaf) = fixture(true);
    let interrupt =
        pax_message::NativeInterrupt::FormTextboxSubmit(pax_message::FormTextboxSubmitArgs {
            id: leaf.id.to_u32(),
            text: "Until Monday!".into(),
        });
    leaf.instance_node
        .borrow()
        .handle_native_interrupt(&leaf, &interrupt);
    leaf.emit_submit(SubmitTrigger::Return, &engine.runtime_context);
    assert_eq!(calls(&root), ["inner:Until Monday!", "observer", "outer"]);
}

#[test]
fn submit_button_default_is_distinct_from_click_propagation() {
    let (engine, root, leaf) = fixture(false);
    root.with_properties_unwrapped(|s: &mut State| s.stop_click = true);
    leaf.dispatch_button_click(
        Event::new(ButtonClick {}),
        &engine.runtime_context.globals(),
        &engine.runtime_context,
    );
    assert_eq!(calls(&root), ["click", "inner:", "observer", "outer"]);
    root.with_properties_unwrapped(|s: &mut State| {
        assert_eq!(s.triggers, [SubmitTrigger::Button]);
        assert_eq!(s.origins, [leaf.id.to_u32()]);
        s.calls.clear();
        s.cancel_click = true;
    });
    assert!(leaf.dispatch_button_click(
        Event::new(ButtonClick {}),
        &engine.runtime_context.globals(),
        &engine.runtime_context
    ));
    assert_eq!(calls(&root), ["click"]);
}

#[test]
fn ordinary_buttons_do_not_submit_and_custom_controls_can_queue_submission() {
    let (engine, root, leaf) = fixture(false);
    leaf.with_properties_unwrapped(|b: &mut Button| b.submit.set(false));
    leaf.dispatch_button_click(
        Event::new(ButtonClick {}),
        &engine.runtime_context.globals(),
        &engine.runtime_context,
    );
    assert_eq!(calls(&root), ["click"]);
    root.with_properties_unwrapped(|s: &mut State| {
        s.calls.clear();
        s.custom_submit = true;
    });
    leaf.dispatch_button_click(
        Event::new(ButtonClick {}),
        &engine.runtime_context.globals(),
        &engine.runtime_context,
    );
    assert_eq!(
        calls(&root),
        ["click"],
        "queued delivery must not re-enter the borrowed owner"
    );
    engine.runtime_context.flush_custom_events().unwrap();
    assert_eq!(calls(&root), ["click", "inner:", "observer", "outer"]);
    root.with_properties_unwrapped(|s: &mut State| {
        assert_eq!(s.triggers, [SubmitTrigger::Programmatic])
    });
}

#[test]
fn stopped_pointer_events_keep_same_node_handlers_for_mouse_and_touch() {
    let (engine, root, leaf) = fixture(false);
    let event = || {
        Event::new(Click {
            mouse: MouseEventArgs {
                x: 1.0,
                y: 1.0,
                button: MouseButton::Left,
                modifiers: vec![],
            },
        })
    };
    assert!(!leaf.dispatch_click(
        event(),
        &engine.runtime_context.globals(),
        &engine.runtime_context
    ));
    assert!(!leaf.dispatch_tap(
        event(),
        &engine.runtime_context.globals(),
        &engine.runtime_context
    ));
    assert_eq!(calls(&root), ["pointer", "observer", "pointer", "observer"]);
}

#[test]
fn queued_submission_drops_detached_origins() {
    let (mut engine, root, leaf) = fixture(false);
    let properties = root.properties.borrow().clone();
    leaf.get_node_context(&engine.runtime_context).submit();
    engine.unmount();
    engine.runtime_context.flush_custom_events().unwrap();
    assert!(State::ref_from_pax_any(&properties.borrow())
        .unwrap()
        .calls
        .is_empty());
}
