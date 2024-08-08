use std::{cell::Cell, collections::HashMap, hash::Hash, marker::PhantomData, rc::Rc};

use pax_runtime_api::{
    functions::{Functions, HelperFunctions},
    CoercionRules, Color, ColorChannel, Numeric, PaxValue, Size,
};
use serde::de::Expected;

use crate::{interpreter::compute_paxel, DependencyCollector};

use super::{parse_pax_expression, PaxExpression, PaxInfix, PaxOperator, PaxPrimary};

fn initialize_test_resolver() -> Rc<HashMap<String, PaxValue>> {
    Functions::register_all_functions();
    let mut idr = HashMap::new();
    idr.insert("a".to_string(), PaxValue::Numeric(Numeric::I64(10)));
    idr.insert("b".to_string(), PaxValue::Numeric(Numeric::I64(4)));
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
    let idr = initialize_test_resolver();
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
    let v = parse_pax_expression(expr, idr).unwrap();
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
    let expr = "Math::min(10,3)";
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
    let expected = PaxValue::Color(Color::rgba(
        ColorChannel::Integer(Numeric::I64(10)),
        ColorChannel::Integer(Numeric::I64(20)),
        ColorChannel::Integer(Numeric::I64(30)),
        ColorChannel::Integer(Numeric::I64(4)),
    ));
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
    let idr = initialize_test_resolver();
    let expr = "a + b";
    let expected = vec!["a".to_string(), "b".to_string()];
    let result = PaxExpression::collect_dependencies(&parse_pax_expression(expr, idr).unwrap());
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
