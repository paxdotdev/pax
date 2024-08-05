use std::{collections::HashMap, hash::Hash, rc::Rc};

use crate::interpreter::compute_paxel;
use pax_runtime_api::{CoercionRules, Numeric, PaxValue, Size};

use super::{parse_pax_expression, PaxExpression, PaxInfix, PaxOperator, PaxPrimary};

fn initialize_test_resolver() -> Rc<HashMap<String, PaxValue>> {
    let mut idr = HashMap::new();
    idr.insert("a".to_string(), PaxValue::Numeric(Numeric::I64(10)));
    idr.insert("b".to_string(), PaxValue::Numeric(Numeric::I64(4)));
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
    let expected = PaxValue::Bool(false);
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
