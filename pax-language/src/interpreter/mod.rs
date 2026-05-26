#[cfg(feature = "parser")]
use computable::Computable;
use pax_runtime_api::PaxValue;
#[cfg(feature = "parser")]
use pest::{
    iterators::{Pair, Pairs},
    pratt_parser::PrattParser,
};
#[cfg(feature = "parser")]
use property_resolution::IdentifierResolver;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
#[cfg(feature = "parser")]
use std::rc::Rc;

#[cfg(feature = "parser")]
use crate::{deserializer::from_pax_ast, get_pax_pratt_parser, parse_pax_pairs, Rule};

pub(crate) mod computable;
pub mod property_resolution;
#[cfg(test)]
mod tests;

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
/// PAXEL expression AST node.
pub enum PaxExpression {
    Primary(Box<PaxPrimary>),
    Prefix(Box<PaxPrefix>),
    Infix(Box<PaxInfix>),
    Postfix(Box<PaxPostfix>),
    Ternary(Box<PaxTernary>),
    NullCoalesce(Box<PaxNullCoalesce>),
}

impl Default for PaxExpression {
    fn default() -> Self {
        Self::Primary(Box::new(PaxPrimary::default()))
    }
}

impl PaxExpression {
    /// Construct a prefix expression.
    pub fn prefix(operator: impl Into<String>, rhs: PaxExpression) -> Self {
        Self::Prefix(Box::new(PaxPrefix {
            operator: PaxOperator::new(operator),
            rhs: Box::new(rhs),
        }))
    }

    /// Construct an infix expression.
    pub fn infix(lhs: PaxExpression, operator: impl Into<String>, rhs: PaxExpression) -> Self {
        Self::Infix(Box::new(PaxInfix {
            operator: PaxOperator::new(operator),
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }))
    }

    /// Construct a postfix expression.
    pub fn postfix(lhs: PaxExpression, operator: impl Into<String>) -> Self {
        Self::Postfix(Box::new(PaxPostfix {
            operator: PaxOperator::new(operator),
            lhs: Box::new(lhs),
        }))
    }

    /// Construct a ternary expression.
    pub fn ternary(
        condition: PaxExpression,
        then_branch: PaxExpression,
        else_branch: PaxExpression,
    ) -> Self {
        Self::Ternary(Box::new(PaxTernary {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        }))
    }

    /// Construct a null-coalescing expression.
    pub fn null_coalesce(lhs: PaxExpression, rhs: PaxExpression) -> Self {
        Self::NullCoalesce(Box::new(PaxNullCoalesce {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }))
    }
}

impl Display for PaxExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaxExpression::Primary(p) => write!(f, "{}", p),
            PaxExpression::Prefix(p) => write!(f, "{}{}", p.operator.name, p.rhs),
            PaxExpression::Infix(i) => write!(f, "{} {} {}", i.lhs, i.operator.name, i.rhs),
            PaxExpression::Postfix(p) => write!(f, "{}{}", p.lhs, p.operator.name),
            PaxExpression::Ternary(t) => {
                write!(f, "{} ? {} : {}", t.condition, t.then_branch, t.else_branch)
            }
            PaxExpression::NullCoalesce(n) => write!(f, "{} ?? {}", n.lhs, n.rhs),
        }
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
/// Primary expression forms: literals, symbols, object/list/tuple literals, calls, and ranges.
pub enum PaxPrimary {
    Literal(PaxValue),
    Grouped(Box<PaxExpression>, Option<PaxUnit>),
    Identifier(PaxIdentifier, Vec<PaxAccessor>),
    Object(Vec<(String, PaxExpression)>),
    FunctionOrEnum(String, String, Vec<PaxExpression>),
    Range(PaxExpression, PaxExpression),
    Tuple(Vec<PaxExpression>),
    List(Vec<PaxExpression>),
}

impl Display for PaxPrimary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaxPrimary::Literal(l) => write!(f, "{}", l),
            PaxPrimary::Grouped(e, u) => {
                if let Some(u) = u {
                    write!(
                        f,
                        "({}){}",
                        e,
                        match u {
                            PaxUnit::Percent => "%",
                            PaxUnit::Pixels => "px",
                            PaxUnit::Radians => "rad",
                            PaxUnit::Degrees => "deg",
                            PaxUnit::Milliseconds => "ms",
                            PaxUnit::Seconds => "s",
                            PaxUnit::Frames => "f",
                        }
                    )
                } else {
                    write!(f, "({})", e)
                }
            }
            PaxPrimary::Identifier(i, a) => {
                write!(f, "{}", i.name)?;
                for accessor in a {
                    match accessor {
                        PaxAccessor::Tuple(i) => write!(f, ".{}", i)?,
                        PaxAccessor::List(e) => write!(f, "[{}]", e)?,
                        PaxAccessor::Struct(s) => write!(f, ".{}", s)?,
                    }
                }
                Ok(())
            }
            PaxPrimary::Object(o) => {
                write!(f, "{{")?;
                let mut o = o.iter().collect::<Vec<_>>();
                o.sort_by(|a, b| a.0.cmp(&b.0));
                for (i, (key, val)) in o.iter().enumerate() {
                    write!(f, "{}: {}", key, val)?;
                    if i != o.len() - 1 {
                        write!(f, ", ")?;
                    }
                }
                write!(f, "}}")?;
                Ok(())
            }
            PaxPrimary::FunctionOrEnum(name, e, a) => {
                if name == "Color" {
                    write!(f, "{}", e)?;
                } else {
                    write!(f, "{}::{}", name, e)?;
                }
                if !a.is_empty() {
                    write!(f, "(")?;
                    for (i, arg) in a.iter().enumerate() {
                        write!(f, "{}", arg)?;
                        if i != a.len() - 1 {
                            write!(f, ", ")?;
                        }
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            PaxPrimary::Range(s, e) => write!(f, "{}..{}", s, e),
            PaxPrimary::Tuple(t) => {
                write!(f, "(")?;
                for (i, e) in t.iter().enumerate() {
                    write!(f, "{}", e)?;
                    if i != t.len() - 1 {
                        write!(f, ", ")?;
                    }
                }
                write!(f, ")")?;
                Ok(())
            }
            PaxPrimary::List(l) => {
                write!(f, "[")?;
                for (i, e) in l.iter().enumerate() {
                    write!(f, "{}", e)?;
                    if i != l.len() - 1 {
                        write!(f, ", ")?;
                    }
                }
                write!(f, "]")?;
                Ok(())
            }
        }
    }
}

impl Default for PaxPrimary {
    fn default() -> Self {
        Self::Literal(PaxValue::default())
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
/// Unit suffix attached to a grouped numeric expression.
pub enum PaxUnit {
    Percent,
    Pixels,
    Radians,
    Degrees,
    Milliseconds,
    Seconds,
    Frames,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
/// Access path applied after an identifier, such as `.field`, `.0`, or `[index]`.
pub enum PaxAccessor {
    Tuple(usize),
    List(PaxExpression),
    Struct(String),
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
/// Prefix operation such as numeric negation or boolean not.
pub struct PaxPrefix {
    operator: PaxOperator,
    rhs: Box<PaxExpression>,
}

impl PaxPrefix {
    /// Name of the prefix operator.
    pub fn operator_name(&self) -> &str {
        &self.operator.name
    }

    /// Right-hand expression.
    pub fn rhs(&self) -> &PaxExpression {
        &self.rhs
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
/// Binary infix operation with left and right expression operands.
pub struct PaxInfix {
    operator: PaxOperator,
    lhs: Box<PaxExpression>,
    rhs: Box<PaxExpression>,
}

impl PaxInfix {
    /// Name of the infix operator.
    pub fn operator_name(&self) -> &str {
        &self.operator.name
    }

    /// Left-hand expression.
    pub fn lhs(&self) -> &PaxExpression {
        &self.lhs
    }

    /// Right-hand expression.
    pub fn rhs(&self) -> &PaxExpression {
        &self.rhs
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
/// Postfix operation node.
pub struct PaxPostfix {
    operator: PaxOperator,
    lhs: Box<PaxExpression>,
}

impl PaxPostfix {
    /// Name of the postfix operator.
    pub fn operator_name(&self) -> &str {
        &self.operator.name
    }

    /// Left-hand expression.
    pub fn lhs(&self) -> &PaxExpression {
        &self.lhs
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
/// Conditional expression with a boolean condition and selected true/false branch.
pub struct PaxTernary {
    condition: Box<PaxExpression>,
    then_branch: Box<PaxExpression>,
    else_branch: Box<PaxExpression>,
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
/// Short-circuiting fallback expression. `Some(value) ?? fallback` evaluates to
/// `value`, `None ?? fallback` evaluates the fallback, and non-option left
/// operands pass through unchanged.
pub struct PaxNullCoalesce {
    lhs: Box<PaxExpression>,
    rhs: Box<PaxExpression>,
}

impl PaxTernary {
    /// Condition expression.
    pub fn condition(&self) -> &PaxExpression {
        &self.condition
    }

    /// Expression evaluated when the condition is true.
    pub fn then_branch(&self) -> &PaxExpression {
        &self.then_branch
    }

    /// Expression evaluated when the condition is false.
    pub fn else_branch(&self) -> &PaxExpression {
        &self.else_branch
    }
}

impl PaxNullCoalesce {
    /// Left-hand expression.
    pub fn lhs(&self) -> &PaxExpression {
        &self.lhs
    }

    /// Right-hand fallback expression.
    pub fn rhs(&self) -> &PaxExpression {
        &self.rhs
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
/// Parsed operator token, stored by display name.
pub struct PaxOperator {
    name: String,
}

impl PaxOperator {
    /// Construct an operator from its source spelling.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Source spelling for this operator.
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
/// Symbol reference in a PAXEL expression.
pub struct PaxIdentifier {
    pub name: String,
}

impl Display for PaxIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl PaxIdentifier {
    /// Construct an identifier from its source spelling.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

/// Parse a pax expression into a computable AST
#[cfg(feature = "parser")]
pub fn parse_pax_expression(expr: &str) -> Result<PaxExpression, String> {
    let parsed_expr = parse_pax_pairs(Rule::expression_body, expr)
        .map_err(|e| format!("Failed to parse expression: {}", e))?;
    let pratt_parser = get_pax_pratt_parser();
    recurse_pratt_parse(parsed_expr, &pratt_parser)
}

/// Parse an already-produced pest pair into a PAXEL expression AST.
#[cfg(feature = "parser")]
pub fn parse_pax_expression_from_pair(expr: Pair<Rule>) -> Result<PaxExpression, String> {
    let pratt_parser = get_pax_pratt_parser();
    recurse_pratt_parse(Pairs::single(expr), &pratt_parser)
}

#[cfg(feature = "parser")]
fn recurse_pratt_parse(
    expr: Pairs<Rule>,
    pratt_parser: &PrattParser<Rule>,
) -> Result<PaxExpression, String> {
    pratt_parser
        .map_primary(move |primary| match primary.as_rule() {
            Rule::xo_literal => {
                let inner = primary.into_inner().next().unwrap();
                match inner.as_rule() {
                    Rule::literal_value => {
                        let pax_value = from_pax_ast(inner)
                            .map_err(|e| format!("Failed to parse literal value: {}", e))?;
                        let value = PaxPrimary::Literal(pax_value);
                        let exp = PaxExpression::Primary(Box::new(value));
                        Ok(exp)
                    }
                    Rule::literal_tuple_access => {
                        let mut inner = inner.into_inner();
                        let ident = inner.next().unwrap().as_str().trim().to_string();
                        let index = inner.next().unwrap().as_str().parse::<usize>().unwrap();
                        let value = PaxPrimary::Identifier(
                            PaxIdentifier { name: ident },
                            vec![PaxAccessor::Tuple(index)],
                        );
                        let exp = PaxExpression::Primary(Box::new(value));
                        Ok(exp)
                    }
                    Rule::literal_list_access => {
                        let mut inner = inner.into_inner();
                        let ident = inner.next().unwrap().as_str().trim().to_string();
                        let index = inner.next().unwrap().as_str();
                        let index = parse_pax_expression(index)?;
                        let value = PaxPrimary::Identifier(
                            PaxIdentifier { name: ident },
                            vec![PaxAccessor::List(index)],
                        );
                        let exp = PaxExpression::Primary(Box::new(value));
                        Ok(exp)
                    }
                    _ => {
                        return Err(format!("Unexpected rule: {:?}", inner.as_rule()));
                    }
                }
            }
            Rule::expression_body | Rule::expression_binary => {
                recurse_pratt_parse(primary.into_inner(), pratt_parser)
            }
            Rule::expression_coalesce => {
                let mut inner = primary.into_inner();
                let lhs = recurse_pratt_parse(Pairs::single(inner.next().unwrap()), pratt_parser)?;
                if inner.next().is_some() {
                    let rhs =
                        recurse_pratt_parse(Pairs::single(inner.next().unwrap()), pratt_parser)?;
                    Ok(PaxExpression::NullCoalesce(Box::new(PaxNullCoalesce {
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    })))
                } else {
                    Ok(lhs)
                }
            }
            Rule::expression_ternary => {
                let mut inner = primary.into_inner();
                let condition =
                    recurse_pratt_parse(Pairs::single(inner.next().unwrap()), pratt_parser)?;
                if inner.next().is_some() {
                    let then_branch =
                        recurse_pratt_parse(Pairs::single(inner.next().unwrap()), pratt_parser)?;
                    inner.next();
                    let else_branch =
                        recurse_pratt_parse(Pairs::single(inner.next().unwrap()), pratt_parser)?;
                    Ok(PaxExpression::Ternary(Box::new(PaxTernary {
                        condition: Box::new(condition),
                        then_branch: Box::new(then_branch),
                        else_branch: Box::new(else_branch),
                    })))
                } else {
                    Ok(condition)
                }
            }
            Rule::expression_grouped => {
                let mut inner = primary.clone().into_inner();
                let expr = inner.next().unwrap();
                let expr_val = recurse_pratt_parse(expr.into_inner(), pratt_parser)?;
                let ret: Result<PaxExpression, String> = if let Some(unit) = inner.next() {
                    let unit = unit.as_str().trim().trim_start_matches(')');
                    match unit {
                        "%" => Ok(PaxExpression::Primary(Box::new(PaxPrimary::Grouped(
                            Box::new(expr_val),
                            Some(PaxUnit::Percent),
                        )))),
                        "px" => Ok(PaxExpression::Primary(Box::new(PaxPrimary::Grouped(
                            Box::new(expr_val),
                            Some(PaxUnit::Pixels),
                        )))),
                        "rad" => Ok(PaxExpression::Primary(Box::new(PaxPrimary::Grouped(
                            Box::new(expr_val),
                            Some(PaxUnit::Radians),
                        )))),
                        "deg" => Ok(PaxExpression::Primary(Box::new(PaxPrimary::Grouped(
                            Box::new(expr_val),
                            Some(PaxUnit::Degrees),
                        )))),
                        "ms" => Ok(PaxExpression::Primary(Box::new(PaxPrimary::Grouped(
                            Box::new(expr_val),
                            Some(PaxUnit::Milliseconds),
                        )))),
                        "s" => Ok(PaxExpression::Primary(Box::new(PaxPrimary::Grouped(
                            Box::new(expr_val),
                            Some(PaxUnit::Seconds),
                        )))),
                        "f" => Ok(PaxExpression::Primary(Box::new(PaxPrimary::Grouped(
                            Box::new(expr_val),
                            Some(PaxUnit::Frames),
                        )))),
                        _ => Err(format!("Unsupported unit: {}", unit)),
                    }
                } else {
                    Ok(PaxExpression::Primary(Box::new(PaxPrimary::Grouped(
                        Box::new(expr_val),
                        None,
                    ))))
                };
                ret
            }
            Rule::xo_enum_or_function_call => {
                let mut inner = primary.into_inner();
                while inner.len() > 3 {
                    inner.next();
                }
                let scope = inner.next().unwrap().as_str().trim().to_string();
                let function_name = inner.next().unwrap().as_str().trim().to_string();

                let args = if let Some(args) = inner.next() {
                    args.into_inner()
                        .map(|a| recurse_pratt_parse(a.into_inner(), pratt_parser))
                        .collect::<Result<Vec<PaxExpression>, String>>()?
                } else {
                    vec![]
                };
                let exp = PaxExpression::Primary(Box::new(PaxPrimary::FunctionOrEnum(
                    scope,
                    function_name,
                    args,
                )));
                Ok(exp)
            }
            Rule::xo_color_space_func => {
                let func = primary.as_str().trim().split("(").next().unwrap();
                let inner = primary.into_inner();
                let args = inner
                    .map(|a| recurse_pratt_parse(a.into_inner(), pratt_parser))
                    .collect::<Result<Vec<PaxExpression>, String>>()?;
                let exp = PaxExpression::Primary(Box::new(PaxPrimary::FunctionOrEnum(
                    "Color".to_string(),
                    func.to_string(),
                    args,
                )));
                Ok(exp)
            }
            Rule::xo_object => {
                let mut inner = primary.into_inner();
                while inner.peek().unwrap().as_rule() == Rule::identifier {
                    inner.next();
                }
                let mut obj = vec![];
                for pair in inner {
                    let mut pair = pair.into_inner();
                    // settings_key = { identifier ~ (":" | "=") }
                    let key = pair
                        .next()
                        .unwrap()
                        .as_str()
                        .trim()
                        .trim_end_matches(':')
                        .trim_end_matches('=')
                        .to_string();
                    let value = pair.next().unwrap();
                    let value = recurse_pratt_parse(value.into_inner(), pratt_parser)?;
                    obj.push((key, value));
                }
                let value = PaxPrimary::Object(obj);
                let exp = PaxExpression::Primary(Box::new(value));
                Ok(exp)
            }
            Rule::xo_range => {
                let mut inner = primary.into_inner();
                let start_rule = Pairs::single(inner.next().unwrap());
                let start = recurse_pratt_parse(start_rule, pratt_parser)?;
                // xo_range_exclusive
                inner.next();
                let end_rule = Pairs::single(inner.next().unwrap());
                let end = recurse_pratt_parse(end_rule, pratt_parser)?;
                let value = PaxPrimary::Range(start, end);
                let exp = PaxExpression::Primary(Box::new(value));
                Ok(exp)
            }
            Rule::xo_tuple => {
                let inner = primary.into_inner();
                let tuple = inner
                    .map(|e| recurse_pratt_parse(e.into_inner(), pratt_parser))
                    .collect::<Result<Vec<PaxExpression>, String>>()?;
                let value = PaxPrimary::Tuple(tuple);
                let exp = PaxExpression::Primary(Box::new(value));
                Ok(exp)
            }
            Rule::xo_list => {
                let inner = primary.into_inner();
                let list = inner
                    .map(|e| recurse_pratt_parse(e.into_inner(), pratt_parser))
                    .collect::<Result<Vec<PaxExpression>, String>>()?;
                let value = PaxPrimary::List(list);
                let exp = PaxExpression::Primary(Box::new(value));
                Ok(exp)
            }
            Rule::xo_symbol => {
                let builtins_prefix = if primary.as_str().starts_with("$") {
                    "$"
                } else {
                    ""
                }
                .to_string();

                let mut symbols = primary.into_inner();

                // skip self or this
                let peek = symbols.peek().unwrap();
                if peek.as_str().trim() == "self" || peek.as_str().trim() == "this" {
                    symbols.next();
                }

                let identifier = builtins_prefix + symbols.next().unwrap().as_str().trim();
                let mut accessors = vec![];
                for symbol in symbols {
                    let accessor = match symbol.as_rule() {
                        // list access
                        Rule::expression_body => {
                            let expr = recurse_pratt_parse(symbol.into_inner(), pratt_parser)?;
                            PaxAccessor::List(expr)
                        }
                        // field access .field
                        Rule::identifier => {
                            let field = symbol.as_str().trim().to_string();
                            PaxAccessor::Struct(field)
                        }
                        _ => {
                            return Err(format!("Unexpected rule: {:?}", symbol.as_rule()));
                        }
                    };
                    accessors.push(accessor);
                }
                let value = PaxPrimary::Identifier(PaxIdentifier { name: identifier }, accessors);
                let exp = PaxExpression::Primary(Box::new(value));
                Ok(exp)
            }
            _ => {
                return Err(format!(
                    "Unexpected rule: {:?}, str: {} ",
                    primary.as_rule(),
                    primary.as_str()
                ));
            }
        })
        .map_prefix(|op, rhs| match op.as_rule() {
            _ => {
                let a = PaxOperator {
                    name: op.as_str().trim().to_string(),
                };
                let r = rhs?;
                let exp = PaxExpression::Prefix(Box::new(PaxPrefix {
                    operator: a,
                    rhs: Box::new(r),
                }));
                Ok(exp)
            }
        })
        .map_postfix(|lhs, op| match op.as_rule() {
            _ => {
                let a = PaxOperator {
                    name: op.as_str().trim().to_string(),
                };
                let l = lhs?;
                let exp = PaxExpression::Postfix(Box::new(PaxPostfix {
                    operator: a,
                    lhs: Box::new(l),
                }));
                Ok(exp)
            }
        })
        .map_infix(|lhs, op, rhs| match op.as_rule() {
            _ => {
                let a = PaxOperator {
                    name: op.as_str().trim().to_string(),
                };
                let l = lhs?;
                let r = rhs?;
                let exp = PaxExpression::Infix(Box::new(PaxInfix {
                    operator: a,
                    lhs: Box::new(l),
                    rhs: Box::new(r),
                }));
                Ok(exp)
            }
        })
        .parse(expr)
}

/// Compute a pax expression to a PaxValue
#[cfg(feature = "parser")]
pub fn compute_paxel(expr: &str, idr: Rc<dyn IdentifierResolver>) -> Result<PaxValue, String> {
    let expr = parse_pax_expression(expr)?;
    expr.compute(idr)
}
