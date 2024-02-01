use std::cell::RefCell;
use std::rc::Rc;

use crate::{
    escape_identifier, LiteralBlockDefinition, LocationInfo, SettingElement, Token, TokenType,
    ValueDefinition,
};

use pest::iterators::{Pair, Pairs};
use pest::Parser;
use pest_derive::Parser;

use pest::pratt_parser::{Assoc, Op, PrattParser};

#[derive(Parser)]
#[grammar = "pax.pest"]
pub struct PaxParser;

const NO_LOC: LocationInfo = LocationInfo {
    start_line_col: (0, 0),
    end_line_col: (0, 0),
};
const NO_PAX: &str = "None";

//What do to with location info?
//a lot of functionality is copied atm
pub fn to_value_definition(raw_value: &str) -> Option<ValueDefinition> {
    let mut value = PaxParser::parse(Rule::any_template_value, raw_value).ok()?; //parse using the normal rules
    let value = value.next().unwrap().into_inner().next().unwrap();
    Some(match value.as_rule() {
        Rule::literal_value => {
            //we want to pratt-parse literals, mostly to unpack `px` and `%` (recursively)
            let (output_string, _) = run_pratt_parser(value.as_str());
            let literal_value_token = Token::new_with_raw_value(
                output_string,
                raw_value.to_string(),
                TokenType::LiteralValue,
                NO_LOC,
                NO_PAX,
            );
            ValueDefinition::LiteralValue(literal_value_token)
        }
        Rule::literal_object => {
            ValueDefinition::Block(derive_value_definition_from_literal_object_pair(value))
        }
        Rule::expression_body => {
            let expression_token = Token::new_with_raw_value(
                value.as_str().to_string(),
                raw_value.to_string(),
                TokenType::Expression,
                NO_LOC,
                NO_PAX,
            );
            ValueDefinition::Expression(expression_token, None)
        }
        Rule::identifier => {
            let identifier_token = Token::new(
                value.as_str().to_string(),
                TokenType::Identifier,
                NO_LOC,
                NO_PAX,
            );
            ValueDefinition::Identifier(identifier_token, None)
        }
        _ => {
            unreachable!("Parsing error 3342638857230: {:?}", value.as_rule());
        }
    })
}

//--------------------------------------------------------------------------------------------
// Everything below is taken from pax-compiler, and should be consolidated at some point
//--------------------------------------------------------------------------------------------

/// Returns (RIL output string, `symbolic id`s found during parse)
/// where a `symbolic id` may be something like `self.num_clicks` or `i`
fn run_pratt_parser(input_paxel: &str) -> (String, Vec<String>) {
    // Operator precedence is declared via the ordering here:
    let pratt = PrattParser::new()
        .op(Op::infix(Rule::xo_tern_then, Assoc::Left)
            | Op::infix(Rule::xo_tern_else, Assoc::Right))
        .op(Op::infix(Rule::xo_bool_and, Assoc::Left) | Op::infix(Rule::xo_bool_or, Assoc::Left))
        .op(Op::infix(Rule::xo_add, Assoc::Left) | Op::infix(Rule::xo_sub, Assoc::Left))
        .op(Op::infix(Rule::xo_mul, Assoc::Left) | Op::infix(Rule::xo_div, Assoc::Left))
        .op(Op::infix(Rule::xo_mod, Assoc::Left))
        .op(Op::infix(Rule::xo_exp, Assoc::Right))
        .op(Op::prefix(Rule::xo_neg))
        .op(Op::infix(Rule::xo_rel_eq, Assoc::Left)
            | Op::infix(Rule::xo_rel_neq, Assoc::Left)
            | Op::infix(Rule::xo_rel_lt, Assoc::Left)
            | Op::infix(Rule::xo_rel_lte, Assoc::Left)
            | Op::infix(Rule::xo_rel_gt, Assoc::Left)
            | Op::infix(Rule::xo_rel_gte, Assoc::Left))
        .op(Op::prefix(Rule::xo_bool_not));

    let pairs = PaxParser::parse(Rule::expression_body, input_paxel)
        .expect(&format!("unsuccessful pratt parse {}", &input_paxel));

    let symbolic_ids = Rc::new(RefCell::new(vec![]));
    let output = recurse_pratt_parse_to_string(pairs, &pratt, Rc::clone(&symbolic_ids));
    (output, symbolic_ids.take())
}

fn derive_value_definition_from_literal_object_pair(
    literal_object: Pair<Rule>,
) -> LiteralBlockDefinition {
    let mut literal_object_pairs = literal_object.into_inner();

    if let None = literal_object_pairs.peek() {
        return LiteralBlockDefinition {
            explicit_type_pascal_identifier: None,
            elements: vec![],
        };
    }

    let explicit_type_pascal_identifier = match literal_object_pairs.peek().unwrap().as_rule() {
        Rule::pascal_identifier => {
            let raw_value = literal_object_pairs.next().unwrap();
            let token = Token::new(
                raw_value.as_str().to_string(),
                TokenType::PascalIdentifier,
                NO_LOC,
                NO_PAX,
            );
            Some(token)
        }
        _ => None,
    };

    LiteralBlockDefinition {
        explicit_type_pascal_identifier,
        elements: literal_object_pairs
            .map(|settings_key_value_pair| {
                match settings_key_value_pair.as_rule() {
                    Rule::settings_key_value_pair => {
                        let mut pairs = settings_key_value_pair.into_inner();
                        let setting_key = pairs.next().unwrap().into_inner().next().unwrap();
                        let setting_key_token = Token::new(
                            setting_key.as_str().to_string(),
                            TokenType::SettingKey,
                            NO_LOC,
                            NO_PAX,
                        );
                        let raw_value = pairs.peek().unwrap().as_str();
                        let value = pairs.next().unwrap().into_inner().next().unwrap();
                        let setting_value_definition = match value.as_rule() {
                            Rule::literal_value => {
                                //we want to pratt-parse literals, mostly to unpack `px` and `%` (recursively)
                                let (output_string, _) = run_pratt_parser(value.as_str());
                                let token = Token::new_with_raw_value(
                                    output_string,
                                    raw_value.to_string(),
                                    TokenType::LiteralValue,
                                    NO_LOC,
                                    NO_PAX,
                                );
                                ValueDefinition::LiteralValue(token)
                            }
                            Rule::literal_object => {
                                ValueDefinition::Block(
                                    //Recurse
                                    derive_value_definition_from_literal_object_pair(value),
                                )
                            }
                            // Rule::literal_enum_value => {ValueDefinition::Enum(raw_value.as_str().to_string())},
                            Rule::expression_body => {
                                let token = Token::new_with_raw_value(
                                    value.as_str().to_string(),
                                    raw_value.to_string(),
                                    TokenType::Expression,
                                    NO_LOC,
                                    NO_PAX,
                                );
                                ValueDefinition::Expression(token, None)
                            }
                            _ => {
                                unreachable!("Parsing error 231453468: {:?}", value.as_rule());
                            }
                        };

                        SettingElement::Setting(setting_key_token, setting_value_definition)
                    }
                    Rule::comment => {
                        let comment = settings_key_value_pair.as_str().to_string();
                        SettingElement::Comment(comment)
                    }
                    _ => {
                        unreachable!(
                            "Parsing error 2314314145: {:?}",
                            settings_key_value_pair.as_rule()
                        );
                    }
                }
            })
            .collect(),
    }
}

/// Workhorse method for compiling Expressions into Rust Intermediate Language (RIL, a string of Rust)
fn recurse_pratt_parse_to_string<'a>(
    expression: Pairs<Rule>,
    pratt_parser: &PrattParser<Rule>,
    symbolic_ids: Rc<RefCell<Vec<String>>>,
) -> String {
    pratt_parser
        .map_primary(move |primary| match primary.as_rule() {
            /* expression_grouped | xo_enum_or_function_call | xo_range     */
            Rule::expression_grouped => {
                /* expression_grouped = { "(" ~ expression_body ~ ")" ~ literal_number_unit? } */
                let mut inner = primary.into_inner();

                let exp_bod = recurse_pratt_parse_to_string(inner.next().unwrap().into_inner(), pratt_parser, Rc::clone(&symbolic_ids));
                if let Some(literal_number_unit) = inner.next() {
                    let unit = literal_number_unit.as_str();

                    if unit == "px" {
                        format!("Size::Pixels({}.into())", exp_bod)
                    } else if unit == "%" {
                        format!("Size::Percent({}.into())", exp_bod)
                    } else if unit == "deg" {
                        format!("Rotation::Degrees({}.into())", exp_bod)
                    } else if unit == "rad" {
                        format!("Rotation::Radians({}.into())", exp_bod)
                    } else {
                        unreachable!()
                    }
                } else {
                    exp_bod
                }
            },
            Rule::xo_enum_or_function_call => {
                /* xo_enum_or_function_call = {identifier ~ (("::") ~ identifier)* ~ ("("~xo_enum_or_function_args_list~")")}
                   xo_enum_or_function_args_list = {expression_body ~ ("," ~ expression_body)*} */

                //prepend identifiers; recurse-pratt-parse `xo_function_args`' `expression_body`s
                let mut pairs = primary.into_inner();

                let mut output = "".to_string();
                let mut next_pair = pairs.next().unwrap();
                while let Rule::identifier = next_pair.as_rule() {
                    output = output + next_pair.as_str();
                    next_pair = pairs.next().unwrap();
                    if let Rule::identifier = next_pair.as_rule() {
                        //look-ahead
                        output = output + "::";
                    }
                };

                let mut expression_body_pairs = next_pair.into_inner();

                output = output + "(";
                while let Some(next_pair) = expression_body_pairs.next() {
                    output = output + "(" + &recurse_pratt_parse_to_string(next_pair.into_inner(), pratt_parser, Rc::clone(&symbolic_ids)) + "),"
                }
                output = output + ")";

                output
            },
            Rule::xo_range => {
                /* { op0: (xo_literal | xo_symbol) ~ op1: (xo_range_inclusive | xo_range_exclusive) ~ op2: (xo_literal | xo_symbol)} */
                let mut pairs = primary.into_inner();

                let op0 = pairs.next().unwrap();

                let op0_out = match op0.as_rule() {
                    Rule::xo_literal => {
                        //return the literal exactly as it is
                        op0.as_str().to_string()
                    },
                    Rule::xo_symbol => {
                        symbolic_ids.borrow_mut().push(op0.as_str().to_string());
                        //for symbolic identifiers, remove any "this" or "self", then return string
                        format!("{}.get_as_int()",convert_symbolic_binding_from_paxel_to_ril(op0))
                    },
                    _ => unimplemented!("")
                };

                let op1 = pairs.next().unwrap();
                let op1_out = op1.as_str().to_string();

                let op2 = pairs.next().unwrap();
                let op2_out = match op2.as_rule() {
                    Rule::xo_literal => {
                        //return the literal exactly as it is
                        op2.as_str().to_string()
                    },
                    Rule::xo_symbol => {
                        symbolic_ids.borrow_mut().push(op2.as_str().to_string());
                        //for symbolic identifiers, remove any "this" or "self", then return string
                        format!("{}.get_as_int()",convert_symbolic_binding_from_paxel_to_ril(op2))
                    },
                    _ => unimplemented!("")
                };

                format!("({} as isize){}({} as isize)", &op0_out, &op1_out, &op2_out)
            },
            Rule::xo_literal => {
                let literal_kind = primary.into_inner().next().unwrap();

                match literal_kind.as_rule() {
                    Rule::literal_number_with_unit => {
                        let mut inner = literal_kind.into_inner();

                        let value = inner.next().unwrap().as_str();
                        let unit = inner.next().unwrap().as_str();

                        if unit == "px" {
                            format!("Size::Pixels({}.into())", value)
                        } else if unit == "%" {
                            format!("Size::Percent({}.into())", value)
                        } else if unit == "deg" {
                            format!("Rotation::Degrees({}.into())", value)
                        } else if unit == "rad" {
                            format!("Rotation::Radians({}.into())", value)
                        } else {
                            unreachable!()
                        }
                    },
                    Rule::literal_number => {
                        let mut inner = literal_kind.into_inner();
                        let value = inner.next().unwrap().as_str();
                        format!("Numeric::from({})", value)
                    },
                    Rule::string => {
                        format!("StringBox::from({}).into()",literal_kind.as_str().to_string())
                    },
                    _ => {
                        /* {literal_enum_value | literal_tuple_access | literal_tuple | string } */
                        literal_kind.as_str().to_string()
                    }
                }
            },
            Rule::xo_object => {
                let mut output : String = "".to_string();

                let mut inner = primary.into_inner();
                let maybe_identifier = inner.next().unwrap();
                let rule = maybe_identifier.as_rule();

                //for parsing xo_object_settings_key_value_pair
                //iterate over key-value pairs; recurse into expressions
                fn handle_xoskvp<'a>(xoskvp: Pair<Rule>, pratt_parser: &PrattParser<Rule>, symbolic_ids: Rc<RefCell<Vec<String>>>) -> String {
                    let mut inner_kvp = xoskvp.into_inner();
                    let settings_key = inner_kvp.next().unwrap().as_str().to_string();
                    let expression_body = inner_kvp.next().unwrap().into_inner();

                    let ril = recurse_pratt_parse_to_string(expression_body, pratt_parser, Rc::clone(&symbolic_ids));
                    format!("{}: {},\n",settings_key, ril)
                }

                if let Rule::identifier = rule {
                    //explicit type declaration, like `SomeType {...}`
                    unimplemented!("Explicit struct type declarations are not yet supported.  Instead of `SomeType {{ ... }}`, try using simply `{{ ... }}`.");
                } else {
                    //no explicit type declaration, like `{...}`
                    // -- this token is the first k/v pair of object declaration; handle as such
                    let ril = handle_xoskvp(maybe_identifier, pratt_parser, Rc::clone(&symbolic_ids));
                    output += &ril;
                }

                let mut remaining_kvps = inner.into_iter();

                while let Some(xoskkvp) = remaining_kvps.next() {
                    let ril =  handle_xoskvp(xoskkvp, pratt_parser, Rc::clone(&symbolic_ids));
                    output += &ril;
                }

                output
            },
            Rule::xo_symbol => {
                symbolic_ids.borrow_mut().push(primary.as_str().to_string());
                format!("{}",convert_symbolic_binding_from_paxel_to_ril(primary))
            },
            Rule::xo_tuple => {
                let mut tuple = primary.into_inner();
                let exp0 = tuple.next().unwrap();
                let exp1 = tuple.next().unwrap();
                let exp0 = recurse_pratt_parse_to_string( exp0.into_inner(), pratt_parser, Rc::clone(&symbolic_ids));
                let exp1 = recurse_pratt_parse_to_string( exp1.into_inner(), pratt_parser, Rc::clone(&symbolic_ids));
                format!("({},{})", exp0, exp1)
            },
            Rule::xo_list => {
                let mut list = primary.into_inner();
                let mut vec = Vec::new();

                while let Some(item) = list.next() {
                    let item_str = recurse_pratt_parse_to_string(item.into_inner(), pratt_parser, Rc::clone(&symbolic_ids));
                    vec.push(item_str);
                }
                format!("vec![{}]", vec.join(","))
            },
            Rule::expression_body => {
                recurse_pratt_parse_to_string(primary.into_inner(), pratt_parser, Rc::clone(&symbolic_ids))
            },
            _ => unreachable!("{}",primary.as_str()),
        })
        .map_prefix(|op, rhs| match op.as_rule() {
            Rule::xo_neg => format!("(-{})", rhs),
            Rule::xo_bool_not => format!("(!{})", rhs),
            _ => unreachable!(),
        })
        // .map_postfix(|lhs, op| match op.as_rule() {
        //     Rule::fac => format!("({}!)", lhs),
        //     _ => unreachable!(),
        // })
        .map_infix(|lhs, op, rhs| match op.as_rule() {
            Rule::xo_add => {format!("({}+{})", lhs, rhs)},
            Rule::xo_bool_and => {format!("({}&&{})", lhs, rhs)},
            Rule::xo_bool_or => {format!("({}||{})", lhs, rhs)},
            Rule::xo_div => {format!("({}/{})", lhs, rhs)},
            Rule::xo_exp => {format!("(({}).pow({}))", lhs, rhs)},
            Rule::xo_mod => {format!("({}%{})", lhs, rhs)},
            Rule::xo_mul => {format!("({}*({}).into())", lhs, rhs)},
            Rule::xo_rel_eq => {format!("({}=={})", lhs, rhs)},
            Rule::xo_rel_gt => {format!("({}>{})", lhs, rhs)},
            Rule::xo_rel_gte => {format!("({}>={})", lhs, rhs)},
            Rule::xo_rel_lt => {format!("({}<{})", lhs, rhs)},
            Rule::xo_rel_lte => {format!("({}<={})", lhs, rhs)},
            Rule::xo_rel_neq => {format!("({}!={})", lhs, rhs)},
            Rule::xo_sub => {format!("({}-{})", lhs, rhs)},
            Rule::xo_tern_then => {format!("if {} {{ {} }}", lhs, rhs)},
            Rule::xo_tern_else => {format!("{} else {{ {} }}", lhs, rhs)},
            _ => unreachable!(),
        })
        .parse(expression)
}

/// Removes leading `self.` or `this.`, escapes remaining symbol to be a suitable atomic identifier
fn convert_symbolic_binding_from_paxel_to_ril(xo_symbol: Pair<Rule>) -> String {
    let mut pairs = xo_symbol.clone().into_inner();
    let maybe_this_or_self = pairs.next().unwrap().as_str();

    let self_or_this_removed = if maybe_this_or_self == "this" || maybe_this_or_self == "self" {
        let mut output = "".to_string();

        //accumulate remaining identifiers, having skipped `this` or `self` with the original `.next()`
        pairs.for_each(|pair| output += &*(".".to_owned() + pair.as_str()));

        //remove initial fencepost "."
        output.replacen(".", "", 1)
    } else {
        //remove original binding; no self or this
        xo_symbol.as_str().to_string()
    };

    escape_identifier(self_or_this_removed)
}
