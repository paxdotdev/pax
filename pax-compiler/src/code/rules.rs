use crate::parsing::Rule;
use std::{collections::HashMap, vec};
use lazy_static::lazy_static;
use pest::iterators::Pair;

const LINE_LIMIT: usize = 80;
const INDENTATION: usize = 4;

pub const PREFIX_OPERATORS: [Rule; 2] = [Rule::xo_neg, Rule::xo_bool_not];

pub const INFIX_OPERATORS: [Rule; 16] = [
    Rule::xo_add,
    Rule::xo_bool_and,
    Rule::xo_bool_or,
    Rule::xo_div,
    Rule::xo_exp,
    Rule::xo_mod,
    Rule::xo_mul,
    Rule::xo_rel_eq,
    Rule::xo_rel_gt,
    Rule::xo_rel_gte,
    Rule::xo_rel_lt,
    Rule::xo_rel_lte,
    Rule::xo_rel_neq,
    Rule::xo_sub,
    Rule::xo_tern_then,
    Rule::xo_tern_else,
];

pub const PRIMARY_OPERANDS: [Rule; 8] = [
    Rule::expression_grouped, 
    Rule::xo_function_call, 
    Rule::xo_object, 
    Rule::xo_range, 
    Rule::xo_tuple, 
    Rule::xo_list, 
    Rule::xo_literal,  
    Rule::xo_symbol,
];

pub fn apply_formatting_rules(pair: Pair<Rule>) -> String{
    let children = pair.clone().into_inner();
    let mut formatted_children: Vec<Child> = Vec::new();

    for child in children {
        formatted_children.push(Child::new(child.as_rule(), apply_formatting_rules(child)));
    }
    
    let formatting_rules = get_formatting_rules(pair.as_rule());
    let applicable_rules = {
        let mut applicable_rules = Vec::new();
        for rule in formatting_rules {
            if rule.is_applicable(formatted_children.clone()) {
                applicable_rules.push(rule);
            }
        }
        applicable_rules
    };

    if applicable_rules.len() == 0 {
        panic!("No applicable formatting rule found for {:?}", pair.as_rule());
    }
    applicable_rules.first().unwrap().format(pair, formatted_children)
}

fn get_formatting_rules(pest_rule: Rule) -> Vec<Box<dyn FormattingRule>> {
    match pest_rule {
        Rule::pax_component_definition => vec![Box::new(PaxComponentDefinitionDefaultRule)],
        Rule::open_tag => vec![Box::new(OpenTagDefaultRule)],
        Rule::self_closing_tag => vec![Box::new(SelfClosingTagDefaultRule)],
        Rule::matched_tag => vec![Box::new(MatchTagDefaultRule)],
        Rule::inner_nodes => vec![Box::new(InnerNodesDefaultRule)],
        Rule::attribute_key_value_pair => vec![Box::new(AttributeKeyValuePairDefaultRule)],
        Rule::attribute_event_binding => vec![Box::new(AttributeEventBindingDefaultRule)],
        Rule::settings_block_declaration => vec![Box::new(SettingsBlockDeclarationDefaultRule)],
        Rule::selector_block => vec![Box::new(SelectorBlockDefaultRule)],
        Rule::literal_object | Rule::xo_object => vec![Box::new(ObjectDefaultRule)],
        Rule::settings_key_value_pair => vec![Box::new(SettingsKeyValuePairDefaultRule)],
        Rule::handlers_block_declaration => vec![Box::new(HandlersBlockDeclarationDefaultRule)],
        Rule::handlers_key_value_pair => vec![Box::new(HandlersKeyValuePairDefaultRule)],
        Rule::literal_function => vec![Box::new(LiteralFunctionDefaultRule)],
        Rule::function_list | Rule::xo_list => vec![Box::new(ListMultiLineRule), Box::new(ListDefaultRule)],
        Rule::literal_tuple | Rule::xo_tuple => vec![Box::new(TupleMultiLineRule), Box::new(TupleDefaultRule)],
        Rule::literal_enum_value | Rule::xo_function_call => vec![Box::new(IdentifierCallMultiLineRule), Box::new(IdentifierCallDefaultRule)],
        Rule::literal_enum_args_list 
        | Rule::xo_function_args_list => vec![Box::new(ArgsListMultiLineRule), Box::new(ArgsListDefaultRule)],
        Rule::expression_body => vec![Box::new(ExpressionBodyMultiLineRule), Box::new(ExpressionBodyDefaultRule)],
        Rule::expression_grouped => vec![Box::new(ExpressionGroupedDefaultRule)],
        Rule::xo_object_settings_key_value_pair => vec![Box::new(XoObjectSettingsKeyValuePairDefaultRule)],
        Rule::statement_for => vec![Box::new(StatementForDefaultRule)],
        Rule::statement_if => vec![Box::new(StatementIfDefaultRule)],
        Rule::statement_slot => vec![Box::new(StatementSlotDefaultRule)],

        Rule::any_template_value | 
        Rule::node_inner_content | 
        Rule::settings_value => vec![Box::new(WrapExpressionRule), Box::new(ForwardRule)],
        

        Rule::root_tag_pair |
        Rule::xo_literal |
        Rule::attribute_event_id |
        Rule::handlers_value |
        Rule::literal_value =>  vec![Box::new(ForwardRule)],

        Rule::selector => vec![Box::new(RemoveWhitespaceRule)],
        Rule::settings_key => vec![Box::new(RemoveWhitespaceRule)],
        Rule::handlers_key => vec![Box::new(RemoveWhitespaceRule)],
        Rule::literal_number_with_unit => vec![Box::new(RemoveWhitespaceRule)],
        Rule::literal_number => vec![Box::new(RemoveWhitespaceRule)],
        Rule::literal_number_integer => vec![Box::new(RemoveWhitespaceRule)],
        Rule::literal_number_float => vec![Box::new(RemoveWhitespaceRule)],
        Rule::literal_number_unit => vec![Box::new(RemoveWhitespaceRule)],
        Rule::literal_boolean => vec![Box::new(RemoveWhitespaceRule)],
        Rule::literal_tuple_access => vec![Box::new(RemoveWhitespaceRule)],
        Rule::closing_tag => vec![Box::new(RemoveWhitespaceRule)],
        
        Rule::inner_tag_error => vec![Box::new(PrintRule)],
        Rule::identifier => vec![Box::new(PrintRule)],
        Rule::pascal_identifier => vec![Box::new(PrintRule)],
        Rule::string => vec![Box::new(PrintRule)],
        Rule::selector_block_error => vec![Box::new(PrintRule)],
        Rule::handler_key_value_pair_error => vec![Box::new(PrintRule)],
        Rule::attribute_key_value_pair_error => vec![Box::new(PrintRule)],
        Rule::tag_error =>  vec![Box::new(PrintRule)],
        Rule::open_tag_error =>  vec![Box::new(PrintRule)],
        Rule::block_level_error => vec![Box::new(PrintRule)],
        Rule::statement_for_predicate_declaration => vec![Box::new(PrintRule)],
        Rule::statement_for_source => vec![Box::new(PrintRule)],
        Rule::expression_body_error => vec![Box::new(PrintRule)],
        Rule::xo_neg => vec![Box::new(PrintRule)],
        Rule::xo_bool_not => vec![Box::new(PrintRule)],
        Rule::xo_add => vec![Box::new(PrintRule)],
        Rule::xo_bool_and => vec![Box::new(PrintRule)],
        Rule::xo_bool_or => vec![Box::new(PrintRule)],
        Rule::xo_div => vec![Box::new(PrintRule)],
        Rule::xo_exp => vec![Box::new(PrintRule)],
        Rule::xo_mod => vec![Box::new(PrintRule)],
        Rule::xo_mul => vec![Box::new(PrintRule)],
        Rule::xo_rel_eq => vec![Box::new(PrintRule)],
        Rule::xo_rel_gt => vec![Box::new(PrintRule)],
        Rule::xo_rel_gte => vec![Box::new(PrintRule)],
        Rule::xo_rel_lt => vec![Box::new(PrintRule)],
        Rule::xo_rel_lte => vec![Box::new(PrintRule)],
        Rule::xo_rel_neq => vec![Box::new(PrintRule)],
        Rule::xo_sub => vec![Box::new(PrintRule)],
        Rule::xo_tern_then => vec![Box::new(PrintRule)],
        Rule::xo_tern_else => vec![Box::new(PrintRule)],
        Rule::xo_range => vec![Box::new(PrintRule)],
        Rule::xo_range_exclusive => vec![Box::new(PrintRule)],
        Rule::expression_wrapped => vec![Box::new(UnreachableRule)],
        Rule::xo_primary => vec![Box::new(UnreachableRule)],
        Rule::xo_prefix => vec![Box::new(UnreachableRule)],
        Rule::xo_infix => vec![Box::new(UnreachableRule)],
        Rule::inner => vec![Box::new(UnreachableRule)],
        Rule::char => vec![Box::new(UnreachableRule)],
        Rule::any_tag_pair => vec![Box::new(UnreachableRule)],
        Rule::EOI => vec![Box::new(UnreachableRule)],
        Rule::WHITESPACE => vec![Box::new(UnreachableRule)],
        Rule::COMMENT => vec![Box::new(UnreachableRule)],
        Rule::empty => vec![Box::new(UnreachableRule)],
        Rule::xo_symbol => vec![Box::new(UnreachableRule)],
        Rule::statement_control_flow => vec![Box::new(UnreachableRule)],
    }
}


#[derive(Clone, Debug)]
struct Child {
    node_type: Rule, 
    formatted_node: String,
}

impl Child {
    fn new(node_type: Rule, formatted_node: String) -> Self {
        Self {
            node_type,
            formatted_node,
        }
    }
}

trait FormattingRule{
    fn is_applicable(&self, _children: Vec<Child> ) -> bool{
        true
    }
    fn format(&self, node: Pair<Rule>,  children: Vec<Child>) -> String;
}

#[derive(Clone)]
struct PaxComponentDefinitionDefaultRule;

impl FormattingRule for PaxComponentDefinitionDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let tags = children.iter().filter(|child| child.node_type == Rule::root_tag_pair);
        let settings = children.iter().filter(|child| child.node_type == Rule::settings_block_declaration);
        let handlers= children.iter().filter(|child| child.node_type == Rule::handlers_block_declaration);
        
        let mut formatted_tags = String::new();
        for (i, tag) in  tags.enumerate() {
            if i > 0{
                formatted_tags.push_str("\n");
            }
            formatted_tags.push_str(&tag.formatted_node);
        }

        let mut formatted_settings = String::new();
        for (i, setting) in  settings.enumerate() {
            if i > 0{
                formatted_settings.push_str("\n");
            }
            formatted_settings.push_str(&setting.formatted_node);
        }

        let mut formatted_handlers = String::new();
        for (i, handler) in  handlers.enumerate() {
            if i > 0{
                formatted_handlers.push_str("\n");
            }
            formatted_handlers.push_str(&handler.formatted_node);
        }

        let component = vec![formatted_tags, formatted_settings, formatted_handlers].join("\n");
        component
    }
}

#[derive(Clone)]
struct OpenTagDefaultRule;

impl FormattingRule for OpenTagDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        formatted_node.push_str("<");
        formatted_node.push_str(&children[0].formatted_node);
        formatted_node.push_str(greedy_append_with_line_limit(children[1..].to_vec(), " ").as_str());
        formatted_node.push_str(">");
        formatted_node
    }
}


#[derive(Clone)]
struct SelfClosingTagDefaultRule;

impl FormattingRule for SelfClosingTagDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        formatted_node.push_str("<");
        formatted_node.push_str(&children[0].formatted_node);
        formatted_node.push_str(greedy_append_with_line_limit(children[1..].to_vec(), " ").as_str());
        formatted_node.push_str("/>");
        formatted_node
    }
}

#[derive(Clone)]
struct MatchTagDefaultRule;

impl FormattingRule for MatchTagDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        let open_tag = &children[0].formatted_node;
        let inner_node = indent_every_line_of_string(children[1].formatted_node.clone());
        let close_tag = &children[2].formatted_node;
        formatted_node.push_str(&format!("{}\n{}\n{}", open_tag, inner_node, close_tag));
        formatted_node
    }
}


#[derive(Clone)]
struct InnerNodesDefaultRule;

impl FormattingRule for InnerNodesDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        let children = children.iter().map(|child| child.formatted_node.clone()).collect::<Vec<String>>().join("\n");
        formatted_node.push_str(&children);
        formatted_node
    }
}


#[derive(Clone)]
struct AttributeKeyValuePairDefaultRule;

impl FormattingRule for AttributeKeyValuePairDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();

        if children.len() == 1 {
            // Event Binding
            formatted_node.push_str(&children[0].formatted_node);
        } else {
            // Setting
            formatted_node.push_str(&children[0].formatted_node);
            formatted_node.push_str("=");
            formatted_node.push_str(&children[1].formatted_node);
        }
        formatted_node
    }
}


#[derive(Clone)]
struct AttributeEventBindingDefaultRule;

impl FormattingRule for AttributeEventBindingDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        let key = children[0].formatted_node.clone();
        let value = children[1].formatted_node.clone();
        formatted_node.push_str(format!("@{}={}", key, value).as_str());
        formatted_node
    }
}


#[derive(Clone)]
struct SettingsBlockDeclarationDefaultRule;

impl FormattingRule for SettingsBlockDeclarationDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        panic!("{:?}", _node);
        let mut formatted_node = String::new();
        let settings = children.iter().map(|child| child.formatted_node.clone()).collect::<Vec<String>>().join("\n");
        let indented_settings = indent_every_line_of_string(settings);
        formatted_node.push_str(format!("@settings {{\n{}\n}}", indented_settings).as_str());
        formatted_node
    }
}


#[derive(Clone)]
struct SelectorBlockDefaultRule;

impl FormattingRule for SelectorBlockDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        let selector = children[0].formatted_node.clone();
        let object = children[1].formatted_node.clone();
        formatted_node.push_str(format!("{} {}", selector, object).as_str());
        formatted_node
    }
}


#[derive(Clone)]
struct RemoveWhitespaceRule;

impl FormattingRule for RemoveWhitespaceRule {
    fn format(&self, node: Pair<Rule>, children: Vec<Child>) -> String {
        let trim_node: String = node.as_str().chars()
        .filter(|c| !c.is_whitespace())
        .collect();
        trim_node
    }
}


#[derive(Clone)]
struct SettingsKeyValuePairDefaultRule;

impl FormattingRule for SettingsKeyValuePairDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        let setting = children.iter().map(|child| child.formatted_node.clone()).collect::<Vec<String>>().join(" ");
        formatted_node.push_str(setting.as_str());
        formatted_node
    }
}


#[derive(Clone)]
struct HandlersBlockDeclarationDefaultRule;

impl FormattingRule for HandlersBlockDeclarationDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        let handlers = children.iter().map(|child| child.formatted_node.clone()).collect::<Vec<String>>().join(",\n");
        let indented_handlers = indent_every_line_of_string(handlers);
        formatted_node.push_str(format!("@handlers {{\n{}\n}}", indented_handlers).as_str());
        formatted_node
    }
}

#[derive(Clone)]
struct HandlersKeyValuePairDefaultRule;

impl FormattingRule for HandlersKeyValuePairDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        let handler = children.iter().map(|child| child.formatted_node.clone()).collect::<Vec<String>>().join(" ");
        formatted_node.push_str(handler.as_str());
        formatted_node
    }
}


#[derive(Clone)]
struct LiteralFunctionDefaultRule;

impl FormattingRule for LiteralFunctionDefaultRule {
    fn format(&self, node: Pair<Rule>, _children: Vec<Child>) -> String {
        node.as_str().trim_end_matches(",").trim().to_string()
    }
}


#[derive(Clone)]
struct ListMultiLineRule;

impl FormattingRule for ListMultiLineRule {
    fn is_applicable(&self, children: Vec<Child> ) -> bool {
        has_multi_line_children(&children)    
    }

    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        let functions = children.iter().map(|child| child.formatted_node.clone()).collect::<Vec<String>>().join(",\n");
        let indented_functions = indent_every_line_of_string(functions);
        formatted_node.push_str(format!("[\n{}\n]", indented_functions).as_str());
        formatted_node
    }
}


#[derive(Clone)]
struct ListDefaultRule;

impl FormattingRule for ListDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        formatted_node.push_str("[");
        formatted_node.push_str(greedy_append_with_line_limit(children, ",").as_str());
        formatted_node.push_str("]");
        formatted_node
    }
}


#[derive(Clone)]
struct TupleMultiLineRule;

impl FormattingRule for TupleMultiLineRule {
    fn is_applicable(&self, children: Vec<Child> ) -> bool {
        has_multi_line_children(&children)
    }

    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        let elements = children.iter().map(|child| child.formatted_node.clone()).collect::<Vec<String>>().join(",\n");
        let indented_elements = indent_every_line_of_string(elements);
        formatted_node.push_str(format!("(\n{}\n)", indented_elements).as_str());
        formatted_node
    }
}


#[derive(Clone)]
struct TupleDefaultRule;

impl FormattingRule for TupleDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        formatted_node.push_str("(");
        formatted_node.push_str(greedy_append_with_line_limit(children, ",").as_str());
        formatted_node.push_str(")");
        formatted_node
    }
}


#[derive(Clone)]
struct IdentifierCallMultiLineRule;

impl FormattingRule for IdentifierCallMultiLineRule {
    fn is_applicable(&self, children: Vec<Child> ) -> bool {
        has_multi_line_children(&children)
    }

    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        for child in children {
            if child.node_type == Rule::pascal_identifier || child.node_type == Rule::identifier  {
                if formatted_node.len() > 0 {
                    formatted_node.push_str("::");
                }
                formatted_node.push_str(&child.formatted_node);
            } else if child.node_type == Rule::literal_enum_args_list || child.node_type == Rule::xo_function_args_list {
                formatted_node.push_str("(\n");
                let indented_child = indent_every_line_of_string(child.formatted_node.clone());
                formatted_node.push_str(&indented_child);
                formatted_node.push_str("\n)");
            }
        }
        formatted_node
    }
}


#[derive(Clone)]
struct IdentifierCallDefaultRule;

impl FormattingRule for IdentifierCallDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        for child in children {
            if child.node_type == Rule::pascal_identifier || child.node_type == Rule::identifier {
                if formatted_node.len() > 0 {
                    formatted_node.push_str("::");
                }
                formatted_node.push_str(&child.formatted_node);
            } else if child.node_type == Rule::literal_enum_args_list || child.node_type == Rule::xo_function_args_list {
                formatted_node.push_str("(");
                formatted_node.push_str(&child.formatted_node);
                formatted_node.push_str(")");
            }
        }
        formatted_node
    }
}


#[derive(Clone)]
struct ArgsListMultiLineRule;

impl FormattingRule for ArgsListMultiLineRule {
    fn is_applicable(&self, children: Vec<Child> ) -> bool {
        has_multi_line_children(&children)
    }

    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        let elements = children.iter().map(|child| child.formatted_node.clone()).collect::<Vec<String>>().join(",\n");
        formatted_node.push_str(&elements);
        formatted_node
    }
}


#[derive(Clone)]
struct ArgsListDefaultRule;

impl FormattingRule for ArgsListDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        formatted_node.push_str(greedy_append_with_line_limit(children, ",").as_str());
        formatted_node
    }
}


#[derive(Clone)]
struct ExpressionBodyMultiLineRule;

impl FormattingRule for ExpressionBodyMultiLineRule {
    fn is_applicable(&self, children: Vec<Child> ) -> bool {
        has_multi_line_children(&children) || children_longer_than_line_limit(&children)
    }

    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        formatted_node.push_str("\n");
        for child in children {
            if is_prefix(&child) {
                formatted_node.push_str(&child.formatted_node);
            } else if is_infix(&child) {
                formatted_node.push_str("\n");
                formatted_node.push_str(&child.formatted_node);
                formatted_node.push_str(" ");
            } else if is_primary_operand(&child) {
                formatted_node.push_str(&child.formatted_node);
            }
        }
        formatted_node
    }
}


#[derive(Clone)]
struct ExpressionBodyDefaultRule;

impl FormattingRule for ExpressionBodyDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        for child in children {
            if is_prefix(&child) {
                formatted_node.push_str(&child.formatted_node);
            } else if is_infix(&child) {
                formatted_node.push_str(" ");
                formatted_node.push_str(&child.formatted_node);
                formatted_node.push_str(" ");
            } else if is_primary_operand(&child) {
                formatted_node.push_str(&child.formatted_node);
            }
        }
        formatted_node
    }
}


#[derive(Clone)]
struct ExpressionGroupedDefaultRule;

impl FormattingRule for ExpressionGroupedDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        let literal_number_unit = if let Some(unit) = children.iter().find(|child| 
            child.node_type == Rule::literal_number_unit) {
            unit.formatted_node.clone()
        } else {
            String::new()
        };

        formatted_node.push_str(format!("({}){}", children[0].formatted_node, literal_number_unit).as_str());
        formatted_node
    }
}


#[derive(Clone)]
struct ObjectDefaultRule;

impl FormattingRule for ObjectDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        let pascal_identifier = if let Some(identifier) = children.iter().find(|child| 
            child.node_type == Rule::pascal_identifier || child.node_type == Rule::identifier) {
            identifier.formatted_node.clone() + " "
        } else {
            String::new()
        };

        let settings_pairs = children.iter().filter(|child| 
            child.node_type == Rule::xo_object_settings_key_value_pair || child.node_type == Rule::settings_key_value_pair).map(|child| child.formatted_node.clone()).collect::<Vec<String>>().join(",\n");
        let indented_settings_pairs = indent_every_line_of_string(settings_pairs);
        formatted_node.push_str(format!("{}{{\n{}\n}}", pascal_identifier, indented_settings_pairs).as_str());
        formatted_node
    }
}


#[derive(Clone)]
struct XoObjectSettingsKeyValuePairDefaultRule;

impl FormattingRule for XoObjectSettingsKeyValuePairDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        let setting = children.iter().map(|child| child.formatted_node.clone()).collect::<Vec<String>>().join(" ");
        formatted_node.push_str(setting.as_str());
        formatted_node
    }
}


#[derive(Clone)]
struct StatementForDefaultRule;

impl FormattingRule for StatementForDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        let sfpd = children[0].formatted_node.clone();
        let sfs = children[1].formatted_node.clone();
        let inner_nodes = children[2].formatted_node.clone();
        let inner_nodes_indented = indent_every_line_of_string(inner_nodes);
        formatted_node.push_str(format!("for {} in {} {{\n{}\n}}", sfpd, sfs, inner_nodes_indented).as_str());
        formatted_node
    }
}


#[derive(Clone)]
struct StatementIfDefaultRule;

impl FormattingRule for StatementIfDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        let exp = children[0].formatted_node.clone();
        let inner_nodes = children[1].formatted_node.clone();
        let inner_nodes_indented = indent_every_line_of_string(inner_nodes);
        formatted_node.push_str(format!("if {} {{\n{}\n}}", exp, inner_nodes_indented).as_str());
        formatted_node
    }
}


#[derive(Clone)]
struct StatementSlotDefaultRule;

impl FormattingRule for StatementSlotDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        let exp = children[0].formatted_node.clone();
        formatted_node.push_str(format!("slot {}", exp).as_str());
        formatted_node
    }
}


#[derive(Clone)]
struct ForwardRule;

impl FormattingRule for ForwardRule {
    fn is_applicable(&self, _children: Vec<Child>) -> bool {
        _children.len() > 0
    }

    fn format(&self, node: Pair<Rule>,  children: Vec<Child>) -> String {
        return children.iter().map(|child| child.formatted_node.clone()).collect::<Vec<String>>().join("");
    }
}


#[derive(Clone)]
struct PrintRule;

impl FormattingRule for PrintRule {
    fn format(&self, node: Pair<Rule>,  children: Vec<Child>) -> String {
        return node.as_str().trim().to_string();
    }
}


#[derive(Clone)]
struct UnreachableRule;

impl FormattingRule for UnreachableRule {
    fn format(&self, node: Pair<Rule>,  children: Vec<Child>) -> String {
        return String::new();
    }
}


#[derive(Clone)]
struct WrapExpressionRule; 

impl FormattingRule for WrapExpressionRule {
    fn is_applicable(&self, children: Vec<Child> ) -> bool {
        if children.len() == 1 {
            if children[0].node_type == Rule::expression_body 
                || children[0].node_type == Rule::expression_body_error {
                return true;
            }
        }
        return false;
    }

    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        format!("{{{}}}", children[0].formatted_node)
    }
}

fn is_prefix(child: &Child) -> bool {
    PREFIX_OPERATORS.contains(&child.node_type)
}

fn is_infix(child: &Child) -> bool {
    INFIX_OPERATORS.contains(&child.node_type)
}

fn is_primary_operand(child: &Child) -> bool {
    PRIMARY_OPERANDS.contains(&child.node_type)
}

fn children_longer_than_line_limit(children: &Vec<Child>) -> bool {
    let mut length = 0;
    for child in children {
        length += child.formatted_node.len();
    }
    length > LINE_LIMIT
}

fn has_multi_line_children(children: &Vec<Child>) -> bool {
    for child in children {
        if child.formatted_node.contains('\n') {
            return true;
        }
    }
    false
}


fn greedy_append_with_line_limit(children: Vec<Child>, seperator: &str) -> String {
    let mut formatted_node = String::new();
    for (i, child) in children.iter().enumerate() {
        let child_lines: Vec<&str> = child.formatted_node.split('\n').collect();
        let first_line_of_child = child_lines[0].len();
        let current_line = if let Some(line) = formatted_node.lines().last() {
            line.to_string()
        } else {
            formatted_node.clone()
        };

        if current_line.len() + first_line_of_child > LINE_LIMIT {
            formatted_node.push_str("\n");
            let indented_child = indent_every_line_of_string(child.formatted_node.clone());
            formatted_node.push_str(&indented_child);
        } else {
            if child_lines.len() > 1 {
                formatted_node.push_str(seperator);
                formatted_node.push_str(&child_lines[0]);
                for line in child_lines.iter().skip(1) {
                    formatted_node.push_str("\n");
                    formatted_node.push_str(&n_indentation_string(1));
                    formatted_node.push_str(&line);
                }
            } else {
                formatted_node.push_str(seperator);
                formatted_node.push_str(&child.formatted_node);
            }
        }
    }
    formatted_node
}

fn n_indentation_string(n: usize) -> String {
    " ".repeat(INDENTATION).repeat(n)
}

fn indent_every_line_of_string(string: String) -> String {
    let mut result = String::new();
    for line in string.lines() {
        result.push_str(&n_indentation_string(1));
        result.push_str(line);
        result.push_str("\n");
    }
    result.trim_end_matches("\n").to_string()
}