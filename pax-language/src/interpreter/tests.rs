use std::{collections::HashMap, rc::Rc};

use pax_runtime_api::{
    functions::Functions, CoercionRules, Color, ColorChannel, Duration, Numeric, PaxValue, Size,
};

use crate::{interpreter::compute_paxel, parse_pax_pairs, DependencyCollector, Rule};

use super::{parse_pax_expression, PaxExpression, PaxInfix, PaxOperator, PaxPrimary};

fn initialize_test_resolver() -> Rc<HashMap<String, PaxValue>> {
    Functions::register_all_functions();
    let mut idr = HashMap::new();
    idr.insert("a".to_string(), PaxValue::Numeric(Numeric::I64(10)));
    idr.insert("b".to_string(), PaxValue::Numeric(Numeric::I64(4)));
    idr.insert("flag".to_string(), PaxValue::Bool(true));
    idr.insert(
        "maybe_a".to_string(),
        PaxValue::Option(Box::new(Some(PaxValue::Numeric(Numeric::I64(10))))),
    );
    idr.insert(
        "missing_value".to_string(),
        PaxValue::Option(Box::new(None)),
    );
    idr.insert(
        "c".to_string(),
        PaxValue::Vec(vec![
            PaxValue::Numeric(Numeric::I64(1)),
            PaxValue::Numeric(Numeric::I64(2)),
        ]),
    );
    idr.insert(
        "d".to_string(),
        PaxValue::Object(
            vec![
                ("a".to_string(), PaxValue::Numeric(Numeric::I64(1))),
                ("b".to_string(), PaxValue::Numeric(Numeric::I64(2))),
            ]
            .into_iter()
            .collect(),
        ),
    );
    idr.insert(
        "e".to_string(),
        PaxValue::Object(
            vec![
                ("a".to_string(), PaxValue::Numeric(Numeric::I64(1))),
                ("b".to_string(), PaxValue::Numeric(Numeric::I64(2))),
                (
                    "c".to_string(),
                    PaxValue::Object(
                        vec![
                            ("a".to_string(), PaxValue::Numeric(Numeric::I64(1))),
                            ("b".to_string(), PaxValue::Numeric(Numeric::I64(2))),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                ),
            ]
            .into_iter()
            .collect(),
        ),
    );
    Rc::new(idr)
}

#[test]
fn test_ast() {
    let expr = "10 + 4";
    let expected = PaxExpression::Infix(Box::new(PaxInfix {
        operator: PaxOperator {
            name: "+".to_string(),
        },
        lhs: Box::new(PaxExpression::Primary(Box::new(PaxPrimary::Literal(
            PaxValue::Numeric(Numeric::I64(10)),
        )))),
        rhs: Box::new(PaxExpression::Primary(Box::new(PaxPrimary::Literal(
            PaxValue::Numeric(Numeric::I64(4)),
        )))),
    }));
    let v = parse_pax_expression(expr).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_addition() {
    let idr = initialize_test_resolver();
    let expr = "10 + 4";
    let expected = PaxValue::Numeric(Numeric::I64(14));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_subtraction() {
    let idr = initialize_test_resolver();
    let expr = "10 - 4";
    let expected = PaxValue::Numeric(Numeric::I64(6));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_multiplication() {
    let idr = initialize_test_resolver();
    let expr = "10 * 4";
    let expected = PaxValue::Numeric(Numeric::I64(40));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_division() {
    let idr = initialize_test_resolver();
    let expr = "10.0 / 4";
    let expected = PaxValue::Numeric(Numeric::F64(2.5));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_exponentiation() {
    let idr = initialize_test_resolver();
    let expr = "10 ^ 4";
    let expected = PaxValue::Numeric(Numeric::F64(10000.0));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_modulus() {
    let idr = initialize_test_resolver();
    let expr = "10 %% 4";
    let expected = PaxValue::Numeric(Numeric::I64(2));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_rel_eq() {
    let idr = initialize_test_resolver();
    let expr = "10 == 4";
    let expected: PaxValue = PaxValue::Bool(false);
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_rel_gt() {
    let idr = initialize_test_resolver();
    let expr = "10 > 4";
    let expected = PaxValue::Bool(true);
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_rel_gte() {
    let idr = initialize_test_resolver();
    let expr = "10 >= 4";
    let expected = PaxValue::Bool(true);
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_rel_lt() {
    let idr = initialize_test_resolver();
    let expr = "10 < 4";
    let expected = PaxValue::Bool(false);
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_rel_lte() {
    let idr = initialize_test_resolver();
    let expr = "10 <= 4";
    let expected = PaxValue::Bool(false);
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_rel_neq() {
    let idr = initialize_test_resolver();
    let expr = "10 != 4";
    let expected = PaxValue::Bool(true);
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_bool_and() {
    let idr = initialize_test_resolver();
    let expr = "true && false";
    let expected = PaxValue::Bool(false);
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_bool_or() {
    let idr = initialize_test_resolver();
    let expr = "true || false";
    let expected = PaxValue::Bool(true);
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_ternary_true_branch() {
    let idr = initialize_test_resolver();
    let expr = "true ? 10 : 4";
    let expected = PaxValue::Numeric(Numeric::I64(10));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_ternary_false_branch() {
    let idr = initialize_test_resolver();
    let expr = "false ? 10 : 4";
    let expected = PaxValue::Numeric(Numeric::I64(4));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_ternary_short_circuits() {
    let idr = initialize_test_resolver();
    let expr = "true ? a : missing_identifier";
    let expected = PaxValue::Numeric(Numeric::I64(10));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_nested_ternary() {
    let idr = initialize_test_resolver();
    let expr = "false ? 1 : true ? 2 : 3";
    let expected = PaxValue::Numeric(Numeric::I64(2));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_null_coalesce_some_unwraps() {
    let idr = initialize_test_resolver();
    let expr = "Some(10) ?? 4";
    let expected = PaxValue::Numeric(Numeric::I64(10));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_null_coalesce_none_uses_fallback() {
    let idr = initialize_test_resolver();
    let expr = "None ?? 4";
    let expected = PaxValue::Numeric(Numeric::I64(4));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_null_coalesce_variable_option() {
    let idr = initialize_test_resolver();
    let expr = "maybe_a ?? b";
    let expected = PaxValue::Numeric(Numeric::I64(10));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_null_coalesce_right_associative() {
    let idr = initialize_test_resolver();
    let expr = "None ?? Some(4) ?? 5";
    let expected = PaxValue::Numeric(Numeric::I64(4));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_null_coalesce_short_circuits_non_option() {
    let idr = initialize_test_resolver();
    let expr = "\"hello\" ?? missing_identifier";
    let expected = PaxValue::String("hello".to_string());
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_parentheses() {
    let idr = initialize_test_resolver();
    let expr = "(10 + 4) * 2";
    let expected = PaxValue::Numeric(Numeric::I64(28));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_parentheses_units() {
    let idr = initialize_test_resolver();
    let expr = "(10 + 4)px";
    let expected = PaxValue::Size(Size::Pixels(Numeric::I64(14)));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_parentheses_units_must_be_adjacent() {
    let adjacent = parse_pax_pairs(Rule::expression_body, "(10 + 4)px").unwrap();
    assert_eq!(adjacent.as_str(), "(10 + 4)px");

    let spaced = parse_pax_pairs(Rule::expression_body, "(10 + 4) px").unwrap();
    assert_ne!(spaced.as_str(), "(10 + 4) px");
}

#[test]
fn test_parentheses_duration_units() {
    let idr = initialize_test_resolver();
    let expr = "(100 + b)ms";
    let expected = PaxValue::Duration(Duration::Milliseconds(Numeric::I64(104)));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_adding_strings() {
    let idr = initialize_test_resolver();
    let expr = "\"hello\" + \" world\"";
    let expected = PaxValue::String("hello world".to_string());
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_function_call() {
    let idr = initialize_test_resolver();
    let expr = "Math::min(3,5)";
    let expected = PaxValue::Numeric(Numeric::I64(3));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_function_call_with_expression() {
    let idr = initialize_test_resolver();
    let expr = "Math::min(10,3 + 1)";
    let expected = PaxValue::Numeric(Numeric::I64(4));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_function_call_with_expression_and_variable() {
    let idr = initialize_test_resolver();
    let expr = "Math::min(a,3 + 1)";
    let expected = PaxValue::Numeric(Numeric::I64(4));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_expr_to_numeric() {
    let idr = initialize_test_resolver();
    let expr = "Math::min(a,Math::max(3,1))";
    let expected = Numeric::from(3);
    let result = Numeric::try_coerce(compute_paxel(expr, idr).unwrap()).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_object_expression() {
    let idr = initialize_test_resolver();
    let expr = "{a: 10+4, b: true || false }";
    let expected = PaxValue::Object(
        vec![
            ("a".to_string(), PaxValue::Numeric(Numeric::I64(14))),
            ("b".to_string(), PaxValue::Bool(true)),
        ]
        .into_iter()
        .collect(),
    );
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_nested_object_expression() {
    let idr = initialize_test_resolver();
    let expr = "{a: 10+4, b: {c: 10, d: 50-30} }";
    let expected = PaxValue::Object(
        vec![
            ("a".to_string(), PaxValue::Numeric(Numeric::I64(14))),
            (
                "b".to_string(),
                PaxValue::Object(
                    vec![
                        ("c".to_string(), PaxValue::Numeric(Numeric::I64(10))),
                        ("d".to_string(), PaxValue::Numeric(Numeric::I64(20))),
                    ]
                    .into_iter()
                    .collect(),
                ),
            ),
        ]
        .into_iter()
        .collect(),
    );
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_range_expression() {
    let idr = initialize_test_resolver();
    let expr = "a..b";
    let expected = PaxValue::Range(
        Box::new(PaxValue::Numeric(Numeric::I64(10))),
        Box::new(PaxValue::Numeric(Numeric::I64(4))),
    );
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_tuple_expression() {
    let idr = initialize_test_resolver();
    let expr = "(a, b)";
    let expected = PaxValue::Vec(vec![
        PaxValue::Numeric(Numeric::I64(10)),
        PaxValue::Numeric(Numeric::I64(4)),
    ]);
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_list_expression() {
    let idr = initialize_test_resolver();
    let expr = "[a, b]";
    let expected = PaxValue::Vec(vec![
        PaxValue::Numeric(Numeric::I64(10)),
        PaxValue::Numeric(Numeric::I64(4)),
    ]);
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_tuple_access() {
    let idr = initialize_test_resolver();
    let expr = "c.0";
    let expected = PaxValue::Numeric(Numeric::I64(1));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_list_access() {
    let idr = initialize_test_resolver();
    let expr = "c[1]";
    let expected = PaxValue::Numeric(Numeric::I64(2));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_color_expression() {
    let idr = initialize_test_resolver();
    let expr = "rgba(10, 20, 30,4)";
    let result = compute_paxel(expr, idr).unwrap();
    let expected = PaxValue::Color(Box::new(Color::rgba(
        ColorChannel::Integer(10),
        ColorChannel::Integer(20),
        ColorChannel::Integer(30),
        ColorChannel::Integer(4),
    )));
    assert_eq!(expected, result);
}

#[test]
fn test_struct_access() {
    let idr = initialize_test_resolver();
    let expr = "d.a";
    let expected = PaxValue::Numeric(Numeric::I64(1));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_triple_nesting_struct_access() {
    let idr = initialize_test_resolver();
    let expr = "e.c.a";
    let expected = PaxValue::Numeric(Numeric::I64(1));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_collect_dependencies() {
    let expr = "a + b";
    let expected = vec!["a".to_string(), "b".to_string()];
    let result = PaxExpression::collect_dependencies(&parse_pax_expression(expr).unwrap());
    assert_eq!(expected, result);
}

#[test]
fn test_collect_builtin_dollar_dependency() {
    let expr = "$base + 12px";
    let expected = vec!["$base".to_string()];
    let result = PaxExpression::collect_dependencies(&parse_pax_expression(expr).unwrap());
    assert_eq!(expected, result);
}

#[test]
fn test_builtin_dollar_struct_access() {
    let mut idr = HashMap::new();
    idr.insert(
        "$gyro".to_string(),
        PaxValue::Object(
            vec![
                ("x".to_string(), PaxValue::Numeric(Numeric::F64(1.0))),
                ("y".to_string(), PaxValue::Numeric(Numeric::F64(2.5))),
                ("z".to_string(), PaxValue::Numeric(Numeric::F64(3.0))),
            ]
            .into_iter()
            .collect(),
        ),
    );

    let result = compute_paxel("$gyro.y", Rc::new(idr)).unwrap();
    assert_eq!(PaxValue::Numeric(Numeric::F64(2.5)), result);
}

#[test]
fn test_collect_ternary_dependencies() {
    let expr = "flag ? a : b";
    let expected = vec!["flag".to_string(), "a".to_string(), "b".to_string()];
    let result = PaxExpression::collect_dependencies(&parse_pax_expression(expr).unwrap());
    assert_eq!(expected, result);
}

#[test]
fn test_collect_null_coalesce_dependencies() {
    let expr = "missing_value ?? b";
    let expected = vec!["missing_value".to_string(), "b".to_string()];
    let result = PaxExpression::collect_dependencies(&parse_pax_expression(expr).unwrap());
    assert_eq!(expected, result);
}

#[test]
fn test_negative_size() {
    let idr = initialize_test_resolver();
    let expr = "-10px";
    let expected = PaxValue::Size(Size::Pixels(Numeric::I64(-10)));
    let result = compute_paxel(expr, idr).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_display_expression() {
    let expr = "10 + 4";
    let expected = "10 + 4";
    let result = format!("{}", parse_pax_expression(expr).unwrap());
    assert_eq!(expected, result);
}

#[test]
fn test_display_complex_expression() {
    let expr = "Math::min(a,Math::max(3,1))";
    let expected = "Math::min(a, Math::max(3, 1))";
    let result = format!("{}", parse_pax_expression(expr).unwrap());
    assert_eq!(expected, result);
}

#[test]
fn test_display_ternary_expression() {
    let expr = "flag ? a + 1 : b + 2";
    let expected = "flag ? a + 1 : b + 2";
    let result = format!("{}", parse_pax_expression(expr).unwrap());
    assert_eq!(expected, result);
}

#[test]
fn test_display_null_coalesce_expression() {
    let expr = "maybe_title ?? \"Untitled\"";
    let expected = "maybe_title ?? \"Untitled\"";
    let result = format!("{}", parse_pax_expression(expr).unwrap());
    assert_eq!(expected, result);
}

#[test]
fn test_display_object() {
    let expr = "{a: 10+4, b: true || false }";
    let expected = "{a: 10 + 4, b: true || false}";
    let result = format!("{}", parse_pax_expression(expr).unwrap());
    assert_eq!(expected, result);
}

#[test]
fn pax995_precedence_and_reparse() {
    let idr = initialize_test_resolver();
    for (source, expected) in [
        ("1 + 2 > 2", "true"),
        ("true || false && false", "true"),
        ("2 * 3 %% 2", "0"),
        ("11 %% 4 * 2", "6"),
        ("12 / 2 %% 4", "2"),
        ("10 - 3 - 2", "5"),
        ("2 ^ 3 ^ 2", "512"),
        ("-2 ^ 2", "4"),
        ("(2 ^ 3) ^ 2", "64"),
        ("1 + 2 == 3 && 4 * 2 >= 8", "true"),
        ("false || true ? 4 : 9", "4"),
        ("None ?? false || true", "true"),
        ("None ?? None ?? 7", "7"),
        ("false ? 1 : true ? 2 : 3", "2"),
        ("(2 + 3)px", "5px"),
    ] {
        let expected = compute_paxel(expected, idr.clone()).unwrap();
        assert_eq!(
            compute_paxel(source, idr.clone()).unwrap(),
            expected,
            "{source}"
        );
        let formatted = parse_pax_expression(source).unwrap().to_string();
        assert_eq!(
            compute_paxel(&formatted, idr.clone()).unwrap(),
            expected,
            "{formatted}"
        );
    }
}

#[test]
fn pax995_boolean_short_circuit() {
    let idr = initialize_test_resolver();
    for (source, expected) in [
        ("false && (c[9] == 1)", false),
        ("true || (c[9] == 1)", true),
        ("true && flag", true),
        ("false || flag", true),
        ("true || (missing_value ?? c[9])", true),
        ("false ? c[9] : true || c[9]", true),
    ] {
        assert_eq!(
            compute_paxel(source, idr.clone()).unwrap(),
            PaxValue::Bool(expected),
            "{source}"
        );
    }
    for source in [
        "true && c[9]",
        "false || c[9]",
        "c[9] || true",
        "c[9] ?? true",
    ] {
        assert!(compute_paxel(source, idr.clone()).is_err(), "{source}");
    }
}

#[test]
fn pax995_index_dependencies() {
    for (source, expected) in [
        ("items[index]", vec!["index", "items"]),
        (
            "items[indices[index + offset]].rows[index]",
            vec!["index", "indices", "items", "offset"],
        ),
    ] {
        let mut deps = parse_pax_expression(source).unwrap().collect_dependencies();
        deps.sort();
        assert_eq!(deps, expected, "{source}");
    }
}

#[test]
fn pax995_template_formatter_preserves_expression_trees() {
    let source = "<Text text={1+2>2?items[index+1]:2*3%%2} />";
    let formatted = crate::formatting::format_pax_template(source.into()).unwrap();
    fn expressions(source: &str) -> Vec<PaxExpression> {
        fn visit(pair: pest::iterators::Pair<'_, Rule>, out: &mut Vec<PaxExpression>) {
            if pair.as_rule() == Rule::expression_body {
                out.push(parse_pax_expression(pair.as_str()).unwrap());
            } else {
                for child in pair.into_inner() {
                    visit(child, out);
                }
            }
        }
        let mut out = Vec::new();
        for pair in parse_pax_pairs(Rule::pax_component_definition, source).unwrap() {
            visit(pair, &mut out);
        }
        out
    }
    assert_eq!(expressions(source), expressions(&formatted));
}

#[test]
fn pax995_required_boolean_operands_preserve_value_type_behavior() {
    let idr = initialize_test_resolver();
    // Unsupported operands retain the existing Math fallback; this change does
    // not introduce truthiness or redesign the value layer's type diagnostics.
    for source in ["true && 1", "false || 1", "1 && true", "1 || false"] {
        assert_eq!(
            compute_paxel(source, idr.clone()).unwrap(),
            PaxValue::default()
        );
    }
}

#[test]
fn pax995_constructed_ast_display_preserves_grouping() {
    use crate::Computable;
    let value = |s| parse_pax_expression(s).unwrap();
    let idr = initialize_test_resolver();
    for expr in [
        PaxExpression::infix(value("1 + 2"), "*", value("3")),
        PaxExpression::infix(value("10"), "-", value("3 - 2")),
        PaxExpression::infix(value("2 ^ 3"), "^", value("2")),
        PaxExpression::prefix("-", value("1 + 2")),
        PaxExpression::infix(value("true || false"), "&&", value("false")),
        PaxExpression::null_coalesce(value("None ?? None"), value("4")),
        PaxExpression::null_coalesce(value("None"), value("false ? 2 : 3")),
        PaxExpression::ternary(value("true ? false : true"), value("1"), value("2")),
    ] {
        let formatted = expr.to_string();
        assert_eq!(
            compute_paxel(&formatted, idr.clone()),
            expr.compute(idr.clone()),
            "{formatted}"
        );
    }
}
