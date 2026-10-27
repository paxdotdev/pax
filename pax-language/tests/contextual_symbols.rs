#![cfg(feature = "parser")]

use pax_language::{
    formatting::format_pax_template, parse_pax_expression, parse_pax_str, Computable,
    DependencyCollector, Rule,
};
use pax_runtime_api::{functions::Functions, PaxValue, ToPaxValue};
use std::{collections::HashMap, rc::Rc};

#[test]
fn contextual_values_keep_their_identity_and_helper_argument_dependencies() {
    Functions::register_all_functions();
    let expression = parse_pax_expression("Math::max(@frames, frames) + values[@frames]").unwrap();
    assert_eq!(
        expression.collect_dependencies(),
        ["@frames", "frames", "values", "@frames"]
    );
    let values: HashMap<String, PaxValue> = HashMap::from([
        ("@frames".into(), 1.to_pax_value()),
        ("frames".into(), 5.to_pax_value()),
        ("values".into(), vec![10, 20].to_pax_value()),
    ]);
    let graph = pax_runtime_api::PropertyGraph::new(|| {});
    let _scope = graph.enter();
    assert_eq!(
        expression.compute(Rc::new(values)).unwrap(),
        25.to_pax_value()
    );
}

#[test]
fn contextual_values_coexist_with_directives_events_and_helper_calls() {
    let source = r#"
if @gpu {
    <Rectangle class="card" width={(@viewport.width / 2)px}
        opacity=@timeline { 0%: 0, 100%: {@base} }
        fill=@gradient { 0%: {@cpu ? RED : BLUE} 100%: WHITE }
        @click=self.activate @in=@timeline { x: { 0%: {@base - 20px}, 100%: {@base} } } />
} else {
    <Text text={Notes::progress_label(self.progress)} />
}
for i in 0..@frames { <Rectangle width={(i)px} /> }
@settings {
    @mount: self.mount
    if @viewport.width < 600 { .card { width: {@base + 12px} } }
}
@timeline reveal {
    playhead: {@millis}
    .card { y: { 0%: {@base}, 100%: {@base + 20px} } }
}
"#;
    parse_pax_str(Rule::pax_component_definition, source).unwrap();
    let formatted = format_pax_template(source.into()).unwrap();
    parse_pax_str(Rule::pax_component_definition, &formatted).unwrap();
    for symbol in [
        "@gpu",
        "@cpu",
        "@viewport.width",
        "@frames",
        "@millis",
        "@base",
        "Notes::progress_label",
    ] {
        assert!(formatted.contains(symbol), "formatter lost {symbol}");
    }
    assert_eq!(format_pax_template(formatted.clone()).unwrap(), formatted);
}

#[test]
fn old_prefix_and_prefixed_helper_calls_are_not_template_expressions() {
    for source in [
        "if $gpu { <Rectangle/> }",
        "<Rectangle width={$base + 10px}/>",
        "<Rectangle width=@viewport.width/>",
        "<Rectangle width={@Math::max(1, 2)}/>",
        "<Rectangle width={@helper(1)}/>",
    ] {
        assert!(
            parse_pax_str(Rule::pax_component_definition, source).is_err(),
            "accepted {source}"
        );
    }
    // Sigils in literal text are not contextual references.
    parse_pax_str(
        Rule::pax_component_definition,
        r#"<Text text="$gpu email@example.com"/>"#,
    )
    .unwrap();
}
