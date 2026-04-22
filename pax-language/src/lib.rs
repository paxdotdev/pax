#[cfg(feature = "parser")]
use pest::error::Error;
#[cfg(feature = "parser")]
pub use pest::iterators::{Pair, Pairs};

#[cfg(feature = "parser")]
pub use pest::pratt_parser::{Assoc, Op, PrattParser};
#[cfg(feature = "parser")]
pub use pest::{Parser, Span};
#[cfg(feature = "parser")]
pub use pest_derive::Parser;

#[cfg(feature = "parser")]
mod parsing;
#[cfg(feature = "parser")]
pub use parsing::get_pax_pratt_parser;
#[cfg(feature = "parser")]
pub mod formatting;

#[cfg(feature = "parser")]
pub mod deserializer;
pub mod interpreter;
#[cfg(feature = "parser")]
pub use deserializer::from_pax;
#[cfg(feature = "parser")]
pub mod helpers;

#[cfg(feature = "parser")]
pub use interpreter::parse_pax_expression;
pub use interpreter::{computable::Computable, property_resolution::DependencyCollector};

/// Pest parser generated from the Pax grammar.
#[cfg(feature = "parser")]
#[derive(Parser)]
#[grammar = "pax.pest"]
pub struct PaxParser;

#[cfg(feature = "parser")]
fn renamed_rules(rule: &Rule) -> String {
    match rule {
        Rule::EOI => "end of file".to_string(),
        Rule::WHITESPACE => " ".to_string(),
        Rule::comment => "comment".to_string(),
        Rule::pax_component_definition =>  "component".to_string(),
        Rule::root_tag_pair => "component tag, comment".to_string(),
        Rule::any_tag_pair => "component tag, comment".to_string(),
        Rule::open_tag => "opening component tag".to_string(),
        Rule::closing_tag => "closing component tag".to_string(),
        Rule::self_closing_tag => "component tag".to_string(),
        Rule::matched_tag => "component tag".to_string(),
        Rule::inner_nodes => "literal value, expression, component tag".to_string(),
        Rule::identifier => "identifier".to_string(),
        Rule::pascal_identifier => "identifier".to_string(),
        Rule::event_id => "@HANDLER_NAME".to_string(),
        Rule::attribute_key_value_pair => "setting key-value pair".to_string(),
        Rule::attribute_event_binding => "handler binding".to_string(),
        Rule::double_binding => "two-way binding".to_string(),
        Rule::any_template_value => "literal value, literal object, expression, identifier".to_string(),
        Rule::node_inner_content => "literal value or expression".to_string(),
        Rule::string => "double quoted string".to_string(),
        Rule::inner => "string".to_string(),
        Rule::char => "char".to_string(),
        Rule::settings_block_declaration => "settings block".to_string(),
        Rule::selector_block => "selector block".to_string(),
        Rule::literal_object => "literal object".to_string(),
        Rule::selector => "selector (e.g. .CLASS_NAME or #ID_NAME)".to_string(),
        Rule::settings_key_value_pair => "setting key-value pair".to_string(),
        Rule::settings_event_binding => "handler binding".to_string(),
        Rule::settings_key => "setting key (e.g. PROPERTY_NAME: )".to_string(),
        Rule::settings_value => "literal value, literal object, {expression}".to_string(),
        Rule::timeline_block_declaration => "timeline block".to_string(),
        Rule::timeline_block_setting => "timeline block setting".to_string(),
        Rule::timeline_block_setting_value => "timeline block setting value".to_string(),
        Rule::timeline_selector_block => "timeline selector block".to_string(),
        Rule::timeline_selector_body => "timeline selector body".to_string(),
        Rule::timeline_property_key_value_pair => "timeline property key-value pair".to_string(),
        Rule::timeline_inline_value => "@timeline block".to_string(),
        Rule::timeline_track => "timeline track".to_string(),
        Rule::timeline_keyframe => "timeline keyframe".to_string(),
        Rule::timeline_keyframe_value => "timeline keyframe value".to_string(),
        Rule::timeline_marker => "timeline marker (e.g. 24 or 50%)".to_string(),
        Rule::timeline_percent => "percent marker (e.g. 50%)".to_string(),
        Rule::timeline_easing_curve => "timeline easing curve".to_string(),
        Rule::timeline_target => "timeline target".to_string(),
        Rule::timeline_local_target => "self or this".to_string(),
        Rule::timeline_symbol => "identifier".to_string(),
        Rule::literal_function => "function name".to_string(),
        Rule::silent_comma => ",".to_string(),
        Rule::function_list => "function list (e.g. [handle_click, handle_click_again] )".to_string(),
        Rule::literal_value => "literal value".to_string(),
        Rule::literal_boolean => "boolean".to_string(),
        Rule::literal_number_with_unit => "number with unit (e.g. 10px, 10%, 10deg, 10rad )".to_string(),
        Rule::literal_number => "number".to_string(),
        Rule::literal_number_integer => "integer".to_string(),
        Rule::literal_number_float => "float".to_string(),
        Rule::literal_number_unit => "unit (px, %, rad, deg)".to_string(),
        Rule::literal_tuple => "tuple".to_string(),
        Rule::literal_tuple_access => "tuple access".to_string(),
        Rule::literal_enum_value => "enum".to_string(),
        Rule::literal_enum_args_list => "enum args list".to_string(),
        Rule::literal_color => "color space function (e.g. rgb(255,255,255) ), color constant (e.g. SLATE )".to_string(),
        Rule::literal_color_space_func => "color space function (e.g. rgb(), hsl(), rgba(), hsla())".to_string(),
        Rule::literal_color_channel => "integers 0-255, 0-100%, or arbitrary numeric deg/rad ".to_string(),
        Rule::xo_color_space_func => "color space function (e.g. rgb(), hsl(), rgba(), hsla())".to_string(),
        Rule::literal_color_const => "color constant (see docs.pax.dev/colors)".to_string(),
        Rule::expression_body => "expression".to_string(),
        Rule::expression_ternary => "ternary expression".to_string(),
        Rule::expression_coalesce => "null coalescing expression".to_string(),
        Rule::expression_binary => "binary expression".to_string(),
        Rule::expression_wrapped => "{ expression }".to_string(),
        Rule::expression_grouped => "( expression )".to_string(),
        Rule::xo_primary => "(expression), color space function, enum, function call, range, tuple, list, literal, identifier".to_string(),
        Rule::xo_prefix => "- , !".to_string(),
        Rule::xo_neg => "-".to_string(),
        Rule::xo_bool_not => "!".to_string(),
        Rule::xo_infix => "+, -, *, /, %%, ^, ==, !=, <, <=, >, >=, &&, ||, ??, ?, :".to_string(),
        Rule::xo_add => "+".to_string(),
        Rule::xo_bool_and => "&&".to_string(),
        Rule::xo_bool_or => "||".to_string(),
        Rule::xo_div => "/".to_string(),
        Rule::xo_exp => "^".to_string(),
        Rule::xo_mod => "%%".to_string(),
        Rule::xo_mul => "*".to_string(),
        Rule::xo_rel_eq => "==".to_string(),
        Rule::xo_rel_gt => ">".to_string(),
        Rule::xo_rel_gte => ">=".to_string(),
        Rule::xo_rel_lt => "<".to_string(),
        Rule::xo_rel_lte => "<=".to_string(),
        Rule::xo_rel_neq => "!=".to_string(),
        Rule::xo_sub => "-".to_string(),
        Rule::xo_null_coalesce => "??".to_string(),
        Rule::xo_tern_then => "?".to_string(),
        Rule::xo_tern_else => ":".to_string(),
        Rule::xo_range => "range (e.g. 0..5 or i..j)".to_string(),
        Rule::xo_range_exclusive => "..".to_string(),
        Rule::xo_literal => "literal value".to_string(),
        Rule::xo_object => "literal object".to_string(),
        Rule::xo_object_settings_key_value_pair => "setting key-value pair".to_string(),
        Rule::xo_symbol => "identifier".to_string(),
        Rule::xo_tuple => "tuple (e.g. (1,2) )".to_string(),
        Rule::xo_list => "list (e.g. [1,2] )".to_string(),
        Rule::xo_enum_or_function_call => "enum, function call".to_string(),
        Rule::xo_enum_or_function_args_list => "args list".to_string(),
        Rule::statement_control_flow => "if, for, slot".to_string(),
        Rule::statement_if => "if".to_string(),
        Rule::statement_if_branch => "if branch".to_string(),
        Rule::statement_else_if_branch => "else if branch".to_string(),
        Rule::statement_else_branch => "else branch".to_string(),
        Rule::statement_for => "for".to_string(),
        Rule::statement_slot => "slot".to_string(),
        Rule::statement_for_predicate_declaration => "for predicate (e.g. i, (elem,i) )".to_string(),
        Rule::statement_for_source => "for source (e.g. 0..5 )".to_string(),
        Rule::statement_for_key => "for key expression (e.g. key item.id )".to_string(),
        Rule::literal_list => "list".to_string(),
        Rule::literal_option => "option".to_string(),
        Rule::literal_some => "Some".to_string(),
        Rule::literal_none => "None".to_string(),
        Rule::literal_list_access => "list access".to_string(),
    }
}

/// Parse a string against a single Pax grammar rule, returning a human-readable error string.
#[cfg(feature = "parser")]
pub fn parse_pax_str(expected_rule: Rule, input: &str) -> Result<Pair<'_, Rule>, String> {
    let pairs = PaxParser::parse(expected_rule, input);
    match pairs {
        Ok(mut pairs) => {
            let pair = pairs.next().unwrap();
            Ok(pair)
        }
        Err(err) => {
            let named_error = err.renamed_rules(renamed_rules);
            Err(format!("{named_error}"))
        }
    }
}

/// Parse a string against a Pax grammar rule, preserving the structured pest error.
#[cfg(feature = "parser")]
pub fn parse_pax_err(expected_rule: Rule, input: &str) -> Result<Pair<'_, Rule>, Error<Rule>> {
    let pairs = PaxParser::parse(expected_rule, input);
    match pairs {
        Ok(mut pairs) => {
            let pair = pairs.next().unwrap();
            Ok(pair)
        }
        Err(err) => {
            let named_error = err.renamed_rules(renamed_rules);
            Err(named_error)
        }
    }
}

/// Parse a string into pest pairs for a Pax grammar rule.
#[cfg(feature = "parser")]
pub fn parse_pax_pairs(expected_rule: Rule, input: &str) -> Result<Pairs<'_, Rule>, Error<Rule>> {
    let pairs = PaxParser::parse(expected_rule, input);
    match pairs {
        Ok(pairs) => Ok(pairs),
        Err(err) => {
            let named_error = err.renamed_rules(renamed_rules);
            Err(named_error)
        }
    }
}
