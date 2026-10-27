use super::*;
use crate::*;
use pax_runtime_api::pax_value::ImplToFromPaxAny;
use pax_runtime_api::{Layer, Platform, OS};
use std::cell::Cell;

#[path = "store_tests.rs"]
mod store_tests;

thread_local! {
    static CREATES: Cell<usize> = const { Cell::new(0) };
    static BINDS: Cell<usize> = const { Cell::new(0) };
    static SCOPES: Cell<usize> = const { Cell::new(0) };
    static MOUNTS: RefCell<Vec<(f64, f64)>> = const { RefCell::new(Vec::new()) };
}

struct Probe {
    value: LocalProperty<f64>,
    untouched: LocalProperty<f64>,
}
impl Default for Probe {
    fn default() -> Self {
        Self {
            value: LocalProperty::new(7.0),
            untouched: LocalProperty::new(19.0),
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
    ComponentPropertyDescriptor::new("value", |p, entries, stack, node| {
        let mut layered = LocalProperty::new(Probe::default().value.get());
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
            clear_settings_motion(node, "value");
            p.value = alias;
        } else {
            crate::bind_settings_property(&p.value, layered, "value", entries, stack, node);
        }
    }),
    ComponentPropertyDescriptor::new("untouched", |p, entries, stack, node| {
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
                bind_settings_property(&p.untouched, candidate, "untouched", entries, stack, node);
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
    |descriptor, props, columns, stack, node| {
        BINDS.with(|c| c.set(c.get() + 1));
        erased_apply_defined_properties::<Probe>(descriptor, props, columns, stack, node);
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
    let mut engine = crate::test_support::empty_engine(
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
fn supplied_root_keeps_its_allocation_and_local_bindings_without_defaulting() {
    let (mut engine, _) = fixture();
    let source = LocalProperty::new(41.0);
    let observed = source.clone();
    let computed = LocalProperty::computed(move || observed.get() + 1.0, &[source.untyped()]);
    let supplied = Rc::new(RefCell::new(
        Probe {
            value: computed,
            untouched: LocalProperty::new(99.0),
        }
        .to_pax_any(),
    ));
    let mut init = args();
    init.prototypical_properties = init.prototypical_properties.adopt(supplied.clone());
    let node = engine.mount_root_component(ComponentInstance::instantiate(init));
    assert!(Rc::ptr_eq(&supplied, &node.properties.borrow()));
    assert_eq!(CREATES.with(Cell::get), 0);
    assert_eq!(values(&node), (42.0, 99.0));
    source.set(50.0);
    assert_eq!(values(&node), (51.0, 99.0));
}

#[test]
fn supplied_root_applies_declared_bindings_and_preserves_unspecified_values() {
    let (mut engine, _) = fixture();
    let supplied = Rc::new(RefCell::new(
        Probe {
            value: LocalProperty::new(41.0),
            untouched: LocalProperty::new(99.0),
        }
        .to_pax_any(),
    ));
    let mut init = args();
    init.prototypical_properties = PropertiesInit::DescriptorInline {
        descriptor: &ERASED,
        defined_properties: BTreeMap::from([("value".into(), expression("12"))]),
    }
    .adopt(supplied.clone());
    let node = engine.mount_root_component(ComponentInstance::instantiate(init));
    assert!(Rc::ptr_eq(&supplied, &node.properties.borrow()));
    assert_eq!(CREATES.with(Cell::get), 0);
    assert_eq!(values(&node), (12.0, 99.0));
}

#[test]
fn fresh_common_defaults_preserve_slots_without_invalidating_unset_properties() {
    fn watch<T: PropertyValue>(property: &LocalProperty<T>) -> (LocalProperty<T>, Rc<Cell<usize>>) {
        let reads = Rc::new(Cell::new(0));
        let counter = reads.clone();
        let source = property.clone();
        let observed = LocalProperty::computed(
            move || {
                counter.set(counter.get() + 1);
                source.get()
            },
            &[property.untyped()],
        );
        (observed, reads)
    }

    let (_engine, root) = fixture();
    let mut common = CommonProperties::default();
    let width_id = common.width.untyped().get_id();
    let (width, width_reads) = watch(&common.width);
    let (height, height_reads) = watch(&common.height);
    assert_eq!(width.get(), None);
    assert_eq!(height.get(), None);
    let mut columns = property_columns_from_defined_properties(&BTreeMap::from([(
        "x".into(),
        expression("80px"),
    )]));
    columns.insert("height".into(), vec![]);
    update_common_properties(
        &mut common,
        &columns,
        &root.stack,
        None,
        CommonPropertiesBindMode::FreshDefaults,
    );
    assert_eq!(common.x.get(), Some(Size::Pixels(80.into())));
    assert_eq!(width.get(), None);
    assert_eq!(height.get(), None);
    assert_eq!(common.width.untyped().get_id(), width_id);
    assert_eq!(
        width_reads.get(),
        1,
        "an absent fresh slot does not invalidate its readers"
    );
    assert_eq!(
        height_reads.get(),
        2,
        "an explicit reset still installs its target"
    );
    common.width.set(Some(Size::Pixels(40.into())));
    assert_eq!(width.get(), Some(Size::Pixels(40.into())));
    assert_eq!(width_reads.get(), 2, "the original handle remains reactive");
}

#[test]
fn absent_common_properties_still_clear_imperative_values_and_dependencies_on_rebind() {
    let (engine, root) = fixture();
    let plan = plan(vec![], None, None);
    let instance = template(&plan);
    let node = child(
        &root,
        &engine.runtime_context,
        instance.clone(),
        HashMap::new(),
    );
    let common = node.get_common_properties();
    let width = common.as_ref().borrow().width.clone();
    let width_id = width.untyped().get_id();
    assert_eq!(width.get(), None);
    assert_eq!(common.as_ref().borrow().opacity.get(), None);
    let source = LocalProperty::new(Some(Size::Pixels(40.into())));
    let dependency = source.clone();
    width.replace_with(LocalProperty::computed(
        move || dependency.get(),
        &[source.untyped()],
    ));
    common
        .as_ref()
        .borrow()
        .height
        .set(Some(Size::Pixels(80.into())));
    assert_eq!(width.get(), source.get());
    node.recreate_with_new_data(instance, &engine.runtime_context);
    assert_eq!(width.get(), None);
    assert_eq!(common.as_ref().borrow().height.get(), None);
    source.set(Some(Size::Pixels(60.into())));
    assert_eq!(width.get(), None, "rebind disconnects the old computation");
    assert_eq!(common.as_ref().borrow().width.untyped().get_id(), width_id);
}

#[test]
fn initially_inactive_common_settings_remain_reactive_and_reset_when_removed() {
    use pax_manifest::{SettingsConditionalBlock, SettingsConditionalBranch, Token};
    let (engine, root) = fixture();
    let enabled = LocalProperty::new(false);
    let classes = LocalProperty::new("active".to_string());
    let ValueDefinition::Expression(condition) = expression("enabled") else {
        unreachable!()
    };
    let plan = plan(
        vec![],
        Some(expression("classes")),
        Some(vec![SettingsBlockElement::Conditional(
            SettingsConditionalBlock {
                branches: vec![SettingsConditionalBranch {
                    condition_expression: Some(condition),
                    elements: vec![SettingsBlockElement::SelectorBlock(
                        Token::new_without_location(".active".into()),
                        LiteralBlockDefinition::new(vec![setting("width", expression("80px"))]),
                    )],
                }],
            },
        )]),
    );
    let node = child(
        &root,
        &engine.runtime_context,
        template(&plan),
        HashMap::from([
            (
                "enabled".into(),
                Variable::new_from_typed_property(enabled.clone()),
            ),
            (
                "classes".into(),
                Variable::new_from_typed_property(classes.clone()),
            ),
        ]),
    );
    let width = node.get_common_properties().as_ref().borrow().width.clone();
    assert_eq!(width.get(), None);
    enabled.set(true);
    assert_eq!(width.get(), Some(Size::Pixels(80.into())));
    assert_eq!(plan.resolutions.get(), 1);
    enabled.set(false);
    assert_eq!(width.get(), None);
    enabled.set(true);
    engine.runtime_context.drain_node_effects();
    classes.set(String::new());
    engine.runtime_context.drain_node_effects();
    assert_eq!(width.get(), None);
    assert!(node.resolved_property_columns.borrow()["width"].is_empty());
    classes.set("active".into());
    engine.runtime_context.drain_node_effects();
    assert_eq!(width.get(), Some(Size::Pixels(80.into())));
}

#[test]
fn template_allocates_binds_and_publishes_scope_once_before_mount() {
    let (engine, root) = fixture();
    let source = LocalProperty::new(31.0);
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
fn common_values_are_prepared_before_mount_handlers() {
    thread_local! {
        static PREPARATIONS: Cell<usize> = const { Cell::new(0) };
        static HANDLED: Cell<bool> = const { Cell::new(false) };
    }
    fn on_mount(_: Rc<RefCell<PaxAny>>, _: &NodeContext, _: Option<PaxAny>) {
        assert_eq!(PREPARATIONS.with(Cell::get), 1);
        HANDLED.with(|handled| handled.set(true));
    }

    let (engine, root) = fixture();
    let mut registry = HandlerRegistry::default();
    registry
        .handlers
        .insert("mount".into(), vec![Handler::new_inline_handler(on_mount)]);
    let mut args = args();
    args.handler_registry = Some(Rc::new(RefCell::new(registry)));
    let node = child(
        &root,
        &engine.runtime_context,
        Leaf::instantiate(args),
        HashMap::new(),
    );
    PREPARATIONS.with(|count| count.set(0));
    HANDLED.with(|handled| handled.set(false));
    node.get_common_properties()
        .as_ref()
        .borrow()
        .width
        .replace_with(LocalProperty::computed(
            || {
                PREPARATIONS.with(|count| count.set(count.get() + 1));
                Some(Size::Pixels(80.into()))
            },
            &[],
        ));
    node.recurse_mount(&engine.runtime_context);
    assert!(HANDLED.with(Cell::get));
    node.recurse_unmount(&engine.runtime_context);
}

#[test]
fn sibling_components_isolate_state_and_their_template_scopes() {
    let (engine, root) = fixture();
    let child_plan = plan(vec![setting("value", expression("self.value"))], None, None);
    let mut component_args = args();
    component_args.component_template = Some(RefCell::new(vec![template(&child_plan)]));
    let component = ComponentInstance::instantiate(component_args);
    let shared_incoming_scope = root.stack.clone();
    let siblings = root.generate_children(
        [
            (
                component.clone() as Rc<dyn InstanceNode>,
                shared_incoming_scope.clone(),
            ),
            (component as Rc<dyn InstanceNode>, shared_incoming_scope),
        ],
        &engine.runtime_context,
        &root.parent_frame,
        true,
    );
    let first = &siblings[0];
    let second = &siblings[1];
    assert!(Rc::ptr_eq(&first.stack, &second.stack));
    assert_ne!(
        first.properties_scope.borrow()["value"]
            .get_untyped_property()
            .get_id(),
        second.properties_scope.borrow()["value"]
            .get_untyped_property()
            .get_id()
    );
    let first_child = first.children.get()[0].clone();
    let second_child = second.children.get()[0].clone();
    assert!(!Rc::ptr_eq(&first_child.stack, &second_child.stack));
    first.with_properties_unwrapped(|p: &mut Probe| p.value.set(42.0));
    assert_eq!(values(first).0, 42.0);
    assert_eq!(values(&first_child).0, 42.0);
    assert_eq!(values(second).0, 7.0);
    assert_eq!(values(&second_child).0, 7.0);
    assert_eq!(values(&root).0, 7.0);
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
    let a = LocalProperty::new(10.0);
    let b = LocalProperty::new(20.0);
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
    // A tab-like child write must go back only to its explicitly bound owner.
    first.with_properties_unwrapped(|p: &mut Probe| p.value.set(2.0));
    assert_eq!(a.get(), 2.0);
    assert_eq!(b.get(), 20.0);
    second.with_properties_unwrapped(|p: &mut Probe| p.value.set(3.0));
    assert_eq!(a.get(), 2.0);
    assert_eq!(b.get(), 3.0);
    assert_eq!(values(&first).0, 2.0);
}

#[test]
fn definitions_prepare_on_first_use_and_keep_node_values_independent() {
    let (engine, root) = fixture();
    let selectors = [".active", ".unused"].map(|selector| {
        SettingsBlockElement::SelectorBlock(
            pax_manifest::Token::new_without_location(selector.into()),
            LiteralBlockDefinition::new(vec![setting("value", expression("amount"))]),
        )
    });
    let plan = plan(
        vec![setting("width", expression("40px"))],
        Some(expression("classes")),
        Some(selectors.into()),
    );
    let blocks = &plan.component_settings.as_ref().unwrap().0.selectors;
    let instance = template(&plan);
    assert!(plan.inline_columns.get().is_none());
    assert!(blocks.iter().all(|block| block.columns.get().is_none()));
    let classes = LocalProperty::new(String::new());
    let amount = LocalProperty::new(11.0);
    let scope = |classes: LocalProperty<String>, amount: LocalProperty<f64>| {
        HashMap::from([
            ("classes".into(), Variable::new_from_typed_property(classes)),
            ("amount".into(), Variable::new_from_typed_property(amount)),
        ])
    };
    let ctx = &engine.runtime_context;
    let first = child(
        &root,
        ctx,
        instance.clone(),
        scope(classes.clone(), amount.clone()),
    );
    ctx.drain_node_effects();
    assert!(plan.inline_columns.get().is_some());
    assert!(blocks.iter().all(|block| block.columns.get().is_none()));
    assert_eq!(values(&first).0, 7.0);
    classes.set("active".into());
    ctx.drain_node_effects();
    assert!(blocks[0].columns.get().is_some());
    assert!(blocks[1].columns.get().is_none());
    let definition = first.resolved_property_columns.borrow()["value"][0]
        .definition
        .clone();
    let second = child(
        &root,
        ctx,
        instance,
        scope(
            LocalProperty::new("active".into()),
            LocalProperty::new(22.0),
        ),
    );
    assert!(Rc::ptr_eq(
        &definition,
        &second.resolved_property_columns.borrow()["value"][0].definition
    ));
    assert!(Rc::ptr_eq(
        &first.resolved_property_columns.borrow()["width"][0].definition,
        &second.resolved_property_columns.borrow()["width"][0].definition,
    ));
    amount.set(33.0);
    assert_eq!((values(&first).0, values(&second).0), (33.0, 22.0));
    classes.set(String::new());
    ctx.drain_node_effects();
    assert_eq!(values(&first).0, 7.0);
    classes.set("active".into());
    ctx.drain_node_effects();
    assert!(Rc::ptr_eq(
        &definition,
        &first.resolved_property_columns.borrow()["value"][0].definition
    ));
    assert!(blocks[1].columns.get().is_none());
    root.recurse_unmount(ctx);
}

#[test]
fn replacement_template_has_fresh_definition_caches() {
    let (engine, root) = fixture();
    let make_plan = |value| {
        plan(
            vec![setting("value", literal(value))],
            None,
            Some(vec![SettingsBlockElement::SelectorBlock(
                pax_manifest::Token::new_without_location("Probe".into()),
                LiteralBlockDefinition::new(vec![setting("width", expression("40px"))]),
            )]),
        )
    };
    let old = make_plan(11.0);
    let node = child(
        &root,
        &engine.runtime_context,
        template(&old),
        HashMap::new(),
    );
    let previous = node.resolved_property_columns.borrow().clone();
    let replacement = make_plan(22.0);
    assert!(replacement.inline_columns.get().is_none());
    assert!(
        replacement.component_settings.as_ref().unwrap().0.selectors[0]
            .columns
            .get()
            .is_none()
    );
    node.recreate_with_new_data(template(&replacement), &engine.runtime_context);
    assert_eq!(values(&node).0, 22.0);
    let current = node.resolved_property_columns.borrow();
    for name in ["value", "width"] {
        assert!(!Rc::ptr_eq(
            &previous[name][0].definition,
            &current[name][0].definition
        ));
    }
    assert!(old.inline_columns.get().is_some());
    assert!(replacement.inline_columns.get().is_some());
    drop(current);
    root.recurse_unmount(&engine.runtime_context);
}

#[test]
fn selector_changes_resolve_once_per_rebind_and_reset_removed_component_properties() {
    let (engine, root) = fixture();
    let classes = LocalProperty::new("active".to_string());
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
    let original_columns = node.resolved_property_columns.borrow().clone();
    engine.runtime_context.drain_node_effects();
    assert_eq!(plan.resolutions.get(), 1);
    classes.set(String::new());
    engine.runtime_context.drain_node_effects();
    assert_eq!(plan.resolutions.get(), 2);
    assert_eq!(values(&node), (7.0, 19.0));
    assert!(node.resolved_property_columns.borrow()["value"].is_empty());
    assert_eq!(
        original_columns["value"].len(),
        1,
        "retained snapshots are immutable"
    );
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
fn prepared_conditional_selectors_keep_values_and_matching_node_local() {
    use pax_manifest::{SettingsConditionalBlock, SettingsConditionalBranch, Token};
    let (engine, root) = fixture();
    let block = |value| {
        SettingsBlockElement::SelectorBlock(
            Token::new_without_location(".active".into()),
            LiteralBlockDefinition::new(vec![setting("value", literal(value))]),
        )
    };
    let branch = |condition: Option<&str>, elements| SettingsConditionalBranch {
        condition_expression: condition.map(|text| {
            let ValueDefinition::Expression(expression) = expression(text) else {
                unreachable!()
            };
            expression
        }),
        elements,
    };
    let selectors = vec![SettingsBlockElement::Conditional(
        SettingsConditionalBlock {
            branches: vec![
                branch(
                    Some("wide"),
                    vec![SettingsBlockElement::Conditional(
                        SettingsConditionalBlock {
                            branches: vec![
                                branch(Some("selected"), vec![block(10.0)]),
                                branch(None, vec![block(20.0)]),
                            ],
                        },
                    )],
                ),
                branch(Some("selected"), vec![block(30.0)]),
                branch(None, vec![block(40.0)]),
            ],
        },
    )];
    let plan = plan(vec![], Some(expression("classes")), Some(selectors));
    let instance = template(&plan);
    let wide = LocalProperty::new(true);
    let selected = LocalProperty::new(true);
    let classes = LocalProperty::new("active".to_string());
    let first = child(
        &root,
        &engine.runtime_context,
        instance.clone(),
        HashMap::from([
            (
                "wide".into(),
                Variable::new_from_typed_property(wide.clone()),
            ),
            (
                "selected".into(),
                Variable::new_from_typed_property(selected.clone()),
            ),
            (
                "classes".into(),
                Variable::new_from_typed_property(classes.clone()),
            ),
        ]),
    );
    let second = child(
        &root,
        &engine.runtime_context,
        instance,
        HashMap::from([
            (
                "wide".into(),
                Variable::new_from_typed_property(LocalProperty::new(false)),
            ),
            (
                "selected".into(),
                Variable::new_from_typed_property(LocalProperty::new(true)),
            ),
            (
                "classes".into(),
                Variable::new_from_typed_property(LocalProperty::new("active".to_string())),
            ),
        ]),
    );
    engine.runtime_context.drain_node_effects();
    assert_eq!((values(&first).0, values(&second).0), (10.0, 30.0));
    let resolutions = plan.resolutions.get();
    selected.set(false);
    assert_eq!((values(&first).0, values(&second).0), (20.0, 30.0));
    wide.set(false);
    assert_eq!(values(&first).0, 40.0);
    selected.set(true);
    assert_eq!(values(&first).0, 30.0);
    assert_eq!(
        plan.resolutions.get(),
        resolutions,
        "conditions remain reactive without re-resolving selectors"
    );
    classes.set(String::new());
    engine.runtime_context.drain_node_effects();
    assert_eq!((values(&first).0, values(&second).0), (7.0, 30.0));
    assert!(!Rc::ptr_eq(
        &first.resolved_property_columns.borrow(),
        &second.resolved_property_columns.borrow()
    ));
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
        let source = LocalProperty::new(31.0);
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
    let theme = LocalProperty::new(8.0);
    root.imported_settings_layers
        .borrow_mut()
        .push(RuntimeSettingsLayer {
            transition: None,
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
            )].into(),
        });
    let plan = plan(vec![setting("value", expression("@base + 1"))], None, None);
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

fn settings_import(root: &Rc<ExpandedNode>, ctx: &Rc<RuntimeContext>) -> Rc<ExpandedNode> {
    let mut args = args();
    args.template_node_type_id = Some(TypeId::build_singleton(
        "test::ImportSettings",
        Some("ImportSettings"),
    ));
    child(root, ctx, Leaf::instantiate(args), HashMap::new())
}

fn settings_provider(
    root: &Rc<ExpandedNode>,
    ctx: &Rc<RuntimeContext>,
    theme: &LocalProperty<f64>,
) -> Rc<ExpandedNode> {
    let mut args = args();
    args.template_node_type_id = Some(TypeId::build_singleton("test::Theme", Some("Theme")));
    args.component_settings = Some(vec![SettingsBlockElement::SelectorBlock(
        pax_manifest::Token::new_without_location("Probe".into()),
        LiteralBlockDefinition::new(vec![setting("value", expression("theme"))]),
    )]);
    child(
        root,
        ctx,
        ComponentInstance::instantiate(args),
        HashMap::from([(
            "theme".into(),
            Variable::new_from_typed_property(theme.clone()),
        )]),
    )
}

#[test]
fn imported_settings_rebind_once_and_preserve_live_selector_layout_and_state() {
    let (engine, root) = fixture();
    let ctx = &engine.runtime_context;
    let classes = LocalProperty::new(String::new());
    let receiver = plan(
        vec![setting("value", expression("@base + 1"))],
        Some(expression("classes")),
        Some(vec![SettingsBlockElement::SelectorBlock(
            pax_manifest::Token::new_without_location(".wide".into()),
            LiteralBlockDefinition::new(vec![setting("width", expression("80px"))]),
        )]),
    );
    let import = settings_import(&root, ctx);
    let node = child(
        &root,
        ctx,
        template(&receiver),
        HashMap::from([(
            "classes".into(),
            Variable::new_from_typed_property(classes.clone()),
        )]),
    );
    let children =
        root.attach_children(vec![import.clone(), node.clone()], ctx, &root.parent_frame);
    root.children.set(children);
    ctx.drain_node_effects();
    node.with_properties_unwrapped(|p: &mut Probe| p.untouched.set(55.0));
    let slot = node.properties_scope.borrow()["value"]
        .get_untyped_property()
        .get_id();
    let selector = node.selector_metadata.borrow().classes.untyped().get_id();
    let layout_rebuilds = node.parent_binding_rebuilds.get();
    let mounts = MOUNTS.with(|m| m.borrow().len());
    let resolutions = receiver.resolutions.get();
    let theme = LocalProperty::new(8.0);
    let mut theme_args = args();
    theme_args.component_settings = Some(vec![SettingsBlockElement::SelectorBlock(
        pax_manifest::Token::new_without_location("Probe".into()),
        LiteralBlockDefinition::new(vec![
            setting("value", expression("theme")),
            setting("height", expression("(theme)px")),
        ]),
    )]);
    let provider = child(
        &root,
        ctx,
        ComponentInstance::instantiate(theme_args),
        HashMap::from([(
            "theme".into(),
            Variable::new_from_typed_property(theme.clone()),
        )]),
    );
    import.attach_sidecar_children(vec![provider], ctx, &import.parent_frame);
    ctx.drain_node_effects();
    assert_eq!(receiver.resolutions.get(), resolutions + 1);
    assert_eq!(values(&node), (9.0, 55.0));
    assert_eq!(node.transform_and_bounds.get().bounds.1, 8.0);
    assert_eq!(
        node.properties_scope.borrow()["value"]
            .get_untyped_property()
            .get_id(),
        slot
    );
    assert_eq!(
        node.selector_metadata.borrow().classes.untyped().get_id(),
        selector
    );
    assert_eq!(node.parent_binding_rebuilds.get(), layout_rebuilds);
    assert_eq!(MOUNTS.with(|m| m.borrow().len()), mounts);

    theme.set(13.0);
    classes.set("wide".into());
    ctx.drain_node_effects();
    assert_eq!(values(&node), (14.0, 55.0));
    assert_eq!(node.transform_and_bounds.get().bounds, (80.0, 13.0));
    import.attach_sidecar_children(vec![], ctx, &import.parent_frame);
    ctx.drain_node_effects();
    assert_eq!(values(&node), (8.0, 55.0));
    classes.set(String::new());
    ctx.drain_node_effects();
    assert_eq!(
        node.get_common_properties().as_ref().borrow().width.get(),
        None
    );
    assert_eq!(node.parent_binding_rebuilds.get(), layout_rebuilds);
    root.recurse_unmount(ctx);
    ctx.drain_node_effects();
}

#[test]
fn imported_settings_keep_custom_factory_rebind_order() {
    let (engine, root) = fixture();
    let ctx = &engine.runtime_context;
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut custom = args();
    let events = calls.clone();
    custom.prototypical_properties = PropertiesInit::Factory(Box::new(move |_, node| {
        if node.is_some() {
            events.borrow_mut().push("bind");
            None
        } else {
            Some(Rc::new(RefCell::new(Probe::default().to_pax_any())))
        }
    }));
    let events = calls.clone();
    custom.properties_scope = PropertiesScopeInit::Factory(Box::new(move |properties| {
        events.borrow_mut().push("scope");
        let properties = properties.as_ref().borrow();
        build_property_scope(Probe::ref_from_pax_any(&properties).unwrap(), &SCOPE)
    }));
    let import = settings_import(&root, ctx);
    let node = child(&root, ctx, Leaf::instantiate(custom), HashMap::new());
    root.children.set(vec![import.clone(), node]);
    calls.borrow_mut().clear();
    let provider = settings_provider(&root, ctx, &LocalProperty::new(8.0));
    import.attach_sidecar_children(vec![provider], ctx, &import.parent_frame);
    root.sync_imported_settings(ctx);
    assert_eq!(
        *calls.as_ref().borrow(),
        ["bind", "scope", "scope", "bind", "scope"]
    );
    root.recurse_unmount(ctx);
    ctx.drain_node_effects();
}

#[test]
fn imported_settings_refresh_typed_alias_scope_and_occlusion_dependencies() {
    let (engine, root) = fixture();
    let ctx = &engine.runtime_context;
    let receiver = plan(vec![], None, None);
    let node = child(&root, ctx, template(&receiver), HashMap::new());
    let import = settings_import(&root, ctx);
    root.children.set(vec![import.clone(), node.clone()]);
    let make_provider = |source: &LocalProperty<f64>| {
        let mut provider = args();
        provider.component_settings = Some(vec![SettingsBlockElement::SelectorBlock(
            pax_manifest::Token::new_without_location("Probe".into()),
            LiteralBlockDefinition::new(vec![setting(
                "value",
                ValueDefinition::DoubleBinding(pax_manifest::PaxIdentifier::new("theme")),
            )]),
        )]);
        child(
            &root,
            ctx,
            ComponentInstance::instantiate(provider),
            HashMap::from([(
                "theme".into(),
                Variable::new_from_typed_property(source.clone()),
            )]),
        )
    };
    let first = LocalProperty::new(11.0);
    let second = LocalProperty::new(22.0);
    for (index, source) in [&first, &second].into_iter().enumerate() {
        import.attach_sidecar_children(vec![make_provider(source)], ctx, &import.parent_frame);
        ctx.drain_node_effects();
        assert_eq!(receiver.resolutions.get(), index + 2);
        assert_eq!(
            node.properties_scope.borrow()["value"]
                .get_untyped_property()
                .get_id(),
            source.untyped().get_id()
        );
        assert_eq!(values(&node).0, source.get());
        ctx.take_occlusion_dirty();
        source.set(source.get() + 1.0);
        ctx.drain_node_effects();
        assert!(
            ctx.take_occlusion_dirty(),
            "occlusion observes the replacement alias"
        );
    }
    first.set(90.0);
    ctx.drain_node_effects();
    assert_eq!(values(&node).0, 23.0);
    root.recurse_unmount(ctx);
    ctx.drain_node_effects();
}

#[test]
fn unchanged_import_discovery_does_not_copy_settings_or_rebind() {
    let (engine, root) = fixture();
    let ctx = &engine.runtime_context;
    let theme = LocalProperty::new(8.0);
    let import = settings_import(&root, ctx);
    let provider = settings_provider(&root, ctx, &theme);
    *import.sidecar_children.borrow_mut() = vec![provider.clone()];
    root.children.set(vec![import.clone()]);
    root.sync_imported_settings(ctx);
    let receiver_plan = plan(vec![setting("value", expression("@base + 1"))], None, None);
    let node = child(&root, ctx, template(&receiver_plan), HashMap::new());
    root.children.set(vec![import, node.clone()]);
    assert_eq!(values(&node).0, 9.0);
    let binds = BINDS.with(Cell::get);
    let copies = provider.settings_layer_materializations.get();
    let scope = root.imported_settings_layers.borrow()[0]
        .provider_stack
        .clone();
    let definition = provider
        .instance_node
        .borrow()
        .base()
        .component_settings
        .clone()
        .unwrap();
    assert!(Rc::ptr_eq(
        &definition.0,
        &root.imported_settings_layers.borrow()[0].settings.0
    ));
    let cloned_layer = root.imported_settings_layers.borrow()[0].clone();
    assert!(Rc::ptr_eq(&definition.0, &cloned_layer.settings.0));
    for _ in 0..100 {
        root.sync_imported_settings(ctx);
    }
    assert_eq!(provider.settings_layer_materializations.get(), copies);
    assert_eq!(BINDS.with(Cell::get), binds);
    assert!(Rc::ptr_eq(
        &scope,
        &root.imported_settings_layers.borrow()[0].provider_stack
    ));
    theme.set(13.0);
    root.sync_imported_settings(ctx);
    assert_eq!(values(&node).0, 14.0);
    assert_eq!(provider.settings_layer_materializations.get(), copies);

    let reloaded = plan(vec![setting("value", expression("@base + 2"))], None, None);
    node.recreate_with_new_data(template(&reloaded), ctx);
    root.sync_imported_settings(ctx);
    assert_eq!(values(&node).0, 15.0);
    theme.set(20.0);
    assert_eq!(values(&node).0, 22.0);
    assert_eq!(provider.settings_layer_materializations.get(), copies);
    root.recurse_unmount(ctx);
    ctx.drain_node_effects();
}

#[test]
fn imported_settings_discovery_is_dependency_driven() {
    let (mut engine, root) = fixture();
    let ctx = engine.runtime_context.clone();
    let theme = LocalProperty::new(8.0);
    let import = settings_import(&root, &ctx);
    let provider = settings_provider(&root, &ctx, &theme);
    import.attach_sidecar_children(vec![provider.clone()], &ctx, &import.parent_frame);
    root.children.set(vec![import.clone()]);
    root.sync_imported_settings(&ctx);
    let node = child(
        &root,
        &ctx,
        template(&plan(vec![], None, None)),
        HashMap::new(),
    );
    root.children.set(vec![import.clone(), node.clone()]);
    ctx.drain_node_effects();
    let discoveries = root.settings_discoveries.get();
    let copies = provider.settings_layer_materializations.get();
    for _ in 0..100 {
        engine.tick();
    }
    assert_eq!(
        root.settings_discoveries.get(),
        discoveries,
        "steady ticks do not traverse imports"
    );
    theme.set(13.0);
    ctx.drain_node_effects();
    assert_eq!(values(&node).0, 13.0);
    assert_eq!(
        root.settings_discoveries.get(),
        discoveries,
        "provider values use their existing DAG edges"
    );
    assert_eq!(provider.settings_layer_materializations.get(), copies);

    let replacement = settings_provider(&root, &ctx, &LocalProperty::new(21.0));
    import.attach_sidecar_children(vec![replacement], &ctx, &import.parent_frame);
    ctx.drain_node_effects();
    assert_eq!(values(&node).0, 21.0);
    // A source retained outside the tree must no longer invalidate this owner.
    let discoveries = root.settings_discoveries.get();
    provider.children.set(vec![]);
    ctx.drain_node_effects();
    assert_eq!(root.settings_discoveries.get(), discoveries);
    import.attach_sidecar_children(vec![], &ctx, &import.parent_frame);
    ctx.drain_node_effects();
    assert_eq!(values(&node).0, 7.0);
    root.recurse_unmount(&ctx);
    ctx.drain_node_effects();
}

#[test]
fn import_observer_follows_nested_structure_and_same_identity_provider_reload() {
    let (engine, root) = fixture();
    let ctx = &engine.runtime_context;
    let import = settings_import(&root, ctx);
    let branch = child(&root, ctx, Leaf::instantiate(args()), HashMap::new());
    import.attach_sidecar_children(vec![branch.clone()], ctx, &import.parent_frame);
    root.children.set(vec![import.clone()]);
    root.sync_imported_settings(ctx);
    let node = child(
        &root,
        ctx,
        template(&plan(vec![], None, None)),
        HashMap::new(),
    );
    root.children.set(vec![import, node.clone()]);
    ctx.drain_node_effects();
    assert_eq!(values(&node).0, 7.0);
    let theme = LocalProperty::new(5.0);
    let provider = settings_provider(&root, ctx, &theme);
    branch.children.set(vec![provider.clone()]);
    ctx.drain_node_effects();
    assert_eq!(values(&node).0, 5.0);
    let old_definition = root.imported_settings_layers.borrow()[0].settings.clone();
    let mut replacement = args();
    replacement.template_node_type_id = Some(TypeId::build_singleton("test::Theme", Some("Theme")));
    replacement.component_settings = Some(vec![SettingsBlockElement::SelectorBlock(
        pax_manifest::Token::new_without_location("Probe".into()),
        LiteralBlockDefinition::new(vec![setting("value", expression("theme * 2"))]),
    )]);
    provider.recreate_with_new_data(ComponentInstance::instantiate(replacement), ctx);
    ctx.drain_node_effects();
    assert!(!Rc::ptr_eq(
        &old_definition.0,
        &root.imported_settings_layers.borrow()[0].settings.0
    ));
    assert_eq!(
        values(&node).0,
        10.0,
        "same provider id and type must still refresh edited settings"
    );
    let reloaded = provider.instance_node.borrow().clone();
    provider.fully_recreate_with_new_data(reloaded, ctx);
    ctx.drain_node_effects();
    assert_eq!(values(&node).0, 10.0, "remount retains import observation");
    theme.set(6.0);
    ctx.drain_node_effects();
    assert_eq!(values(&node).0, 12.0);
    branch.children.set(vec![]);
    ctx.drain_node_effects();
    assert_eq!(values(&node).0, 7.0);
    root.recurse_unmount(ctx);
    ctx.drain_node_effects();
}

#[test]
fn import_discovery_preserves_provider_order_replacement_and_removal() {
    let (engine, root) = fixture();
    let ctx = &engine.runtime_context;
    let first = settings_provider(&root, ctx, &LocalProperty::new(10.0));
    let second = settings_provider(&root, ctx, &LocalProperty::new(20.0));
    let import = settings_import(&root, ctx);
    *import.sidecar_children.borrow_mut() = vec![first.clone(), second.clone()];
    root.children.set(vec![import.clone()]);
    root.sync_imported_settings(ctx);
    let plan = plan(vec![], None, None);
    let node = child(&root, ctx, template(&plan), HashMap::new());
    root.children.set(vec![import.clone(), node.clone()]);
    assert_eq!(values(&node).0, 20.0);
    import.attach_sidecar_children(vec![second, first], ctx, &import.parent_frame);
    ctx.drain_node_effects();
    assert_eq!(values(&node).0, 10.0);
    let replacement = settings_provider(&root, ctx, &LocalProperty::new(30.0));
    import.attach_sidecar_children(vec![replacement], ctx, &import.parent_frame);
    ctx.drain_node_effects();
    assert_eq!(values(&node).0, 30.0);
    import.attach_sidecar_children(vec![], ctx, &import.parent_frame);
    ctx.drain_node_effects();
    assert_eq!(values(&node).0, 7.0);
    assert!(root.imported_settings_layers.borrow().is_empty());
    root.recurse_unmount(ctx);
    ctx.drain_node_effects();
}

#[test]
fn import_discovery_keeps_transition_policy_reactive() {
    let (engine, root) = fixture();
    let ctx = &engine.runtime_context;
    let theme = LocalProperty::new(0.0);
    let policy = LocalProperty::new(None);
    let import = settings_import(&root, ctx);
    *import.import_settings_transition.borrow_mut() = Some(policy.clone());
    let provider = settings_provider(&root, ctx, &theme);
    *import.sidecar_children.borrow_mut() = vec![provider.clone()];
    root.children.set(vec![import.clone()]);
    root.sync_imported_settings(ctx);
    let node = child(
        &root,
        ctx,
        template(&plan(vec![], None, None)),
        HashMap::new(),
    );
    root.children.set(vec![import, node.clone()]);
    ctx.drain_node_effects();
    assert_eq!(values(&node).0, 0.0);

    policy.set(Some(SettingsTransitionConfig {
        duration: Duration::Milliseconds(100.into()),
        curve: "Linear",
    }));
    ctx.drain_node_effects();
    let copies = provider.settings_layer_materializations.get();
    assert_eq!(copies, 2);
    theme.set(10.0);
    ctx.drain_node_effects();
    ctx.globals().elapsed_millis.set(40);
    assert_eq!(values(&node).0, 4.0);

    policy.set(Some(SettingsTransitionConfig {
        duration: Duration::Milliseconds(200.into()),
        curve: "Linear",
    }));
    theme.set(20.0);
    ctx.drain_node_effects();
    ctx.globals().elapsed_millis.set(140);
    assert_eq!(values(&node).0, 12.0);
    assert_eq!(provider.settings_layer_materializations.get(), copies);

    policy.set(None);
    ctx.drain_node_effects();
    assert_eq!(values(&node).0, 20.0);
    assert_eq!(provider.settings_layer_materializations.get(), copies + 1);
    theme.set(30.0);
    ctx.drain_node_effects();
    assert_eq!(values(&node).0, 30.0);
    root.recurse_unmount(ctx);
    ctx.drain_node_effects();
}

fn animated_layer(
    root: &ExpandedNode,
    theme: &LocalProperty<f64>,
    settings: Vec<SettingElement>,
) -> RuntimeSettingsLayer {
    RuntimeSettingsLayer {
        provider_id: root.id,
        provider_type_id: TypeId::build_singleton("test::Theme", Some("Theme")),
        provider_stack: RuntimePropertiesStackFrame::new(HashMap::from([(
            "theme".into(),
            Variable::new_from_typed_property(theme.clone()),
        )])),
        settings: vec![SettingsBlockElement::SelectorBlock(
            pax_manifest::Token::new_without_location("Probe".into()),
            LiteralBlockDefinition::new(settings),
        )].into(),
        transition: Some(LocalProperty::new(Some(SettingsTransitionConfig {
            duration: Duration::Milliseconds(100.into()),
            curve: "Linear",
        }))),
    }
}

// Models the ImportSettings primitive's sidecar provider without depending on
// pax-std (which itself depends on this crate).
struct ThemeImport {
    base: BaseInstance,
}
impl InstanceNode for ThemeImport {
    fn instantiate(mut args: InstantiationArgs) -> Rc<Self> {
        args.template_node_type_id = Some(TypeId::build_singleton(
            "test::ImportSettings",
            Some("ImportSettings"),
        ));
        Rc::new(Self {
            base: BaseInstance::new(
                args,
                InstanceFlags {
                    layer: Layer::DontCare,
                    invisible_to_slot: true,
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
    fn handle_mount(self: Rc<Self>, node: &Rc<ExpandedNode>, ctx: &Rc<RuntimeContext>) {
        *node.import_settings_transition.borrow_mut() =
            Some(LocalProperty::new(Some(SettingsTransitionConfig {
                duration: Duration::Milliseconds(100.into()),
                curve: "Linear",
            })));
        let mut provider_args = args();
        provider_args.template_node_type_id =
            Some(TypeId::build_singleton("test::Theme", Some("Theme")));
        provider_args.component_settings = Some(vec![SettingsBlockElement::SelectorBlock(
            pax_manifest::Token::new_without_location("Probe".into()),
            LiteralBlockDefinition::new(vec![setting("value", expression("untouched"))]),
        )]);
        let provider = child(
            node,
            ctx,
            ComponentInstance::instantiate(provider_args),
            HashMap::new(),
        );
        node.attach_sidecar_children(vec![provider], ctx, &node.parent_frame);
    }
    fn resolve_debug(
        &self,
        f: &mut std::fmt::Formatter,
        _: Option<&ExpandedNode>,
    ) -> std::fmt::Result {
        f.write_str("ThemeImport")
    }
}

#[test]
fn late_mounted_components_resolve_their_theme_before_the_first_presentation() {
    use crate::constants::{PRE_RENDER_HANDLERS, TICK_HANDLERS};

    fn open_detail(properties: Rc<RefCell<PaxAny>>, _: &NodeContext, _: Option<PaxAny>) {
        Probe::ref_from_pax_any(&properties.as_ref().borrow())
            .unwrap()
            .value
            .set(1.0);
    }

    for resumable in [false, true] {
    for phase in [TICK_HANDLERS, PRE_RENDER_HANDLERS, "frame", "millis"] {
        let clock = Rc::new(Cell::new(0_u128));
        let clock_read = clock.clone();
        let mut engine = crate::test_support::empty_engine(
            (320.0, 240.0),
            Platform::Web,
            OS::Mac,
            Box::new(move || clock_read.get()),
            Default::default(),
        );
        if resumable { engine.enable_resumable_startup(); }
        let mut root_args = args();
        let mut registry = HandlerRegistry::default();
        registry
            .handlers
            .insert(phase.into(), vec![Handler::new_inline_handler(open_detail)]);
        root_args.handler_registry = Some(Rc::new(RefCell::new(registry)));
        let root = engine.mount_root_component(ComponentInstance::instantiate(root_args));
        let context = engine.runtime_context.clone();
        let open = match phase {
            "frame" => {
                let frames = context.globals().elapsed_frames;
                let dependencies = [frames.untyped()];
                LocalProperty::computed(
                    move || if frames.get() > 0 { 1.0 } else { 0.0 },
                    &dependencies,
                )
            }
            "millis" => {
                let millis = context.globals().elapsed_millis;
                let dependencies = [millis.untyped()];
                LocalProperty::computed(
                    move || if millis.get() > 0 { 1.0 } else { 0.0 },
                    &dependencies,
                )
            }
            _ => root.with_properties_unwrapped(|p: &mut Probe| p.value.clone()),
        };
        let detail = Rc::new(RefCell::new(None));
        let effect = {
            let root = Rc::downgrade(&root);
            let weak_context = Rc::downgrade(&context);
            let detail = detail.clone();
            let dependencies = [open.untyped()];
            context.register_node_effect(root.upgrade().unwrap().id, &dependencies, move || {
                if open.get() != 1.0 || detail.as_ref().borrow().is_some() {
                    return;
                }
                let (Some(root), Some(context)) = (root.upgrade(), weak_context.upgrade()) else {
                    return;
                };
                let mut detail_args = args();
                detail_args.component_template = Some(RefCell::new(vec![
                    ThemeImport::instantiate(args()) as Rc<dyn InstanceNode>,
                    template(&plan(vec![], None, None)),
                ]));
                let nodes = root.generate_children(
                    [(
                        ComponentInstance::instantiate(detail_args) as Rc<dyn InstanceNode>,
                        context.globals().stack_frame(),
                    )],
                    &context,
                    &root.parent_frame,
                    true,
                );
                *detail.as_ref().borrow_mut() = Some(Rc::downgrade(&nodes[0]));
                root.children.set(nodes);
            })
        };
        clock.set(10);
        if resumable {
            for _ in 0..10000 {
                if !engine.startup_pending() { break; }
                engine.service_startup(0., || 0.);
            }
            assert!(!engine.startup_pending());
        } else { engine.tick(); }
        let detail = detail
            .as_ref()
            .borrow()
            .as_ref()
            .and_then(std::rc::Weak::upgrade)
            .expect("late handler mounted detail");
        let children = detail.children.get();
        let receiver = &children[1];
        assert_eq!(values(receiver).0, 19.0, "first presentation after {phase}");
        engine.tick();
        assert_eq!(
            values(receiver).0,
            19.0,
            "no default-to-theme fade after {phase}"
        );

        // A later provider update must retain the ordinary animated behavior.
        children[0].sidecar_children.borrow()[0]
            .with_properties_unwrapped(|p: &mut Probe| p.untouched.set(27.0));
        context.drain_node_effects();
        assert_eq!(values(receiver).0, 19.0);
        context.globals().elapsed_millis.set(60);
        assert_eq!(
            values(receiver).0,
            23.0,
            "theme changes still animate after {phase}"
        );
        drop(effect);
        engine.unmount();
    }
    }
}

#[test]
fn imported_motion_survives_rebinding_and_removal_for_common_and_component_properties() {
    let (engine, root) = fixture();
    let context = &engine.runtime_context;
    let theme = LocalProperty::new(0.0);
    root.imported_settings_layers
        .borrow_mut()
        .push(animated_layer(
            &root,
            &theme,
            vec![
                setting("value", expression("theme")),
                setting("x", expression("(theme)px")),
            ],
        ));
    let plan = plan(vec![], None, None);
    let node = child(&root, context, template(&plan), HashMap::new());
    assert_eq!(values(&node).0, 0.0);
    let common = node.common_properties.borrow().clone();
    let x = common.as_ref().borrow().x.clone();
    let value_id = node.with_properties_unwrapped(|p: &mut Probe| p.value.untyped().get_id());
    theme.set(10.0);
    context.drain_node_effects();
    context.globals().elapsed_millis.set(40);
    theme.set(20.0);
    node.instance_node.borrow().base().bind_properties(&node);
    context.drain_node_effects();
    assert_eq!(values(&node).0, 4.0);
    assert_eq!(x.get().unwrap().get_pixels(100.0), 4.0);
    assert_eq!(
        value_id,
        node.with_properties_unwrapped(|p: &mut Probe| p.value.untyped().get_id())
    );
    context.globals().elapsed_millis.set(90);
    assert_eq!(values(&node).0, 12.0);
    assert_eq!(x.get().unwrap().get_pixels(100.0), 12.0);
    root.imported_settings_layers.borrow_mut().clear();
    node.reset_removed_runtime_properties.set(true);
    node.instance_node.borrow().base().bind_properties(&node);
    context.drain_node_effects();
    context.globals().elapsed_millis.set(140);
    assert_eq!(values(&node).0, 9.5); // Probe's default is 7.
    assert_eq!(x.get().unwrap().get_pixels(100.0), 6.0); // Unset x resolves to zero.
    context.globals().elapsed_millis.set(190);
    assert_eq!(values(&node).0, 7.0);
    assert!(x.get().is_none());
}

#[test]
fn explicit_settings_win_without_allocating_motion_and_animated_nodes_release_state() {
    let (engine, root) = fixture();
    let context = &engine.runtime_context;
    let theme = LocalProperty::new(10.0);
    root.imported_settings_layers
        .borrow_mut()
        .push(animated_layer(
            &root,
            &theme,
            vec![setting("value", expression("theme"))],
        ));
    let explicit = plan(vec![setting("value", literal(42.0))], None, None);
    let node = child(&root, context, template(&explicit), HashMap::new());
    assert_eq!(values(&node).0, 42.0);
    assert!(node.settings_motion.as_ref().borrow().is_empty());
    drop(node);
    context.drain_node_effects();
    let initial = pax_runtime_api::properties::property_table_total_properties_count();
    let animated = plan(vec![], None, None);
    for _ in 0..8 {
        let node = child(&root, context, template(&animated), HashMap::new());
        theme.update(|value| *value += 1.0);
        context.drain_node_effects();
        assert!(!node.settings_motion.as_ref().borrow().is_empty());
        drop(node);
        context.drain_node_effects();
    }
    assert_eq!(
        pax_runtime_api::properties::property_table_total_properties_count(),
        initial
    );
}
impl DefinitionToInstanceTraverser for Traverser {
    #[cfg(feature = "designtime")]
    fn get_designtime_manager(
        &self,
        _: String,
    ) -> Option<Rc<RefCell<pax_designtime::DesigntimeManager>>> {
        Some(Rc::new(RefCell::new(
            pax_designtime::DesigntimeManager::new_offline(self.0.borrow().clone()),
        )))
    }
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
    use pax_manifest::{
        ComponentDefinition, ComponentTemplate, PaxManifest, SettingsConditionalBlock,
        SettingsConditionalBranch, Token,
    };
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
    let mut rich = PaxManifest {
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
    let height_selector = |height| {
        SettingsBlockElement::SelectorBlock(
            Token::new_without_location("Probe".into()),
            LiteralBlockDefinition::new(vec![setting("height", expression(height))]),
        )
    };
    let ValueDefinition::Expression(enabled) = expression("enabled") else {
        unreachable!()
    };
    rich.components.get_mut(&root_type).unwrap().settings =
        Some(vec![SettingsBlockElement::Conditional(
            SettingsConditionalBlock {
                branches: vec![
                    SettingsConditionalBranch {
                        condition_expression: Some(enabled),
                        elements: vec![height_selector("50px")],
                    },
                    SettingsConditionalBranch {
                        condition_expression: None,
                        elements: vec![height_selector("70px")],
                    },
                ],
            },
        )]);
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
        let enabled = LocalProperty::new(true);
        let node = child(&root, &engine.runtime_context, instance, HashMap::from([
            ("enabled".into(), Variable::new_from_typed_property(enabled.clone())),
        ]));
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
        assert_eq!(node.get_common_properties().as_ref().borrow().height.get(), Some(Size::Pixels(50.into())));
        enabled.set(false);
        assert_eq!(node.get_common_properties().as_ref().borrow().height.get(), Some(Size::Pixels(70.into())));
        assert_eq!(
            node.get_common_properties().as_ref().borrow().width.get(),
            Some(Size::Pixels(80.into()))
        );
        let ctx = &engine.runtime_context;
        let import = settings_import(&root, ctx);
        let provider = settings_provider(&root, ctx, &LocalProperty::new(20.0));
        import.attach_sidecar_children(vec![provider], ctx, &import.parent_frame);
        root.children.set(vec![import, node.clone()]);
        root.sync_imported_settings(ctx);
        assert_eq!(plan.resolutions.get(), 2);
        assert_eq!(values(&node), (42.0, 19.0), "inline values still win");
        traverser.0.borrow_mut().components.get_mut(&root_type).unwrap().settings =
            Some(vec![height_selector("90px")]);
        let replacement = traverser.build_template_node(&root_type, &leaf_id.get_template_node_id(), None);
        node.recreate_with_new_data(replacement, ctx);
        assert_eq!(node.get_common_properties().as_ref().borrow().height.get(), Some(Size::Pixels(90.into())));
        assert_eq!(values(&node), (42.0, 19.0));
        root.recurse_unmount(ctx);
        ctx.drain_node_effects();
    }
}

#[test]
fn unrelated_import_preserves_bindings_scopes_and_written_common_state() {
    let (engine, root) = fixture();
    let ctx = &engine.runtime_context;
    let import = settings_import(&root, ctx);
    let receiver = plan(vec![setting("width", expression("40px"))], None, None);
    let node = child(&root, ctx, template(&receiver), HashMap::new());
    root.children.set(vec![import.clone(), node.clone()]);
    ctx.drain_node_effects();
    let mut provider_args = args();
    provider_args.component_settings = Some(vec![SettingsBlockElement::SelectorBlock(
        pax_manifest::Token::new_without_location("OtherType".into()),
        LiteralBlockDefinition::new(vec![setting("value", literal(9.))]),
    )]);
    let provider = child(
        &root,
        ctx,
        ComponentInstance::instantiate(provider_args),
        HashMap::new(),
    );
    let common = node.get_common_properties();
    common
        .as_ref()
        .borrow()
        .width
        .set(Some(pax_runtime_api::Size::Pixels(88.into())));
    let binds = BINDS.with(Cell::get);
    let scopes = SCOPES.with(Cell::get);
    import.attach_sidecar_children(vec![provider], ctx, &import.parent_frame);
    ctx.drain_node_effects();
    assert_eq!(BINDS.with(Cell::get), binds);
    assert_eq!(SCOPES.with(Cell::get), scopes);
    assert_eq!(
        common
            .as_ref()
            .borrow()
            .width
            .get()
            .unwrap()
            .get_pixels(100.),
        88.
    );
    root.recurse_unmount(ctx);
}

#[test]
fn changed_import_preserves_unmodified_common_bindings_and_defaults() {
    let (engine, root) = fixture();
    let ctx = &engine.runtime_context;
    let import = settings_import(&root, ctx);
    let receiver = plan(vec![setting("width", expression("40px"))], None, None);
    let node = child(&root, ctx, template(&receiver), HashMap::new());
    root.children.set(vec![import.clone(), node.clone()]);
    ctx.drain_node_effects();
    let common = node.get_common_properties();
    common
        .as_ref()
        .borrow()
        .width
        .set(Some(pax_runtime_api::Size::Pixels(88.into())));
    common
        .as_ref()
        .borrow()
        .x
        .set(Some(pax_runtime_api::Size::Pixels(12.into())));
    let theme = LocalProperty::new(9.);
    let provider = settings_provider(&root, ctx, &theme);
    import.attach_sidecar_children(vec![provider], ctx, &import.parent_frame);
    ctx.drain_node_effects();
    assert_eq!(values(&node).0, 9.);
    assert_eq!(
        common
            .as_ref()
            .borrow()
            .width
            .get()
            .unwrap()
            .get_pixels(100.),
        88.
    );
    assert_eq!(
        common.as_ref().borrow().x.get().unwrap().get_pixels(100.),
        12.
    );
    theme.set(15.);
    assert_eq!(values(&node).0, 15.);
    import.attach_sidecar_children(vec![], ctx, &import.parent_frame);
    ctx.drain_node_effects();
    assert_eq!(values(&node).0, 7.);
    assert_eq!(
        common
            .as_ref()
            .borrow()
            .width
            .get()
            .unwrap()
            .get_pixels(100.),
        88.
    );
    root.recurse_unmount(ctx);
}
