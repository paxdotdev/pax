use super::*;
use pax_language::parse_pax_expression;

fn expression(raw: &str) -> ValueDefinition {
    Functions::register_all_functions();
    ValueDefinition::Expression(ExpressionInfo::new(parse_pax_expression(raw).unwrap()))
}

fn scope<T: PropertyValue + ToPaxValue>(source: &Property<T>) -> Rc<RuntimePropertiesStackFrame> {
    RuntimePropertiesStackFrame::new(HashMap::from([(
        "source".to_string(),
        Variable::new_from_typed_property(source.clone()),
    )]))
}

fn build<T: CoercionRules + PropertyValue + ToPaxValue>(
    definition: &ValueDefinition,
    stack: &Rc<RuntimePropertiesStackFrame>,
) -> Property<T> {
    build_component_property(
        "test",
        definition,
        stack,
        stack.clone(),
        |_, _| unreachable!(),
    )
}

#[test]
fn plain_identifiers_and_unadorned_groups_use_typed_forwarding() {
    let source = Property::new(vec![1.0_f64, 2.0]);
    let stack = scope(&source);
    for definition in [
        ValueDefinition::Identifier(PaxIdentifier::new("source")),
        expression("source"),
        expression("self.source"),
        expression("(self.source)"),
    ] {
        assert!(try_typed_property_binding::<Vec<f64>>("test", &definition, &stack).is_some());
        let destination = build::<Vec<f64>>(&definition, &stack);
        assert_ne!(source.untyped().get_id(), destination.untyped().get_id());
        assert_eq!(destination.get(), source.get());
        source.set(vec![3.0, 4.0]);
        assert_eq!(destination.get(), vec![3.0, 4.0]);
        destination.set(vec![9.0]);
        assert_eq!(source.get(), vec![3.0, 4.0]);
    }
}

#[test]
fn expressions_units_projection_and_coercions_keep_the_interpreter_path() {
    let source = Property::new(4.0_f64);
    let stack = scope(&source);
    for raw in ["source + 1", "(source)px", "[source]", "true ? source : 0"] {
        assert!(try_typed_property_binding::<f64>("test", &expression(raw), &stack).is_none());
    }
    assert_eq!(build::<f64>(&expression("source + 1"), &stack).get(), 5.0);
    assert_eq!(
        build::<Size>(&expression("(source)px"), &stack)
            .get()
            .expect_pixels(),
        Numeric::F64(4.0)
    );
    assert!(try_typed_property_binding::<Numeric>("test", &expression("source"), &stack).is_none());
    assert_eq!(
        build::<Numeric>(&expression("source"), &stack).get(),
        Numeric::F64(4.0)
    );
    let list = scope(&Property::new(vec![2.0_f64, 3.0]));
    assert!(try_typed_property_binding::<f64>("test", &expression("source[1]"), &list).is_none());
    assert_eq!(build::<f64>(&expression("source[1]"), &list).get(), 3.0);
    let record = scope(&Property::new(PaxValue::Object(vec![(
        "x".into(),
        7.0.to_pax_value(),
    )])));
    assert!(try_typed_property_binding::<f64>("test", &expression("source.x"), &record).is_none());
    assert_eq!(build::<f64>(&expression("source.x"), &record).get(), 7.0);
}

#[test]
fn common_property_exact_option_and_option_lifting_stay_distinct() {
    let source = Property::new(Some(Size::Pixels(3.0.into())));
    let stack = scope(&source);
    let definition = expression("source");
    assert!(try_typed_property_binding::<Option<Size>>("x", &definition, &stack).is_some());
    let destination = build_common_property_value::<Size>("x", &definition, &stack);
    assert_eq!(destination.get(), source.get());
    source.set(None);
    assert_eq!(destination.get(), None);
    destination.set(Some(Size::Pixels(9.0.into())));
    assert_eq!(source.get(), None);
    let plain = scope(&Property::new(Size::Pixels(5.0.into())));
    assert!(try_typed_property_binding::<Option<Size>>("x", &definition, &plain).is_none());
    assert_eq!(
        build_common_property_value::<Size>("x", &definition, &plain).get(),
        Some(Size::Pixels(5.0.into()))
    );
}

#[test]
fn actual_double_binding_still_aliases_the_parent() {
    let source = Property::new(4.0_f64);
    let definition = ValueDefinition::DoubleBinding(PaxIdentifier::new("source"));
    let destination = build::<f64>(&definition, &scope(&source));
    assert_eq!(source.untyped().get_id(), destination.untyped().get_id());
    destination.set(8.0);
    assert_eq!(source.get(), 8.0);
}

#[test]
fn custom_coercion_runs_even_for_the_same_rust_type() {
    #[derive(Default, Clone, Debug)]
    struct Normalized(f64);
    impl pax_runtime_api::Interpolatable for Normalized {}
    impl ToPaxValue for Normalized {
        fn to_pax_value(self) -> PaxValue {
            self.0.to_pax_value()
        }
    }
    impl CoercionRules for Normalized {
        fn try_coerce(value: PaxValue) -> Result<Self, String> {
            Ok(Self(f64::try_coerce(value)?.max(0.0)))
        }
    }
    let source = Property::new(Normalized(-1.0));
    let stack = scope(&source);
    let definition = expression("source");
    assert!(try_typed_property_binding::<Normalized>("test", &definition, &stack).is_none());
    assert_eq!(build::<Normalized>(&definition, &stack).get().0, 0.0);
    let nested = scope(&Property::new(vec![Normalized(-2.0)]));
    assert!(try_typed_property_binding::<Vec<Normalized>>("test", &definition, &nested).is_none());
    assert_eq!(
        build::<Vec<Normalized>>(&definition, &nested).get()[0].0,
        0.0
    );
}

#[test]
fn noncanonical_dependency_metadata_uses_fallback() {
    let source = Property::new(1.0_f64);
    let stack = scope(&source);
    let ValueDefinition::Expression(mut info) = expression("source") else {
        unreachable!()
    };
    info.dependencies.clear();
    assert!(try_typed_property_binding::<f64>(
        "test",
        &ValueDefinition::Expression(info.clone()),
        &stack
    )
    .is_none());
    info.dependencies = vec!["source".into(), "external_clock".into()];
    assert!(
        try_typed_property_binding::<f64>("test", &ValueDefinition::Expression(info), &stack)
            .is_none()
    );
}

#[test]
fn typed_base_candidate_keeps_reactive_settings_selection() {
    let source = Property::new(2.0_f64);
    let condition = Property::new(true);
    let stack = scope(&source).push(HashMap::from([(
        "enabled".into(),
        Variable::new_from_typed_property(condition.clone()),
    )]));
    let base = Property::new(10.0_f64);
    let stack_with_base = stack_with_base(&stack, base.clone());
    let candidate = build::<f64>(&expression("$base"), &stack_with_base);
    let ValueDefinition::Expression(condition_expr) = expression("enabled") else {
        unreachable!()
    };
    let selected = apply_runtime_settings_condition(
        "test",
        Some(&RuntimeSettingsCondition {
            positive: vec![condition_expr],
            negative: vec![],
        }),
        &stack,
        source.clone(),
        candidate,
    );
    assert_eq!(selected.get(), 10.0);
    base.set(12.0);
    assert_eq!(selected.get(), 12.0);
    condition.set(false);
    assert_eq!(selected.get(), 2.0);
    source.set(3.0);
    assert_eq!(selected.get(), 3.0);
}
