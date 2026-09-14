use super::*;
use crate::*;
use pax_runtime_api::pax_value::ImplToFromPaxAny;
use pax_runtime_api::{Layer, Platform, OS};
use std::cell::Cell;

thread_local! {
    static CREATES: Cell<usize> = const { Cell::new(0) };
    static BINDS: Cell<usize> = const { Cell::new(0) };
    static SCOPES: Cell<usize> = const { Cell::new(0) };
    static MOUNTS: RefCell<Vec<(f64, f64)>> = const { RefCell::new(Vec::new()) };
}

struct Probe {
    value: Property<f64>,
    untouched: Property<f64>,
}
impl Default for Probe {
    fn default() -> Self {
        Self {
            value: Property::new(7.0),
            untouched: Property::new(19.0),
        }
    }
}
impl ImplToFromPaxAny for Probe {}

static SCOPE: [PropertyScopeDescriptor<Probe>; 2] = [
    PropertyScopeDescriptor::new("value", |p| {
        Variable::new_from_typed_property(p.value.clone())
    }),
    PropertyScopeDescriptor::new("untouched", |p| {
        Variable::new_from_typed_property(p.untouched.clone())
    }),
];
static FIELDS: [ComponentPropertyDescriptor<Probe>; 2] = [
    ComponentPropertyDescriptor::new("value", |p, entries, stack| {
        let mut layered = Property::new(Probe::default().value.get());
        let mut alias = None;
        for entry in entries {
            let source = entry.source_stack.as_ref().unwrap_or(stack);
            let base_stack = stack_with_base(source, layered.clone());
            let timeline_stack = if matches!(&entry.value, ValueDefinition::Timeline(track) if track.use_local_property_scope)
            {
                let mut local = build_property_scope(p, &SCOPE);
                local.insert(
                    "value".into(),
                    Variable::new_from_typed_property(layered.clone()),
                );
                base_stack.push(local)
            } else {
                base_stack.clone()
            };
            let candidate = build_component_property(
                "value",
                &entry.value,
                &base_stack,
                timeline_stack,
                |_, _| unreachable!(),
            );
            alias = if entry.condition.is_none()
                && matches!(entry.value, ValueDefinition::DoubleBinding(_))
            {
                Some(candidate.clone())
            } else {
                None
            };
            layered = apply_runtime_settings_condition(
                "value",
                entry.condition.as_ref(),
                source,
                layered,
                candidate,
            );
        }
        if let Some(alias) = alias {
            p.value = alias;
        } else {
            p.value.replace_with(layered);
        }
    }),
    ComponentPropertyDescriptor::new("untouched", |p, entries, stack| {
        if let Some(entry) = entries.last() {
            let candidate = build_component_property(
                "untouched",
                &entry.value,
                stack,
                stack.clone(),
                |_, _| unreachable!(),
            );
            if matches!(entry.value, ValueDefinition::DoubleBinding(_)) {
                p.untouched = candidate;
            } else {
                p.untouched.replace_with(candidate);
            }
        }
    }),
];
static TYPED: ComponentDescriptor<Probe> =
    ComponentDescriptor::new("test::Probe", &SCOPE, &FIELDS, &[], |args| {
        Leaf::instantiate(args)
    });
static ERASED: ErasedComponentDescriptor = ErasedComponentDescriptor::new(
    "test::Probe",
    &TYPED,
    || {
        CREATES.with(|c| c.set(c.get() + 1));
        erased_create_properties::<Probe>()
    },
    |descriptor, props, columns, stack| {
        BINDS.with(|c| c.set(c.get() + 1));
        erased_apply_defined_properties::<Probe>(descriptor, props, columns, stack);
    },
    |descriptor, props| {
        SCOPES.with(|c| c.set(c.get() + 1));
        erased_build_property_scope::<Probe>(descriptor, props)
    },
    &[],
    |args| Leaf::instantiate(args),
);

struct Leaf {
    base: BaseInstance,
}
impl InstanceNode for Leaf {
    fn instantiate(args: InstantiationArgs) -> Rc<Self> {
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    layer: Layer::DontCare,
                    invisible_to_slot: false,
                    invisible_to_raycasting: true,
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
        f: &mut std::fmt::Formatter,
        _: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.write_str("Probe")
    }
    fn handle_mount(self: Rc<Self>, node: &Rc<ExpandedNode>, _: &Rc<RuntimeContext>) {
        let values = values(node);
        MOUNTS.with(|seen| seen.borrow_mut().push(values));
    }
}

fn args() -> InstantiationArgs {
    InstantiationArgs {
        prototypical_common_properties: CommonPropertiesInit::Default,
        prototypical_properties: PropertiesInit::DescriptorDefault(&ERASED),
        handler_registry: None,
        children: None,
        component_template: None,
        component_settings: None,
        template_node_identifier: None,
        template_node_type_id: None,
        template_node_selector_info: None,
        transition_config: Default::default(),
        properties_scope: PropertiesScopeInit::Descriptor(&ERASED),
    }
}
fn fixture() -> (PaxEngine, Rc<ExpandedNode>) {
    let mut engine = PaxEngine::new_empty(
        (320.0, 240.0),
        Platform::Web,
        OS::Mac,
        Box::new(|| 0),
        Default::default(),
    );
    let root = engine.mount_root_component(ComponentInstance::instantiate(args()));
    CREATES.with(|c| c.set(0));
    BINDS.with(|c| c.set(0));
    SCOPES.with(|c| c.set(0));
    MOUNTS.with(|m| m.borrow_mut().clear());
    (engine, root)
}
fn expression(raw: &str) -> ValueDefinition {
    Functions::register_all_functions();
    ValueDefinition::Expression(ExpressionInfo::new(
        pax_language::parse_pax_expression(raw).unwrap(),
    ))
}
fn setting(name: &str, value: ValueDefinition) -> SettingElement {
    SettingElement::Setting(
        pax_manifest::Token::new_without_location(name.into()),
        value,
    )
}
fn literal(value: f64) -> ValueDefinition {
    ValueDefinition::LiteralValue(value.to_pax_value())
}
fn plan(
    settings: Vec<SettingElement>,
    classes: Option<ValueDefinition>,
    selectors: Option<Vec<SettingsBlockElement>>,
) -> Rc<TemplatePropertyPlan> {
    let mut node = TemplateNodeDefinition {
        type_id: TypeId::build_singleton("test::Probe", Some("Probe")),
        control_flow_settings: None,
        settings: Some(settings),
        selector_info: pax_manifest::TemplateNodeSelectorInfo {
            class_binding: classes,
            ..Default::default()
        },
        raw_comment_string: None,
    };
    node.normalize_selector_info();
    Rc::new(TemplatePropertyPlan::new(
        node,
        BTreeMap::new(),
        selectors,
        vec![],
    ))
}
fn template(plan: &Rc<TemplatePropertyPlan>) -> Rc<Leaf> {
    let mut args = args();
    args.template_node_type_id = Some(plan.node.type_id.clone());
    args.template_node_selector_info = Some(plan.node.selector_info.clone());
    args.prototypical_common_properties = CommonPropertiesInit::Template(plan.clone());
    args.prototypical_properties = PropertiesInit::Template {
        descriptor: &ERASED,
        plan: plan.clone(),
    };
    Leaf::instantiate(args)
}
fn child(
    root: &Rc<ExpandedNode>,
    ctx: &Rc<RuntimeContext>,
    instance: Rc<dyn InstanceNode>,
    vars: HashMap<String, Variable>,
) -> Rc<ExpandedNode> {
    root.create_children_detached(
        [(instance, ctx.globals().stack_frame().push(vars))],
        ctx,
        &Rc::downgrade(root),
    )
    .remove(0)
}
fn values(node: &Rc<ExpandedNode>) -> (f64, f64) {
    let props = Rc::clone(&node.properties.borrow());
    let props = props.as_ref().borrow();
    let p = Probe::ref_from_pax_any(&props).unwrap();
    (p.value.get(), p.untouched.get())
}

#[test]
fn template_allocates_binds_and_publishes_scope_once_before_mount() {
    let (engine, root) = fixture();
    let source = Property::new(31.0);
    let plan = plan(
        vec![
            setting("value", expression("source + 1")),
            setting("width", expression("80px")),
            setting(
                "id",
                ValueDefinition::Identifier(PaxIdentifier::new("probe")),
            ),
        ],
        None,
        None,
    );
    let node = child(
        &root,
        &engine.runtime_context,
        template(&plan),
        HashMap::from([(
            "source".into(),
            Variable::new_from_typed_property(source.clone()),
        )]),
    );
    assert_eq!(
        (
            CREATES.with(Cell::get),
            BINDS.with(Cell::get),
            SCOPES.with(Cell::get),
            plan.resolutions.get()
        ),
        (1, 1, 1, 1)
    );
    assert_eq!(values(&node), (32.0, 19.0));
    assert_eq!(
        node.selector_metadata.borrow().id.get().as_deref(),
        Some("probe")
    );
    node.recurse_mount(&engine.runtime_context);
    assert_eq!(MOUNTS.with(|m| m.borrow().clone()), vec![(32.0, 19.0)]);
    source.set(55.0);
    assert_eq!(values(&node), (56.0, 19.0));
    node.recurse_unmount(&engine.runtime_context);
}

#[test]
fn shared_plan_keeps_nodes_independent_and_publishes_final_double_binding_alias() {
    let (engine, root) = fixture();
    let plan = plan(
        vec![setting(
            "value",
            ValueDefinition::DoubleBinding(PaxIdentifier::new("source")),
        )],
        None,
        None,
    );
    let a = Property::new(10.0);
    let b = Property::new(20.0);
    let first = child(
        &root,
        &engine.runtime_context,
        template(&plan),
        HashMap::from([(
            "source".into(),
            Variable::new_from_typed_property(a.clone()),
        )]),
    );
    let second = child(
        &root,
        &engine.runtime_context,
        template(&plan),
        HashMap::from([(
            "source".into(),
            Variable::new_from_typed_property(b.clone()),
        )]),
    );
    assert_eq!(plan.resolutions.get(), 2);
    assert_eq!(
        first.properties_scope.borrow()["value"]
            .get_untyped_property()
            .get_id(),
        a.untyped().get_id()
    );
    assert_eq!(
        second.properties_scope.borrow()["value"]
            .get_untyped_property()
            .get_id(),
        b.untyped().get_id()
    );
    a.set(30.0);
    assert_eq!(values(&first).0, 30.0);
    assert_eq!(values(&second).0, 20.0);
}

#[test]
fn selector_changes_resolve_once_per_rebind_and_reset_removed_component_properties() {
    let (engine, root) = fixture();
    let classes = Property::new("active".to_string());
    let plan = plan(
        vec![],
        Some(expression("classes")),
        Some(vec![SettingsBlockElement::SelectorBlock(
            pax_manifest::Token::new_without_location(".active".into()),
            LiteralBlockDefinition::new(vec![
                setting("value", literal(44.0)),
                setting("width", expression("80px")),
            ]),
        )]),
    );
    let node = child(
        &root,
        &engine.runtime_context,
        template(&plan),
        HashMap::from([(
            "classes".into(),
            Variable::new_from_typed_property(classes.clone()),
        )]),
    );
    assert_eq!(values(&node), (44.0, 19.0));
    engine.runtime_context.drain_node_effects();
    assert_eq!(plan.resolutions.get(), 1);
    classes.set(String::new());
    engine.runtime_context.drain_node_effects();
    assert_eq!(plan.resolutions.get(), 2);
    assert_eq!(values(&node), (7.0, 19.0));
    assert_eq!(
        node.get_common_properties().as_ref().borrow().width.get(),
        None
    );
    classes.set("active".into());
    engine.runtime_context.drain_node_effects();
    assert_eq!(plan.resolutions.get(), 3);
    assert_eq!(values(&node).0, 44.0);
}

#[test]
fn reload_rebinds_again_without_reallocating_component_slots() {
    let (engine, root) = fixture();
    let first = plan(vec![setting("value", literal(4.0))], None, None);
    let node = child(
        &root,
        &engine.runtime_context,
        template(&first),
        HashMap::new(),
    );
    let slot = node.properties_scope.borrow()["value"]
        .get_untyped_property()
        .get_id();
    let second = plan(vec![setting("value", literal(12.0))], None, None);
    node.recreate_with_new_data(template(&second), &engine.runtime_context);
    assert_eq!(values(&node), (12.0, 19.0));
    assert_eq!(
        node.properties_scope.borrow()["value"]
            .get_untyped_property()
            .get_id(),
        slot
    );
    assert_eq!(CREATES.with(Cell::get), 1);
    assert!(second.resolutions.get() > 0);
}

#[test]
fn legacy_factories_and_scope_callbacks_keep_both_phases_in_mixed_initializers() {
    let (engine, root) = fixture();
    let calls = Rc::new(RefCell::new(vec![]));
    let mut args = args();
    args.prototypical_common_properties = CommonPropertiesInit::Inline {
        defined_properties: BTreeMap::from([("width".into(), expression("80px"))]),
    };
    let events = calls.clone();
    args.prototypical_properties = PropertiesInit::Factory(Box::new(move |_, node| {
        if let Some(node) = node {
            events.borrow_mut().push("bind");
            assert!(node.properties_scope.borrow().contains_key("value"));
            assert_eq!(
                node.get_common_properties().as_ref().borrow().width.get(),
                Some(Size::Pixels(80.into()))
            );
            None
        } else {
            events.borrow_mut().push("allocate");
            Some(Rc::new(RefCell::new(Probe::default().to_pax_any())))
        }
    }));
    let events = calls.clone();
    args.properties_scope = PropertiesScopeInit::Factory(Box::new(move |properties| {
        events.borrow_mut().push("scope");
        let props = properties.as_ref().borrow();
        build_property_scope(Probe::ref_from_pax_any(&props).unwrap(), &SCOPE)
    }));
    let node = child(
        &root,
        &engine.runtime_context,
        Leaf::instantiate(args),
        HashMap::new(),
    );
    assert_eq!(
        *calls.as_ref().borrow(),
        ["allocate", "scope", "bind", "scope"]
    );
    assert_eq!(values(&node), (7.0, 19.0));
}

#[test]
fn local_timeline_preserves_forward_double_binding_and_clock() {
    for use_template_plan in [false, true] {
        let (engine, root) = fixture();
        let mut args = args();
        args.prototypical_properties = PropertiesInit::DescriptorInline {
            descriptor: &ERASED,
            defined_properties: BTreeMap::from([(
                "value".into(),
                ValueDefinition::Timeline(TimelineTrackDefinition {
                    elements: vec![
                        TimelineTrackElement::Keyframe(TimelineKeyframe {
                            marker: TimelineMarker::Frame(0),
                            value: expression("self.untouched"),
                            easing: Some(pax_manifest::Token::new_without_location(
                                "Linear".into(),
                            )),
                        }),
                        TimelineTrackElement::Keyframe(TimelineKeyframe {
                            marker: TimelineMarker::Frame(100),
                            value: expression("self.untouched + 100"),
                            easing: None,
                        }),
                    ],
                    playhead: None,
                    duration: None,
                    repeat: Some(false),
                    starting_value: None,
                    interruption: Default::default(),
                    use_local_property_scope: true,
                }),
            )]),
        };
        if let PropertiesInit::DescriptorInline {
            defined_properties, ..
        } = &mut args.prototypical_properties
        {
            defined_properties.insert(
                "untouched".into(),
                ValueDefinition::DoubleBinding(PaxIdentifier::new("source")),
            );
        }
        let source = Property::new(31.0);
        let instance: Rc<dyn InstanceNode> = if use_template_plan {
            let PropertiesInit::DescriptorInline {
                defined_properties, ..
            } = args.prototypical_properties
            else {
                unreachable!()
            };
            let settings = defined_properties
                .into_iter()
                .map(|(name, value)| setting(&name, value))
                .collect();
            template(&plan(settings, None, None))
        } else {
            Leaf::instantiate(args)
        };
        let node = child(
            &root,
            &engine.runtime_context,
            instance,
            HashMap::from([(
                "source".into(),
                Variable::new_from_typed_property(source.clone()),
            )]),
        );
        assert_eq!(
            (
                CREATES.with(Cell::get),
                BINDS.with(Cell::get),
                SCOPES.with(Cell::get)
            ),
            (1, 2, 2)
        );
        assert_eq!(values(&node).0, 31.0);
        engine.runtime_context.globals().elapsed_frames.set(50);
        assert_eq!(values(&node).0, 81.0);
        source.set(40.0);
        assert_eq!(values(&node).0, 90.0);
    }
}

#[test]
fn repeated_plan_instantiation_releases_node_local_properties() {
    let (engine, root) = fixture();
    engine.runtime_context.drain_node_effects();
    let initial = pax_runtime_api::properties::property_table_total_properties_count();
    let plan = plan(vec![setting("value", literal(42.0))], None, None);
    for _ in 0..64 {
        let node = child(
            &root,
            &engine.runtime_context,
            template(&plan),
            HashMap::new(),
        );
        assert_eq!(values(&node).0, 42.0);
        drop(node);
        engine.runtime_context.drain_node_effects();
    }
    assert_eq!(
        pax_runtime_api::properties::property_table_total_properties_count(),
        initial
    );
    assert_eq!(Rc::strong_count(&plan), 1);
    assert_eq!(plan.resolutions.get(), 64);
}

#[test]
fn imported_provider_scope_and_inline_base_remain_reactive() {
    let (engine, root) = fixture();
    let theme = Property::new(8.0);
    root.imported_settings_layers
        .borrow_mut()
        .push(RuntimeSettingsLayer {
            provider_id: root.id,
            provider_type_id: TypeId::build_singleton("test::Theme", Some("Theme")),
            provider_stack: engine
                .runtime_context
                .globals()
                .stack_frame()
                .push(HashMap::from([(
                    "theme".into(),
                    Variable::new_from_typed_property(theme.clone()),
                )])),
            settings: vec![SettingsBlockElement::SelectorBlock(
                pax_manifest::Token::new_without_location("Probe".into()),
                LiteralBlockDefinition::new(vec![setting("value", expression("theme"))]),
            )],
        });
    let plan = plan(vec![setting("value", expression("$base + 1"))], None, None);
    let node = child(
        &root,
        &engine.runtime_context,
        template(&plan),
        HashMap::new(),
    );
    assert_eq!(values(&node).0, 9.0);
    theme.set(13.0);
    assert_eq!(values(&node).0, 14.0);
    assert_eq!(plan.resolutions.get(), 1);
}

struct Traverser(RefCell<pax_manifest::PaxManifest>);
impl DefinitionToInstanceTraverser for Traverser {
    fn new(manifest: pax_manifest::PaxManifest) -> Self {
        Self(RefCell::new(manifest))
    }
    fn get_manifest(&self) -> std::cell::Ref<'_, pax_manifest::PaxManifest> {
        self.0.borrow()
    }
    fn get_component_descriptor(&self, _: &TypeId) -> Option<&'static ErasedComponentDescriptor> {
        Some(&ERASED)
    }
}

#[test]
fn rich_and_binary_manifests_build_the_same_shared_node_local_plan() {
    use pax_manifest::{ComponentDefinition, ComponentTemplate, PaxManifest};
    let root_type = TypeId::build_singleton("test::Root", Some("Root"));
    let leaf_plan = plan(
        vec![
            setting("value", literal(42.0)),
            setting("width", expression("80px")),
        ],
        None,
        None,
    );
    let leaf_type = leaf_plan.node.type_id.clone();
    let mut template = ComponentTemplate::new(root_type.clone(), None);
    let leaf_id = template.add_root_node_back(leaf_plan.node.clone());
    let component = |type_id, template, is_main_component| ComponentDefinition {
        type_id,
        template,
        is_main_component,
        is_primitive: false,
        is_struct_only_component: false,
        module_path: "test".into(),
        primitive_instance_import_path: None,
        settings: None,
        timelines: vec![],
        route_branch: None,
    };
    let rich = PaxManifest {
        components: BTreeMap::from([
            (
                root_type.clone(),
                component(root_type.clone(), Some(template), true),
            ),
            (leaf_type.clone(), component(leaf_type, None, false)),
        ]),
        main_component_type_id: root_type.clone(),
        type_table: HashMap::new(),
        assets_dirs: vec![],
        engine_import_path: "pax_engine".into(),
    };
    let binary =
        pax_manifest::binary::from_slice(&pax_manifest::binary::to_vec(&rich).unwrap()).unwrap();
    for manifest in [rich, binary] {
        let (engine, root) = fixture();
        let traverser = Traverser::new(manifest);
        let instance =
            traverser.build_template_node(&root_type, &leaf_id.get_template_node_id(), None);
        let PropertiesInit::Template { plan, .. } =
            &instance.base().instance_prototypical_properties
        else {
            panic!("expected shared template plan")
        };
        let plan = plan.clone();
        let node = child(&root, &engine.runtime_context, instance, HashMap::new());
        assert_eq!(
            (
                CREATES.with(Cell::get),
                BINDS.with(Cell::get),
                SCOPES.with(Cell::get),
                plan.resolutions.get()
            ),
            (1, 1, 1, 1)
        );
        assert_eq!(values(&node), (42.0, 19.0));
        assert_eq!(
            node.get_common_properties().as_ref().borrow().width.get(),
            Some(Size::Pixels(80.into()))
        );
    }
}
