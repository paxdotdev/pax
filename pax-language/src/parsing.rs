use pest::pratt_parser::{Assoc, Op, PrattParser};

use crate::Rule;

/// Build the Pratt parser that encodes PAXEL operator precedence and associativity.
pub fn get_pax_pratt_parser() -> PrattParser<Rule> {
    // Lowest to highest. Ternary and right-associative coalescing wrap this
    // binary grammar in pax.pest. Keep unary negation above exponentiation.
    PrattParser::new()
        .op(Op::infix(Rule::xo_bool_or, Assoc::Left))
        .op(Op::infix(Rule::xo_bool_and, Assoc::Left))
        .op(Op::infix(Rule::xo_rel_eq, Assoc::Left)
            | Op::infix(Rule::xo_rel_neq, Assoc::Left)
            | Op::infix(Rule::xo_rel_lt, Assoc::Left)
            | Op::infix(Rule::xo_rel_lte, Assoc::Left)
            | Op::infix(Rule::xo_rel_gt, Assoc::Left)
            | Op::infix(Rule::xo_rel_gte, Assoc::Left))
        .op(Op::infix(Rule::xo_add, Assoc::Left) | Op::infix(Rule::xo_sub, Assoc::Left))
        .op(Op::infix(Rule::xo_mul, Assoc::Left)
            | Op::infix(Rule::xo_div, Assoc::Left)
            | Op::infix(Rule::xo_mod, Assoc::Left))
        .op(Op::infix(Rule::xo_exp, Assoc::Right))
        .op(Op::prefix(Rule::xo_neg))
        .op(Op::prefix(Rule::xo_bool_not))
}
