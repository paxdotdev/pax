use pax_chassis_common::native_interrupt::NativeInterruptDispatcher;
use pax_message::{
    FormCheckboxToggleArgs, FormSliderChangeArgs, FormTextboxChangeArgs, FormTextboxInputArgs,
    FormTextboxSubmitArgs, KeyDownInterruptArgs, KeyPressInterruptArgs, KeyUpInterruptArgs,
    ModifierKeyMessage, NativeInterrupt, RenderSurfaceUpdateArgs, ScrollerPositionInterruptArgs,
    TouchCancelInterruptArgs, TouchEndInterruptArgs, TouchMessage, TouchMoveInterruptArgs,
    TouchStartInterruptArgs, WheelInterruptArgs,
};
use pax_runtime::api::pax_value::{ImplToFromPaxAny, PaxAny, ToFromPaxAny};
use pax_runtime::api::*;
use pax_runtime::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug, Default, PartialEq)]
struct ControlState {
    text: String,
    checked: bool,
    value: f64,
    scroll: (f64, f64),
}
impl ImplToFromPaxAny for ControlState {}

#[derive(Debug, PartialEq)]
enum Observation {
    Input(String, ControlState),
    Change(String, ControlState),
    Submit(u32, SubmitTrigger, ControlState),
    Checkbox(bool, ControlState),
    Slider(f64, ControlState),
    Scroll((f64, f64), ControlState),
    Key(&'static str, String, bool, Vec<&'static str>),
    Wheel(&'static str, (f64, f64), (f64, f64), Vec<&'static str>),
}

#[derive(Default)]
struct Owner {
    calls: Vec<Observation>,
}
impl ImplToFromPaxAny for Owner {}

// This primitive supplies only the native-value update seam. Event ownership,
// handler invocation and cancellation are exercised through the real engine.
struct NativeControl {
    base: BaseInstance,
}

impl InstanceNode for NativeControl {
    fn instantiate(args: InstantiationArgs) -> Rc<Self> {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    layer: Layer::Native,
                    invisible_to_slot: false,
                    invisible_to_raycasting: false,
                    is_component: false,
                    is_slot: false,
                },
            ),
        })
    }

    fn base(&self) -> &BaseInstance {
        &self.base
    }

    fn resolve_debug(
        &self,
        formatter: &mut std::fmt::Formatter,
        _: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        formatter.debug_struct("NativeControl").finish()
    }

    fn handle_native_interrupt(&self, node: &Rc<ExpandedNode>, interrupt: &NativeInterrupt) {
        node.with_properties_unwrapped(|state: &mut ControlState| match interrupt {
            NativeInterrupt::FormTextboxInput(args) => state.text = args.text.clone(),
            NativeInterrupt::FormTextboxSubmit(args) => state.text = args.text.clone(),
            NativeInterrupt::FormCheckboxToggle(args) => state.checked = args.state,
            NativeInterrupt::FormSliderChange(args) => state.value = args.value,
            NativeInterrupt::ScrollerPosition(args) => {
                state.scroll = (args.scroll_x, args.scroll_y)
            }
            _ => {}
        });
    }
}

fn arguments<T: ToFromPaxAny + Default>() -> InstantiationArgs {
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
        component_camera: None,
        template_node_identifier: None,
        template_node_type_id: None,
        template_node_selector_info: None,
        transition_config: Default::default(),
        properties_scope: PropertiesScopeInit::None,
    }
}

fn control_state(context: &NodeContext) -> ControlState {
    context
        .expanded_node
        .upgrade()
        .unwrap()
        .with_properties_unwrapped(|state: &mut ControlState| state.clone())
}

fn record(owner: Rc<RefCell<PaxAny>>, observation: Observation) {
    Owner::mut_from_pax_any(&mut owner.borrow_mut())
        .unwrap()
        .calls
        .push(observation);
}

fn input(owner: Rc<RefCell<PaxAny>>, context: &NodeContext, event: Option<PaxAny>) {
    let event = Event::<TextboxInput>::from_pax_any(event.unwrap()).unwrap();
    record(
        owner,
        Observation::Input(event.text.clone(), control_state(context)),
    );
}

fn change(owner: Rc<RefCell<PaxAny>>, context: &NodeContext, event: Option<PaxAny>) {
    let event = Event::<TextboxChange>::from_pax_any(event.unwrap()).unwrap();
    record(
        owner,
        Observation::Change(event.text.clone(), control_state(context)),
    );
}

fn submit(owner: Rc<RefCell<PaxAny>>, context: &NodeContext, event: Option<PaxAny>) {
    let event = Event::<Submit>::from_pax_any(event.unwrap()).unwrap();
    record(
        owner,
        Observation::Submit(event.origin, event.trigger, control_state(context)),
    );
}

fn checkbox(owner: Rc<RefCell<PaxAny>>, context: &NodeContext, event: Option<PaxAny>) {
    let event = Event::<CheckboxChange>::from_pax_any(event.unwrap()).unwrap();
    record(
        owner,
        Observation::Checkbox(event.checked, control_state(context)),
    );
}

fn slider(owner: Rc<RefCell<PaxAny>>, context: &NodeContext, event: Option<PaxAny>) {
    let event = Event::<SliderChange>::from_pax_any(event.unwrap()).unwrap();
    record(
        owner,
        Observation::Slider(event.value, control_state(context)),
    );
}

fn scroll(owner: Rc<RefCell<PaxAny>>, context: &NodeContext, event: Option<PaxAny>) {
    let event = Event::<Scroll>::from_pax_any(event.unwrap()).unwrap();
    record(
        owner,
        Observation::Scroll((event.delta_x, event.delta_y), control_state(context)),
    );
}

fn modifier_names(modifiers: &[ModifierKey]) -> Vec<&'static str> {
    modifiers
        .iter()
        .map(|modifier| match modifier {
            ModifierKey::Shift => "shift",
            ModifierKey::Control => "control",
            ModifierKey::Alt => "alt",
            ModifierKey::Command => "command",
        })
        .collect()
}

fn record_keyboard(owner: Rc<RefCell<PaxAny>>, phase: &'static str, args: &KeyboardEventArgs) {
    record(
        owner,
        Observation::Key(
            phase,
            args.key.clone(),
            args.is_repeat,
            modifier_names(&args.modifiers),
        ),
    );
}

fn wheel(owner: Rc<RefCell<PaxAny>>, _: &NodeContext, event: Option<PaxAny>) {
    let event = Event::<Wheel>::from_pax_any(event.unwrap()).unwrap();
    record(
        owner,
        Observation::Wheel(
            "target",
            (event.x, event.y),
            (event.delta_x, event.delta_y),
            modifier_names(&event.modifiers),
        ),
    );
}

fn ancestor_wheel(owner: Rc<RefCell<PaxAny>>, _: &NodeContext, event: Option<PaxAny>) {
    let event = Event::<Wheel>::from_pax_any(event.unwrap()).unwrap();
    record(
        owner,
        Observation::Wheel(
            "ancestor",
            (event.x, event.y),
            (event.delta_x, event.delta_y),
            modifier_names(&event.modifiers),
        ),
    );
    if event
        .modifiers
        .iter()
        .any(|modifier| matches!(modifier, ModifierKey::Control))
    {
        event.prevent_default();
    }
}

macro_rules! keyboard_handler {
    ($name:ident, $event:ty, $phase:literal) => {
        fn $name(owner: Rc<RefCell<PaxAny>>, _: &NodeContext, event: Option<PaxAny>) {
            let event = Event::<$event>::from_pax_any(event.unwrap()).unwrap();
            record_keyboard(owner, $phase, &event.keyboard);
            if event.keyboard.key == "Escape" {
                event.prevent_default();
            }
        }
    };
}
keyboard_handler!(key_down, KeyDown, "down");
keyboard_handler!(key_up, KeyUp, "up");
keyboard_handler!(key_press, KeyPress, "press");

fn empty_engine() -> PaxEngine {
    #[cfg(feature = "designtime")]
    {
        let manager = pax_designtime::DesigntimeManager::new_offline(pax_manifest::PaxManifest {
            components: Default::default(),
            main_component_type_id: Default::default(),
            type_table: Default::default(),
            assets_dirs: Vec::new(),
            engine_import_path: "pax_engine".into(),
        });
        PaxEngine::new_empty_with_designtime(
            (320.0, 240.0),
            Rc::new(RefCell::new(manager)),
            Platform::Native,
            OS::Linux,
            Box::new(|| 0),
            Default::default(),
        )
    }
    #[cfg(not(feature = "designtime"))]
    {
        PaxEngine::new_empty(
            (320.0, 240.0),
            Platform::Native,
            OS::Windows,
            Box::new(|| 0),
            Default::default(),
        )
    }
}

struct Fixture {
    engine: PaxEngine,
    owner: Rc<ExpandedNode>,
    control: Rc<ExpandedNode>,
    dispatcher: NativeInterruptDispatcher,
}

impl Fixture {
    fn new() -> Self {
        type HandlerFn = fn(Rc<RefCell<PaxAny>>, &NodeContext, Option<PaxAny>);
        let handlers: [(&str, HandlerFn); 10] = [
            ("textbox_input", input),
            ("textbox_change", change),
            ("submit", submit),
            ("checkbox_change", checkbox),
            ("slider_change", slider),
            ("scroll", scroll),
            ("key_down", key_down),
            ("key_up", key_up),
            ("key_press", key_press),
            ("wheel", wheel),
        ];
        let mut child = arguments::<ControlState>();
        child.handler_registry = Some(Rc::new(RefCell::new(HandlerRegistry {
            handlers: handlers
                .into_iter()
                .map(|(name, function)| {
                    (name.to_owned(), vec![Handler::new_inline_handler(function)])
                })
                .collect(),
        })));
        let mut parent = arguments::<Owner>();
        parent.handler_registry = Some(Rc::new(RefCell::new(HandlerRegistry {
            handlers: std::collections::HashMap::from([(
                "wheel".into(),
                vec![Handler::new_component_handler(ancestor_wheel)],
            )]),
        })));
        parent.component_template = Some(RefCell::new(vec![NativeControl::instantiate(child)]));
        let mut engine = empty_engine();
        let owner = engine.mount_root_component(ComponentInstance::instantiate(parent));
        let control = owner.children.get()[0].clone();
        Self {
            engine,
            owner,
            control,
            dispatcher: Default::default(),
        }
    }

    fn id(&self) -> u32 {
        self.control.id.to_u32()
    }

    fn calls(&self) -> Vec<Observation> {
        self.owner
            .with_properties_unwrapped(|owner: &mut Owner| std::mem::take(&mut owner.calls))
    }

    fn dispatch(&self, interrupt: NativeInterrupt) -> Option<bool> {
        self.dispatcher.dispatch(&self.engine, &interrupt)
    }

    fn position(&self, logical: (f64, f64), presentation: Option<(f64, f64)>) {
        assert_eq!(
            self.dispatch(NativeInterrupt::ScrollerPosition(
                ScrollerPositionInterruptArgs {
                    id: self.id(),
                    scroll_x: logical.0,
                    scroll_y: logical.1,
                    presentation_scroll_x: presentation.map(|point| point.0),
                    presentation_scroll_y: presentation.map(|point| point.1),
                }
            )),
            Some(false)
        );
    }
}

impl std::ops::Drop for Fixture {
    fn drop(&mut self) {
        self.engine.unmount();
    }
}

#[test]
fn native_values_are_updated_before_inline_handlers_run() {
    let fixture = Fixture::new();
    let id = fixture.id();
    assert_eq!(
        fixture.dispatch(NativeInterrupt::FormTextboxInput(FormTextboxInputArgs {
            id,
            text: "composed text".into(),
        })),
        Some(false)
    );
    assert_eq!(
        fixture.dispatch(NativeInterrupt::FormTextboxChange(FormTextboxChangeArgs {
            id,
            text: "composed text".into(),
        })),
        Some(false)
    );
    assert_eq!(
        fixture.dispatch(NativeInterrupt::FormCheckboxToggle(
            FormCheckboxToggleArgs { id, state: true }
        )),
        Some(false)
    );
    assert_eq!(
        fixture.dispatch(NativeInterrupt::FormSliderChange(FormSliderChangeArgs {
            id,
            value: 0.75,
        })),
        Some(false)
    );
    let text = ControlState {
        text: "composed text".into(),
        ..Default::default()
    };
    let checked = ControlState {
        checked: true,
        ..text.clone()
    };
    assert_eq!(
        fixture.calls(),
        vec![
            Observation::Input("composed text".into(), text.clone()),
            Observation::Change("composed text".into(), text),
            Observation::Checkbox(true, checked.clone()),
            Observation::Slider(
                0.75,
                ControlState {
                    value: 0.75,
                    ..checked
                }
            ),
        ]
    );
}

#[test]
fn native_textbox_submission_updates_value_before_emitting_return() {
    let fixture = Fixture::new();
    let id = fixture.id();
    assert_eq!(
        fixture.dispatch(NativeInterrupt::FormTextboxSubmit(FormTextboxSubmitArgs {
            id,
            text: "final composed value".into(),
        })),
        Some(false)
    );
    assert_eq!(
        fixture.calls(),
        vec![Observation::Submit(
            id,
            SubmitTrigger::Return,
            ControlState {
                text: "final composed value".into(),
                ..Default::default()
            },
        )]
    );
}

#[test]
fn keyboard_delivery_preserves_metadata_and_default_prevention() {
    let fixture = Fixture::new();
    for key in ["Escape", "a"] {
        let modifiers = || {
            vec![
                ModifierKeyMessage::Shift,
                ModifierKeyMessage::Control,
                ModifierKeyMessage::Alt,
                ModifierKeyMessage::Command,
            ]
        };
        for interrupt in [
            NativeInterrupt::KeyDown(KeyDownInterruptArgs {
                key: key.into(),
                modifiers: modifiers(),
                is_repeat: true,
            }),
            NativeInterrupt::KeyUp(KeyUpInterruptArgs {
                key: key.into(),
                modifiers: modifiers(),
                is_repeat: true,
            }),
            NativeInterrupt::KeyPress(KeyPressInterruptArgs {
                key: key.into(),
                modifiers: modifiers(),
                is_repeat: true,
            }),
        ] {
            assert_eq!(fixture.dispatch(interrupt), Some(key == "Escape"));
        }
        assert_eq!(
            fixture.calls(),
            ["down", "up", "press"].map(|phase| Observation::Key(
                phase,
                key.into(),
                true,
                vec!["shift", "control", "alt", "command"]
            ))
        );
    }
}

#[test]
fn scrolling_reports_presentation_deltas_and_keeps_authoritative_values() {
    let fixture = Fixture::new();
    fixture.engine.runtime_context.set_scroller_surface_state(
        fixture.id(),
        ScrollerSurfaceState {
            viewport_width: 320.0,
            viewport_height: 240.0,
            content_width: 800.0,
            content_height: 900.0,
            clip_content: true,
            ..Default::default()
        },
    );
    fixture.position((10.0, 20.0), Some((15.0, 30.0)));
    assert!(
        fixture.calls().is_empty(),
        "the first position establishes a baseline"
    );
    fixture.position((40.0, 50.0), Some((20.0, 27.0)));
    fixture.position((41.0, 51.0), Some((20.0, 27.0)));
    assert_eq!(
        fixture.calls(),
        vec![Observation::Scroll(
            (5.0, -3.0),
            ControlState {
                scroll: (40.0, 50.0),
                ..Default::default()
            }
        )]
    );
    let state = fixture
        .engine
        .runtime_context
        .get_scroller_surface_state(fixture.id())
        .unwrap();
    assert_eq!((state.scroll_x, state.scroll_y), (41.0, 51.0));
    assert_eq!(
        (state.presentation_scroll_x, state.presentation_scroll_y),
        (20.0, 27.0)
    );
    assert_eq!((state.content_width, state.content_height), (800.0, 900.0));
    fixture.position((23.0, 35.0), None);
    assert_eq!(
        fixture.calls(),
        vec![Observation::Scroll(
            (3.0, 8.0),
            ControlState {
                scroll: (23.0, 35.0),
                ..Default::default()
            }
        )]
    );
}

#[test]
fn wheel_input_bubbles_once_and_returns_ancestor_default_prevention() {
    let fixture = Fixture::new();
    fixture
        .control
        .transform_and_bounds
        .replace_with(LocalProperty::new(TransformAndBounds::default()));
    fixture.position((0.0, 0.0), None);
    for (modifiers, names, prevented) in [
        (vec![], vec![], false),
        (
            vec![
                ModifierKeyMessage::Shift,
                ModifierKeyMessage::Control,
                ModifierKeyMessage::Alt,
                ModifierKeyMessage::Command,
            ],
            vec!["shift", "control", "alt", "command"],
            true,
        ),
    ] {
        assert_eq!(
            fixture.dispatch(NativeInterrupt::Wheel(WheelInterruptArgs {
                x: 12.0,
                y: 14.0,
                delta_x: -0.25,
                delta_y: 1.75,
                modifiers,
            })),
            Some(prevented)
        );
        assert_eq!(
            fixture.calls(),
            vec![
                Observation::Wheel("target", (12.0, 14.0), (-0.25, 1.75), names.clone()),
                Observation::Wheel("ancestor", (12.0, 14.0), (-0.25, 1.75), names),
            ],
            "wheel delivery must not manufacture a scroll notification"
        );
    }
    fixture.position((2.0, 3.0), None);
    assert_eq!(
        fixture.calls(),
        vec![Observation::Scroll(
            (2.0, 3.0),
            ControlState {
                scroll: (2.0, 3.0),
                ..Default::default()
            }
        )]
    );
}

#[test]
fn wheel_input_outside_geometry_preserves_the_engine_root_fallback() {
    let fixture = Fixture::new();
    fixture
        .control
        .transform_and_bounds
        .replace_with(LocalProperty::new(TransformAndBounds::default()));
    let interrupt = serde_json::from_str(
        r#"{"Wheel":{"x":-100,"y":-100,
        "delta_x":120,"delta_y":0,"modifiers":["Control"]}}"#,
    )
    .unwrap();
    assert_eq!(fixture.dispatch(interrupt), Some(true));
    assert_eq!(
        fixture.calls(),
        vec![Observation::Wheel(
            "ancestor",
            (-100.0, -100.0),
            (120.0, 0.0),
            vec!["control"],
        )]
    );
}

#[test]
fn apple_wheel_compatibility_keeps_synthetic_scroll_outside_native_scrollers() {
    let mut fixture = Fixture::new();
    fixture.dispatcher = NativeInterruptDispatcher::with_synthetic_wheel_scroll();
    fixture
        .control
        .transform_and_bounds
        .replace_with(LocalProperty::new(TransformAndBounds::default()));
    assert_eq!(
        fixture.dispatch(NativeInterrupt::Wheel(WheelInterruptArgs {
            x: 12.0,
            y: 14.0,
            delta_x: -0.25,
            delta_y: 1.75,
            modifiers: vec![],
        })),
        Some(false)
    );
    assert_eq!(
        fixture.calls(),
        vec![
            Observation::Wheel("target", (12.0, 14.0), (-0.25, 1.75), vec![]),
            Observation::Wheel("ancestor", (12.0, 14.0), (-0.25, 1.75), vec![]),
            Observation::Scroll((-0.25, 1.75), ControlState::default()),
        ]
    );
}

#[test]
fn wheel_transport_without_a_scene_leaves_native_default_available() {
    let engine = empty_engine();
    let dispatcher = NativeInterruptDispatcher::default();
    let interrupt = serde_json::from_str(
        r#"{"Wheel":{"x":12,"y":14,"delta_x":120,"delta_y":0,"modifiers":[]}}"#,
    )
    .unwrap();
    assert_eq!(dispatcher.dispatch(&engine, &interrupt), Some(false));
}

#[test]
fn deleted_scroller_id_starts_from_a_fresh_position() {
    let fixture = Fixture::new();
    fixture.position((0.0, 100.0), None);
    fixture.position((0.0, 120.0), None);
    assert_eq!(fixture.calls().len(), 1);
    fixture.dispatcher.forget_scroller(fixture.id());
    fixture.position((0.0, 5.0), None);
    assert!(fixture.calls().is_empty());
    fixture.position((0.0, 9.0), None);
    assert_eq!(
        fixture.calls(),
        vec![Observation::Scroll(
            (0.0, 4.0),
            ControlState {
                scroll: (0.0, 9.0),
                ..Default::default()
            }
        )]
    );
}

#[test]
fn dispatcher_sessions_do_not_share_scroll_history() {
    let fixture = Fixture::new();
    fixture.position((0.0, 100.0), None);
    let fresh = NativeInterruptDispatcher::default();
    assert_eq!(
        fresh.dispatch(
            &fixture.engine,
            &NativeInterrupt::ScrollerPosition(ScrollerPositionInterruptArgs {
                id: fixture.id(),
                scroll_x: 0.0,
                scroll_y: 7.0,
                presentation_scroll_x: None,
                presentation_scroll_y: None,
            })
        ),
        Some(false)
    );
    assert!(fixture.calls().is_empty());
    fixture.position((0.0, 103.0), None);
    assert_eq!(
        fixture.calls(),
        vec![Observation::Scroll(
            (0.0, 3.0),
            ControlState {
                scroll: (0.0, 103.0),
                ..Default::default()
            }
        )]
    );
}

#[test]
fn synthetic_touch_gestures_are_isolated_and_stop_on_end_or_cancel() {
    let send = |fixture: &Fixture, phase: &str, identifier, delta: (f64, f64)| {
        let touches = vec![TouchMessage {
            x: 12.0,
            y: 14.0,
            identifier,
            delta_x: delta.0,
            delta_y: delta.1,
        }];
        let interrupt = match phase {
            "start" => NativeInterrupt::TouchStart(TouchStartInterruptArgs { touches }),
            "move" => NativeInterrupt::TouchMove(TouchMoveInterruptArgs { touches }),
            "end" => NativeInterrupt::TouchEnd(TouchEndInterruptArgs { touches }),
            "cancel" => NativeInterrupt::TouchCancel(TouchCancelInterruptArgs { touches }),
            _ => unreachable!(),
        };
        assert_eq!(fixture.dispatch(interrupt), Some(false));
    };

    for finish in ["end", "cancel"] {
        let first = Fixture::new();
        let second = Fixture::new();
        for fixture in [&first, &second] {
            // Supply a fixed hit rectangle without depending on a layout tick.
            fixture
                .control
                .transform_and_bounds
                .replace_with(LocalProperty::new(TransformAndBounds::default()));
        }
        send(&first, "start", 41, (0.0, 0.0));
        send(&second, "start", 42, (0.0, 0.0));
        for (fixture, identifier) in [(&first, 41), (&second, 42)] {
            let captured = fixture
                .engine
                .runtime_context
                .captured_touch_target(identifier)
                .expect("touch starts on the native primitive");
            assert!(Rc::ptr_eq(&captured, &fixture.control));
        }

        send(&first, "move", 41, (2.0, 3.0));
        assert_eq!(
            first.calls(),
            vec![Observation::Scroll((2.0, 3.0), ControlState::default())],
            "starting another session must not replace this gesture"
        );
        send(&second, "move", 42, (5.0, 7.0));
        assert_eq!(
            second.calls(),
            vec![Observation::Scroll((5.0, 7.0), ControlState::default())]
        );
        send(&second, finish, 42, (0.0, 0.0));
        assert!(second
            .engine
            .runtime_context
            .captured_touch_target(42)
            .is_none());
        send(&second, "move", 42, (11.0, 13.0));
        assert!(
            second.calls().is_empty(),
            "{finish} must stop synthetic scrolling"
        );

        send(&first, "move", 41, (17.0, 19.0));
        assert_eq!(
            first.calls(),
            vec![Observation::Scroll((17.0, 19.0), ControlState::default())],
            "{finish} in another session must not clear this gesture"
        );
        send(&first, finish, 41, (0.0, 0.0));
        assert!(first
            .engine
            .runtime_context
            .captured_touch_target(41)
            .is_none());
        send(&first, "move", 41, (23.0, 29.0));
        assert!(
            first.calls().is_empty(),
            "{finish} must clear the original gesture"
        );
    }
}

#[test]
fn presentation_work_remains_with_the_platform_adapter() {
    let fixture = Fixture::new();
    assert_eq!(
        fixture.dispatch(NativeInterrupt::RenderSurfaceUpdate(
            RenderSurfaceUpdateArgs { layer_id: Some(0) }
        )),
        None
    );
    assert!(fixture.calls().is_empty());
}
