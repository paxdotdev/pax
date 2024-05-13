use pest::error::Error;
pub use pest::iterators::{Pair, Pairs};

pub use pest::pratt_parser::{Assoc, Op, PrattParser};
pub use pest::{Parser, Span};
pub use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "pax.pest"]
pub struct PaxParser;

pub fn parse_pax_str(expected_rule: Rule, input: &str) -> Result<Pair<Rule>, String> {
    let pairs = PaxParser::parse(expected_rule, input);
    match pairs {
        Ok(mut pairs) => {
            let pair = pairs.next().unwrap();
            Ok(pair)
        }
        Err(err) => {
            Err(format!("{err}"))
        }
    }
} 

pub fn parse_pax_err(expected_rule: Rule, input: &str) -> Result<Pair<Rule>, Error<Rule>> {
    let pairs = PaxParser::parse(expected_rule, input);
    match pairs {
        Ok(mut pairs) => {
            let pair = pairs.next().unwrap();
            Ok(pair)
        }
        Err(err) => {
            Err(err)
        }
    }
}