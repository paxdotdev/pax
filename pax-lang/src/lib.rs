use pest::error::Error;
pub use pest::iterators::{Pair, Pairs};

pub use pest::pratt_parser::{Assoc, Op, PrattParser};
pub use pest::{Parser, Span};
pub use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "pax.pest"]
pub struct PaxParser;

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
        Rule::id => "id".to_string(),
        Rule::id_binding => "id binding (e.g. id=ID_SELECTOR )".to_string(),
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
        Rule::expression_wrapped => "{ expression }".to_string(),
        Rule::expression_grouped => "( expression )".to_string(),
        Rule::xo_primary => "(expression), color space function, enum, function call, range, tuple, list, literal, identifier".to_string(),
        Rule::xo_prefix => "- , !".to_string(),
        Rule::xo_neg => "-".to_string(),
        Rule::xo_bool_not => "!".to_string(),
        Rule::xo_infix => "+, -, *, /, %, ^, ==, !=, <, <=, >, >=, &&, ||".to_string(),
        Rule::xo_add => "+".to_string(),
        Rule::xo_bool_and => "&&".to_string(),
        Rule::xo_bool_or => "||".to_string(),
        Rule::xo_div => "/".to_string(),
        Rule::xo_exp => "^".to_string(),
        Rule::xo_mod => "%".to_string(),
        Rule::xo_mul => "*".to_string(),
        Rule::xo_rel_eq => "==".to_string(),
        Rule::xo_rel_gt => ">".to_string(),
        Rule::xo_rel_gte => ">=".to_string(),
        Rule::xo_rel_lt => "<".to_string(),
        Rule::xo_rel_lte => "<=".to_string(),
        Rule::xo_rel_neq => "!=".to_string(),
        Rule::xo_sub => "-".to_string(),
        Rule::xo_tern_then => "then".to_string(),
        Rule::xo_tern_else => "else".to_string(),
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
        Rule::statement_for => "for".to_string(),
        Rule::statement_slot => "slot".to_string(),
        Rule::statement_for_predicate_declaration => "for predicate (e.g. i, (elem,i) )".to_string(),
        Rule::statement_for_source => "for source (e.g. 0..5 )".to_string(),
        Rule::literal_list => "list".to_string(),
        Rule::literal_option => "option".to_string(),
        Rule::literal_some => "Some".to_string(),
        Rule::literal_none => "None".to_string(),
    }
}

pub fn parse_pax_str(expected_rule: Rule, input: &str) -> Result<Pair<Rule>, String> {
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

pub fn parse_pax_err(expected_rule: Rule, input: &str) -> Result<Pair<Rule>, Error<Rule>> {
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
