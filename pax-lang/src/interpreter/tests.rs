use pax_runtime_api::{Numeric, PaxValue, Size};
use crate::interpreter::compute_paxel;

use super::{parse_pax_expression, PaxExpression, PaxInfix, PaxOperator, PaxPrimary};

#[test]
fn test_ast() {
    let expr = "10 + 4";
    let expected = PaxExpression::Infix(Box::new(PaxInfix {
        operator: PaxOperator { name: "+".to_string() },
        lhs: Box::new(PaxExpression::Primary(Box::new(PaxPrimary::Literal(PaxValue::Numeric(Numeric::I64(10)))))),
        rhs: Box::new(PaxExpression::Primary(Box::new(PaxPrimary::Literal(PaxValue::Numeric(Numeric::I64(4)))))),
    }));
    let v = parse_pax_expression(expr.to_string()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_addition() {
    let expr = "10 + 4";
    let expected = PaxValue::Numeric(Numeric::I64(14));
    let v = compute_paxel(expr.to_string()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_subtraction() {
    let expr = "10 - 4";
    let expected = PaxValue::Numeric(Numeric::I64(6));
    let v = compute_paxel(expr.to_string()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_multiplication() {
    let expr = "10 * 4";
    let expected = PaxValue::Numeric(Numeric::I64(40));
    let v = compute_paxel(expr.to_string()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_division() {
    let expr = "10.0 / 4";
    let expected = PaxValue::Numeric(Numeric::F64(2.5));
    let v = compute_paxel(expr.to_string()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_exponentiation() {
    let expr = "10 ^ 4";
    let expected = PaxValue::Numeric(Numeric::F64(10000.0));
    let v = compute_paxel(expr.to_string()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_modulus() {
    let expr = "10 %% 4";
    let expected = PaxValue::Numeric(Numeric::I64(2));
    let v = compute_paxel(expr.to_string()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_rel_eq() {
    let expr = "10 == 4";
    let expected = PaxValue::Bool(false);
    let v = compute_paxel(expr.to_string()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_rel_gt() {
    let expr = "10 > 4";
    let expected = PaxValue::Bool(true);
    let v = compute_paxel(expr.to_string()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_rel_gte() {
    let expr = "10 >= 4";
    let expected = PaxValue::Bool(true);
    let v = compute_paxel(expr.to_string()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_rel_lt() {
    let expr = "10 < 4";
    let expected = PaxValue::Bool(false);
    let v = compute_paxel(expr.to_string()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_rel_lte() {
    let expr = "10 <= 4";
    let expected = PaxValue::Bool(false);
    let v = compute_paxel(expr.to_string()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_rel_neq() {
    let expr = "10 != 4";
    let expected = PaxValue::Bool(true);
    let v = compute_paxel(expr.to_string()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_bool_and() {
    let expr = "true && false";
    let expected = PaxValue::Bool(false);
    let v = compute_paxel(expr.to_string()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_bool_or() {
    let expr = "true || false";
    let expected = PaxValue::Bool(true);
    let v = compute_paxel(expr.to_string()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_parentheses() {
    let expr = "(10 + 4) * 2";
    let expected = PaxValue::Numeric(Numeric::I64(28));
    let v = compute_paxel(expr.to_string()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_parentheses_units() {
    let expr = "(10 + 4)px";
    let expected = PaxValue::Size(Size::Pixels(Numeric::I64(14)));
    let v = compute_paxel(expr.to_string()).unwrap();
    assert_eq!(expected, v);
}

#[test]
fn test_adding_strings() {
    let expr = "\"hello\" + \" world\"";
    let expected = PaxValue::String("hello world".to_string());
    let v = compute_paxel(expr.to_string()).unwrap();
    assert_eq!(expected, v);
}