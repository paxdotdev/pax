use std::collections::HashMap;

use pax_runtime_api::{borrow_mut, pax_value::functions::call_function, PaxValue, Percent, RefCell, Rotation, Size};
use pest::{iterators::{Pair, Pairs}, pratt_parser::{self, PrattParser}};

use crate::{deserializer::from_pax_ast, get_pax_pratt_parser, parse_pax_err, parse_pax_pairs, PaxParser, Rule};

mod tests;


#[derive(PartialEq, Debug)]
pub enum PaxExpression {
    Primary(Box<PaxPrimary>),
    Prefix(Box<PaxPrefix>),
    Infix(Box<PaxInfix>),
    Postfix(Box<PaxPostfix>),
}

impl Computable for PaxExpression {
    fn compute(&self) -> Result<PaxValue, String> {
        match self {
            PaxExpression::Primary(p)  => p.compute(),
            PaxExpression::Prefix(p)   => p.compute(),
            PaxExpression::Infix(p)    => p.compute(),
            PaxExpression::Postfix(p)  => p.compute(),
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum PaxPrimary {
    Literal(PaxValue), // deserializer
    Identifier(PaxIdentifier),  // untyped -> 
}

impl Computable for PaxPrimary {
    fn compute(&self) -> Result<PaxValue, String> {
        match self {
            PaxPrimary::Literal(v) => Ok(v.clone()),
            PaxPrimary::Identifier(i) => {
                Err("Identifier not implemented".to_string())
            }
        }
    }
}


#[derive(PartialEq, Debug)]
pub struct PaxPrefix {
    operator: PaxOperator,
    rhs: Box<PaxExpression>,
}

impl Computable for PaxPrefix {
    fn compute(&self) -> Result<PaxValue, String> {
        let rhs = self.rhs.compute()?;
        let operator = &self.operator.name;
        call_function(operator.to_string(), vec![rhs])
    }
}


#[derive(PartialEq, Debug)]
pub struct PaxInfix {
    operator: PaxOperator,
    lhs: Box<PaxExpression>,
    rhs: Box<PaxExpression>,
}

impl Computable for PaxInfix {
    fn compute(&self) -> Result<PaxValue, String> {
        let lhs = self.lhs.compute()?;
        let rhs = self.rhs.compute()?;
        let operator = &self.operator.name;
        call_function(operator.to_string(), vec![lhs, rhs])
    }
}

#[derive(PartialEq, Debug)]
pub struct PaxPostfix {
    operator: PaxOperator,
    lhs: Box<PaxExpression>,
}

impl Computable for PaxPostfix {
    fn compute(&self) -> Result<PaxValue, String> {
        let lhs = self.lhs.compute()?;
        let operator = &self.operator.name;
        call_function(operator.to_string(), vec![lhs])
    }
}

trait Computable {
    fn compute(&self) -> Result<PaxValue, String>;
}
#[derive(PartialEq, Debug)]
pub struct PaxOperator {
    name: String,
}

#[derive(PartialEq, Debug)]
pub struct PaxIdentifier {
    name: String,
}


pub fn parse_pax_expression(expr: String) -> Result<PaxExpression, String> {
    let parsed_expr = parse_pax_pairs(Rule::expression_body, &expr)
        .map_err(|e| format!("Failed to parse expression: {}", e))?;
    let pratt_parser = get_pax_pratt_parser();
    recurse_pratt_parse(parsed_expr, &pratt_parser)
}

pub fn recurse_pratt_parse(expr: Pairs<Rule>, pratt_parser:&PrattParser<Rule>) -> Result<PaxExpression, String> {
    pratt_parser
        .map_primary(move |primary| match primary.as_rule() {
            Rule::xo_literal => {
                let inner = primary.into_inner().next().unwrap();
                match inner.as_rule() {
                    Rule::literal_value => {
                        let pax_value = from_pax_ast(inner).map_err(|e| format!("Failed to parse literal value: {}", e))?;
                        let value = PaxPrimary::Literal(pax_value);
                        let exp = PaxExpression::Primary(Box::new(value));
                        Ok(exp)
                    }
                    _ => {
                        return Err(format!("Unexpected rule: {:?}", inner.as_rule()));
                    }
                }
            },
            Rule::expression_body => {
                recurse_pratt_parse(primary.into_inner(), pratt_parser)
            }
            Rule::expression_grouped => {
                let mut inner = primary.clone().into_inner();
                let expr = inner.next().unwrap();
                let expr_val = recurse_pratt_parse(expr.into_inner(), pratt_parser)?.compute()?;
                let ret: Result<PaxValue, String> = if let Some(unit) = inner.next() {
                    match expr_val {
                        PaxValue::Numeric(n) => {
                            let unit = unit.as_str();
                            match unit {
                                "%" => Ok(PaxValue::Percent(Percent(n))),
                                "px" => Ok(PaxValue::Size(Size::Pixels(n))),
                                "rad" => Ok(PaxValue::Rotation(Rotation::Radians(n))),
                                "deg" => Ok(PaxValue::Rotation(Rotation::Degrees(n))),
                                _ => Err(format!("Unsupported unit: {}", unit))
                            }
                        }
                        _ => Err(format!("Unsupported value for unit conversion: {:?}", expr_val))
                    }
                } else {
                    Ok(expr_val)
                };
                ret.map(|v| PaxExpression::Primary(Box::new(PaxPrimary::Literal(v)))
                )
                
            }
            _ => {
                return Err(format!("Unexpected rule: {:?}", primary.as_rule()));
            }
        } )
        .map_prefix(|op, rhs| match op.as_rule() {
            _          => {
                let a = PaxOperator { name: op.as_str().trim().to_string() };
                let r = rhs?;
                let exp = PaxExpression::Prefix(
                    Box::new(PaxPrefix{
                        operator: a,
                        rhs: Box::new(r),
                    })
                );
                Ok(exp)
            },
        })
        .map_postfix(|lhs, op| match op.as_rule() {
            _          => {
                let a = PaxOperator { name: op.as_str().trim().to_string() };
                let l = lhs?;
                let exp = PaxExpression::Postfix(
                    Box::new(PaxPostfix{
                        operator: a,
                        lhs: Box::new(l),
                    })
                );
                Ok(exp)
            },
        })
        .map_infix(|lhs, op, rhs| match op.as_rule()  {
            _  => {
                let a = PaxOperator { name: op.as_str().trim().to_string() };
                let l = lhs?;
                let r = rhs?;
                let exp = PaxExpression::Infix(
                    Box::new(PaxInfix{
                        operator: a,
                        lhs: Box::new(l),
                        rhs: Box::new(r),
                    })
                );
                Ok(exp)
            },
        })
        .parse(expr)
}

pub fn compute_paxel(expr: String) -> Result<PaxValue, String> {
    let expr = parse_pax_expression(expr)?;
    expr.compute()
}