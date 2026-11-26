use super::*;

#[test]
fn projected_height_waits_for_current_wrapping_feedback_and_places_both_followers() {
    use crate::core::text::{Text, TextInstance};
    use pax_message::{
        NativeInterrupt, NativeMessage, TextMeasurementInvalidatedArgs, TextMeasurementResponseArgs,
    };
    use pax_runtime::{Slot, SlotInstance};
    thread_local! {
        static HOOKS: RefCell<Vec<(u32, bool)>> = const { RefCell::new(Vec::new()) };
    }
    fn mounted(_: Rc<RefCell<PaxAny>>, ctx: &NodeContext, _: Option<PaxAny>) {
        let n = ctx.expanded_node.upgrade().unwrap();
        assert!(ctx.bounds_self.get().0 > 0.);
        HOOKS.with_borrow_mut(|log| log.push((n.id.0, true)));
    }
    fn unmounted(_: Rc<RefCell<PaxAny>>, ctx: &NodeContext, _: Option<PaxAny>) {
        HOOKS.with_borrow_mut(|log| log.push((ctx.expanded_node.upgrade().unwrap().id.0, false)));
    }
    fn named(mut args: InstantiationArgs, name: &str) -> InstantiationArgs {
        args.template_node_selector_info = Some(pax_manifest::TemplateNodeSelectorInfo {
            id: Some(pax_manifest::Token::new_without_location(name.into())),
            ..Default::default()
        });
        args
    }
    fn sized(
        mut args: InstantiationArgs,
        width: LocalProperty<Option<Size>>,
        height: LocalProperty<Option<Size>>,
    ) -> InstantiationArgs {
        args.prototypical_common_properties =
            CommonPropertiesInit::Factory(Box::new(move |_, _| {
                Some(Rc::new(RefCell::new(CommonProperties {
                    width: width.clone(),
                    height: height.clone(),
                    ..Default::default()
                })))
            }));
        args
    }
    fn px(v: i64) -> LocalProperty<Option<Size>> {
        LocalProperty::new(Some(Size::Pixels(v.into())))
    }
    fn leaf(h: i64) -> Rc<dyn InstanceNode> {
        TestLeaf::instantiate(sized(primitive_args(vec![]), px(100), px(h)))
    }
    fn generation(messages: Vec<NativeMessage>) -> Option<u64> {
        messages
            .into_iter()
            .filter_map(|m| match m {
                NativeMessage::TextUpdate(p) => p.measure_generation,
                _ => None,
            })
            .last()
    }
    let run = |staged| {
        let w = px(100);
        let row_height = px(80);
        let mut text_args = sized(primitive_args(vec![]), w.clone(), LocalProperty::new(None));
        text_args.prototypical_properties = PropertiesInit::Factory(Box::new(|_, n| {
            n.is_none().then(|| {
                Rc::new(RefCell::new(
                    Text {
                        text: Property::new("a paragraph that wraps in the drawer".into()),
                        ..Default::default()
                    }
                    .to_pax_any(),
                ))
            })
        }));
        let text_instance = TextInstance::instantiate(text_args);
        let demo =
            ComponentInstance::instantiate(component_args((0..12).map(|_| leaf(8)).collect()));
        let content = StackerInstance::instantiate(named(
            sized(
                stacker_args_with(
                    vec![text_instance.clone(), demo],
                    Stacker {
                        autosize: Property::new(true),
                        sizes: Property::new(vec![None, Some(Size::Pixels(90.into()))]),
                        ..Default::default()
                    },
                ),
                w.clone(),
                LocalProperty::new(None),
            ),
            "copy",
        ));
        HOOKS.with_borrow_mut(Vec::clear);
        let mut provider_args = component_args(vec![content]);
        let mut handlers = pax_runtime::HandlerRegistry::default();
        handlers.handlers.insert(
            "mount".into(),
            vec![pax_runtime::Handler::new_component_handler(mounted)],
        );
        handlers.handlers.insert(
            "unmount".into(),
            vec![pax_runtime::Handler::new_component_handler(unmounted)],
        );
        provider_args.handler_registry = Some(Rc::new(RefCell::new(handlers)));
        let provider = ComponentInstance::instantiate(provider_args);
        let selected = LocalProperty::new(Numeric::I64(0));
        let selection = selected.clone();
        let mut slot = primitive_args(vec![]);
        slot.prototypical_properties = PropertiesInit::Factory(Box::new(move |_, n| {
            n.is_none().then(|| {
                Rc::new(RefCell::new(
                    Slot {
                        index: selection.clone(),
                        ..Default::default()
                    }
                    .to_pax_any(),
                ))
            })
        }));
        let mut host = named(
            sized(
                component_args(vec![SlotInstance::instantiate(slot)]),
                w.clone(),
                row_height.clone(),
            ),
            "row",
        );
        host.children = Some(RefCell::new(vec![provider.clone(), provider]));
        let flow = StackerInstance::instantiate(sized(
            stacker_args_with(
                vec![
                    leaf(200),
                    ComponentInstance::instantiate(host),
                    leaf(110),
                    leaf(120),
                ],
                Stacker {
                    autosize: Property::new(true),
                    ..Default::default()
                },
            ),
            w.clone(),
            LocalProperty::new(None),
        ));
        let ctx = Rc::new(crate::test_support::runtime_context());
        let mut engine = pax_runtime::PaxEngine {
            runtime_context: ctx.clone(),
            root_expanded_node: None,
            scroller_tiling_policy: Default::default(),
        };
        if staged {
            engine.enable_resumable_startup();
            engine.enable_indexed_mounts(0.);
        }
        engine.set_viewport_size((100., 100.));
        let root =
            engine.mount_root_component(ComponentInstance::instantiate(component_args(vec![flow])));
        let node_ctx = root.get_node_context(&ctx);
        let mut pending = generation(ctx.take_native_messages());
        let mut clock = 0.;
        if staged {
            node_ctx.declare_projected_height("row", "copy").unwrap();
            for _ in 0..1000 {
                engine.service_startup(0., || 0.);
                if engine.accept_startup_measurements() {
                    break;
                }
            }
            assert!(engine.begin_background_mounting());
            engine.prioritize_background_owners();
            engine.observe_mount_hosts(0., [0., 290., 100., 400.], &Default::default(), &[], 250.);
            let initial = ctx.inspect_projected_heights().pop().unwrap();
            assert_eq!(initial.consumers.len(), 2);
            assert!(
                initial.exact.is_none() && !initial.pending_mounts.is_empty(),
                "positive reserve is provisional"
            );
            for _ in 0..300 {
                engine.service_background_mounts(0., || {
                    clock += 1.;
                    clock
                });
                let m = engine.tick();
                pending = generation(m).or(pending);
                if pending.is_some() {
                    break;
                }
            }
        } else {
            pending = generation(engine.tick()).or(pending);
        }
        let flow = root.children.get()[0].clone();
        let rows = flow.children.get();
        let stack = rows[1].children.get()[0].children.get()[0].children.get()[0].clone();
        let text = stack.children.get()[0].clone();
        let old = pending.expect("native host received first wrapping request");
        let measured = stack.measured_size.clone();
        let report = || ctx.inspect_projected_heights().pop().and_then(|r| r.exact);
        let send = |generation, height| {
            text_instance.handle_native_interrupt(
                &text,
                &NativeInterrupt::TextMeasurementResponse(TextMeasurementResponseArgs {
                    id: text.id.0,
                    generation,
                    width: text.presentation_bounds().get().0,
                    height,
                }),
            )
        };
        // The source's ordinary Rust feedback keeps the two followers synchronous.
        let feedback = row_height.clone();
        let h = measured.clone();
        node_ctx.subscribe(&[measured.untyped()], move || {
            if let Some((_, v)) = h.get() {
                feedback.set_if_neq(Some(Size::Pixels(v.into())));
            }
        });
        w.set(Some(Size::Pixels(60.into())));
        ctx.drain_node_effects();
        let narrow =
            generation(ctx.take_native_messages()).expect("narrower wrap requests new generation");
        send(old, 18.);
        assert_eq!(text.measured_size.get(), None);
        send(narrow, 54.);
        if staged {
            let r = ctx.inspect_projected_heights().pop().unwrap();
            assert!(r.pending_settlement && r.exact.is_none());
        }
        ctx.drain_node_effects();
        assert_eq!(measured.get().unwrap().1, 144.);
        if staged {
            assert_eq!(report(), Some(144.));
        }
        text_instance.handle_native_interrupt(
            &text,
            &NativeInterrupt::TextMeasurementInvalidated(TextMeasurementInvalidatedArgs {
                id: text.id.0,
                generation: narrow,
            }),
        );
        if staged {
            assert_eq!(report(), None);
        }
        ctx.drain_node_effects();
        let font = generation(ctx.take_native_messages()).unwrap();
        send(narrow, 18.);
        assert_eq!(measured.get().unwrap().1, 144.);
        send(font, 72.);
        ctx.drain_node_effects();
        if staged {
            assert_eq!(report(), Some(162.));
        }
        if staged {
            for top in [420., 280.] {
                engine.observe_mount_hosts(
                    clock,
                    [0., top, 100., top + 100.],
                    &Default::default(),
                    &[],
                    250.,
                );
                assert!(
                    generation(engine.tick()).is_none(),
                    "pure scroll reuses the native measurement"
                );
                assert_eq!(report(), Some(162.));
            }
        }
        let pos = |i: usize| rows[i].container_frame.get().unwrap().transform.m[5];
        let answer = (measured.get().unwrap().1, pos(2), pos(3));
        // Switch slots with a font measurement in flight. Both projected
        // components share a template and own different native lifetimes.
        text_instance.handle_native_interrupt(
            &text,
            &NativeInterrupt::TextMeasurementInvalidated(TextMeasurementInvalidatedArgs {
                id: text.id.0,
                generation: font,
            }),
        );
        ctx.drain_node_effects();
        let abandoned = generation(ctx.take_native_messages()).unwrap();
        selected.set(Numeric::I64(1));
        let mut replacement = None;
        for _ in 0..300 {
            replacement = generation(engine.tick()).or(replacement);
            if replacement.is_some() {
                break;
            }
            if staged {
                engine.service_background_mounts(0., || {
                    clock += 1.;
                    clock
                });
            }
        }
        let replacement = replacement.expect("new selected text requests its own measurement");
        let new_stack = rows[1].children.get()[0].children.get()[0].children.get()[0].clone();
        let new_text = new_stack.children.get()[0].clone();
        assert_ne!(new_text.id, text.id);
        assert_ne!(replacement, abandoned);
        let deliver = |g, h| {
            text_instance.handle_native_interrupt(
                &new_text,
                &NativeInterrupt::TextMeasurementResponse(TextMeasurementResponseArgs {
                    id: new_text.id.0,
                    generation: g,
                    width: 60.,
                    height: h,
                }),
            )
        };
        deliver(abandoned, 99.);
        assert!(new_text.measured_size.get().is_none());
        if staged {
            assert!(report().is_none());
        }
        deliver(replacement, 54.);
        ctx.drain_node_effects();
        if staged {
            assert_eq!(report(), Some(144.));
        }
        engine.unmount();
        HOOKS.with_borrow(|log| {
            assert_eq!(log.len(), 4);
            assert_eq!(log[0], (log[1].0, true));
            assert!(!log[1].1);
            assert_ne!(log[0].0, log[2].0);
            assert_eq!(log[2], (log[3].0, true));
            assert!(!log[3].1);
        });
        answer
    };
    let reference = run(false);
    assert_eq!(reference, (162., 362., 472.));
    assert_eq!(run(true), reference);
}

#[test]
fn measuring_a_projected_stack_skips_its_allocated_demo() {
    use pax_runtime::engine::startup::{StartupAxes, StartupDemand};

    fn layout_args(mut args: InstantiationArgs, height: Option<f64>) -> InstantiationArgs {
        args.prototypical_common_properties =
            CommonPropertiesInit::Factory(Box::new(move |_, _| {
                Some(Rc::new(RefCell::new(CommonProperties {
                    width: LocalProperty::new(Some(Size::Pixels(100.into()))),
                    height: LocalProperty::new(height.map(|v| Size::Pixels(v.into()))),
                    ..Default::default()
                })))
            }));
        args
    }
    let run = |staged: bool| {
        let text = TestLeaf::instantiate(layout_args(primitive_args(vec![]), None));
        let detail = TestLeaf::instantiate(primitive_args(vec![]));
        let demo =
            ComponentInstance::instantiate(layout_args(component_args(vec![detail]), Some(90.0)));
        let stack = StackerInstance::instantiate(layout_args(
            stacker_args_with(
                vec![text, demo],
                Stacker {
                    autosize: Property::new(true),
                    sizes: Property::new(vec![None, Some(Size::Pixels(90.into()))]),
                    ..Default::default()
                },
            ),
            None,
        ));
        let ctx = Rc::new(crate::test_support::runtime_context());
        let mut engine = pax_runtime::PaxEngine {
            runtime_context: ctx.clone(),
            root_expanded_node: None,
            scroller_tiling_policy: Default::default(),
        };
        if staged {
            engine.enable_resumable_startup();
        }
        let root = engine
            .mount_root_component(ComponentInstance::instantiate(component_args(vec![stack])));
        if staged {
            root.get_node_context(&ctx).set_startup_planner(|_, plan| {
                plan.require_children(StartupDemand::measure(StartupAxes { x: false, y: true }));
            });
            for _ in 0..2000 {
                engine.service_startup(0.0, || 0.0);
                if engine.accept_startup_measurements() {
                    break;
                }
            }
        }
        let stack = root.children.get()[0].clone();
        let rows = stack.children.get();
        if staged {
            let flow = stack
                .instance_node
                .borrow()
                .measurement_flow(&stack, StartupAxes { x: false, y: true })
                .unwrap();
            assert_eq!(flow[1].1, Some(90.0));
            assert_eq!(flow[1].0.attached.get(), 0);
        }
        assert_ne!(rows[0].attached.get(), 0);
        rows[0].set_measured_size(100.0, 36.0);
        ctx.drain_node_effects();
        let extent = stack.measured_size.get().unwrap().1;
        let demo_attached = rows[1].attached.get() != 0;
        engine.unmount();
        (extent, demo_attached)
    };
    assert_eq!(run(false), (126.0, true));
    assert_eq!(run(true), (126.0, false));
}
