use pax_lang::{get_pax_pratt_parser, Pairs, Parser, PaxParser, PrattParser, Rule};
use pax_runtime_api::PaxValue;

thread_local! {
    static PRATT_PARSER: PrattParser<Rule> = get_pax_pratt_parser();
}

pub enum PaxExpression {
    Primary(Box<PaxPrimary>),
    Prefix(Box<PaxPrefix>),
    Infix(Box<PaxInfix>),
    Postfix(Box<PaxPostfix>),
}

pub enum PaxPrimary {
    Literal(PaxValue), // deserializer
    Identifier(PaxIdentifier),  // untyped -> 
}

pub struct PaxPrefix {
    operator: PaxOperator,
    rhs: Box<PaxExpression>,
}

pub struct PaxInfix {
    operator: PaxOperator,
    lhs: Box<PaxExpression>,
    rhs: Box<PaxExpression>,
}

pub struct PaxPostfix {
    operator: PaxOperator,
    lhs: Box<PaxExpression>,
}

trait Computable {
    fn compute(&self) -> PaxValue;
}

pub struct PaxOperator {
    name: String,
}

pub struct PaxIdentifier {
    name: String,
}

pub fn parse_pax_expression(expr: String) -> Result<PaxExpression, String> {
    let parsed_expr = PaxParser::parse(Rule::expression_body, &expr)
        .map_err(|e| format!("Failed to parse expression: {}", e))?;
    todo!()
}

pub fn recurse_pratt_parse(expr: Pairs<Rule>) -> Result<PaxExpression, String> {
    PRATT_PARSER.with(|parser|{
        parser.map_primary(
            move | primary | match primary.as_rule() {
               Rule::expression_grouped => {}
            }
        )
    })

    todo!()
}