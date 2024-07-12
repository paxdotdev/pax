use core::panic;
use pax_lang::{Pair, Rule};
use std::{collections::VecDeque, vec};

const LINE_LIMIT: usize = 120;
const INDENTATION: usize = 4;

pub const PREFIX_OPERATORS: [Rule; 2] = [Rule::xo_neg, Rule::xo_bool_not];
pub const DO_NOT_INSERT_TAB_MARKER: &str = "|-DO_NOT_INSERT_TAB-|";

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
    Rule::xo_enum_or_function_call,
    Rule::xo_object,
    Rule::xo_range,
    Rule::xo_tuple,
    Rule::xo_list,
    Rule::xo_literal,
    Rule::xo_symbol,
];

pub fn format(component: Pair<Rule>) -> String {
    apply_formatting_rules(component).replace(DO_NOT_INSERT_TAB_MARKER, "")
}

pub fn apply_formatting_rules(pair: Pair<Rule>) -> String {
    let children = pair.clone().into_inner();
    let mut formatted_children: Vec<Child> = Vec::new();

    for child in children {
        let child_formatted = apply_formatting_rules(child.clone());
        let _child = Child::new(child.as_rule(), child_formatted.clone());
        formatted_children.push(_child.clone());
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
        panic!(
            "No applicable formatting rule found for {:?}",
            pair.as_rule()
        );
    }

    applicable_rules
        .first()
        .unwrap()
        .format(pair.clone(), formatted_children)
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
        Rule::settings_event_binding => vec![Box::new(SettingsEventBindingDefaultRule)],
        Rule::selector_block => vec![Box::new(SelectorBlockDefaultRule)],
        Rule::literal_object | Rule::xo_object => vec![Box::new(ObjectDefaultRule)],
        Rule::settings_key_value_pair => vec![Box::new(SettingsKeyValuePairDefaultRule)],
        Rule::literal_function => vec![Box::new(LiteralFunctionDefaultRule)],
        Rule::function_list | Rule::xo_list | Rule::literal_list => {
            vec![Box::new(ListMultiLineRule), Box::new(ListDefaultRule)]
        }
        Rule::literal_tuple | Rule::xo_tuple => {
            vec![Box::new(TupleMultiLineRule), Box::new(TupleDefaultRule)]
        }
        Rule::literal_enum_value | Rule::xo_enum_or_function_call => vec![
            Box::new(IdentifierCallMultiLineRule),
            Box::new(IdentifierCallDefaultRule),
        ],
        Rule::event_id => vec![Box::new(EventIdDefaultRule)],
        Rule::literal_enum_args_list
        | Rule::xo_enum_or_function_args_list
        | Rule::literal_color => vec![
            Box::new(ArgsListMultiLineRule),
            Box::new(ArgsListDefaultRule),
        ],
        Rule::expression_body => vec![
            Box::new(ExpressionBodyMultiLineRule),
            Box::new(ExpressionBodyDefaultRule),
        ],
        Rule::expression_grouped => vec![Box::new(ExpressionGroupedDefaultRule)],
        Rule::xo_object_settings_key_value_pair => {
            vec![Box::new(XoObjectSettingsKeyValuePairDefaultRule)]
        }
        Rule::statement_for => vec![Box::new(StatementForDefaultRule)],
        Rule::statement_if => vec![Box::new(StatementIfDefaultRule)],
        Rule::statement_slot => vec![Box::new(StatementSlotDefaultRule)],
        Rule::any_template_value | Rule::node_inner_content | Rule::settings_value => {
            vec![Box::new(WrapExpressionRule), Box::new(ForwardRule)]
        }
        Rule::root_tag_pair
        | Rule::xo_literal
        | Rule::literal_value
        | Rule::statement_control_flow => vec![Box::new(ForwardRule)],

        Rule::selector
        | Rule::settings_key
        | Rule::literal_number_with_unit
        | Rule::literal_number
        | Rule::literal_number_integer
        | Rule::literal_number_float
        | Rule::literal_number_unit
        | Rule::literal_boolean
        | Rule::literal_tuple_access
        | Rule::closing_tag
        | Rule::xo_symbol
        | Rule::id_binding
        | Rule::literal_color_channel
        | Rule::EOI => vec![Box::new(RemoveWhitespaceRule)],

        Rule::identifier
        | Rule::pascal_identifier
        | Rule::double_binding
        | Rule::statement_for_predicate_declaration
        | Rule::statement_for_source
        | Rule::comment
        | Rule::xo_neg
        | Rule::xo_bool_not
        | Rule::xo_add
        | Rule::xo_bool_and
        | Rule::xo_bool_or
        | Rule::xo_div
        | Rule::xo_exp
        | Rule::xo_mod
        | Rule::xo_mul
        | Rule::xo_rel_eq
        | Rule::xo_rel_gt
        | Rule::xo_rel_gte
        | Rule::xo_rel_lt
        | Rule::xo_rel_lte
        | Rule::xo_rel_neq
        | Rule::xo_sub
        | Rule::xo_tern_then
        | Rule::xo_tern_else
        | Rule::xo_range
        | Rule::literal_color_space_func
        | Rule::xo_color_space_func
        | Rule::literal_color_const
        | Rule::xo_range_exclusive => vec![Box::new(PrintRule)],

        Rule::expression_wrapped
        | Rule::xo_primary
        | Rule::xo_prefix
        | Rule::xo_infix
        | Rule::inner
        | Rule::char
        | Rule::any_tag_pair
        | Rule::WHITESPACE
        | Rule::id
        | Rule::silent_comma => vec![Box::new(IgnoreRule)],

        Rule::string => vec![Box::new(DoNotIndentRule)],
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

trait FormattingRule {
    fn is_applicable(&self, _children: Vec<Child>) -> bool {
        true
    }
    fn format(&self, node: Pair<Rule>, children: Vec<Child>) -> String;
}

#[derive(Clone)]
struct PaxComponentDefinitionDefaultRule;

impl FormattingRule for PaxComponentDefinitionDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let tags = children
            .iter()
            .filter(|child| child.node_type == Rule::root_tag_pair);
        let settings = children
            .iter()
            .filter(|child| child.node_type == Rule::settings_block_declaration);

        let mut component = vec![];

        let mut formatted_tags = String::new();
        for (i, tag) in tags.enumerate() {
            if i > 0 {
                formatted_tags.push_str("\n");
            }
            formatted_tags.push_str(&tag.formatted_node);
        }

        if formatted_tags.len() > 0 {
            component.push(formatted_tags)
        }

        let mut formatted_settings = String::new();
        for (i, setting) in settings.enumerate() {
            if i > 0 {
                formatted_settings.push_str("\n");
            }
            formatted_settings.push_str(&setting.formatted_node);
        }

        if formatted_settings.len() > 0 {
            component.push(formatted_settings)
        }

        let formatted_component = component.join("\n\n");
        formatted_component
    }
}

#[derive(Clone)]
struct OpenTagDefaultRule;

impl FormattingRule for OpenTagDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        formatted_node.push_str("<");
        formatted_node.push_str(&children[0].formatted_node);
        formatted_node.push_str(" ");
        formatted_node
            .push_str(greedy_append_with_line_limit(children[1..].to_vec(), " ").as_str());
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
        formatted_node.push_str(" ");
        formatted_node
            .push_str(greedy_append_with_line_limit(children[1..].to_vec(), " ").as_str());
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
        let children = children
            .iter()
            .map(|child| child.formatted_node.clone())
            .collect::<Vec<String>>()
            .join("\n");
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
        formatted_node.push_str(&format!("{}={}", key, value).as_str());
        formatted_node
    }
}

#[derive(Clone)]
struct SettingsBlockDeclarationDefaultRule;

impl FormattingRule for SettingsBlockDeclarationDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();

        enum SettingType {
            Selector,
            Event,
            Unknown,
        }

        let mut current = SettingType::Unknown;
        let mut unknown_comments: VecDeque<Child> = VecDeque::new();
        let mut handlers: VecDeque<Child> = VecDeque::new();
        let mut selectors: VecDeque<Child> = VecDeque::new();

        for child in children.iter().rev() {
            if child.node_type == Rule::selector_block {
                current = SettingType::Selector;
                selectors.push_front(child.clone());
            } else if child.node_type == Rule::settings_event_binding {
                current = SettingType::Event;
                handlers.push_front(child.clone());
            } else if child.node_type == Rule::comment {
                match current {
                    SettingType::Selector => selectors.push_front(child.clone()),
                    SettingType::Event => handlers.push_front(child.clone()),
                    SettingType::Unknown => unknown_comments.push_front(child.clone()),
                }
            }
        }

        if selectors.is_empty() {
            handlers.extend(unknown_comments);
        } else {
            selectors.extend(unknown_comments);
        }

        let mut settings: Vec<String> = Vec::new();

        if !handlers.is_empty() {
            settings.push(
                handlers
                    .iter()
                    .map(|child| child.formatted_node.clone())
                    .collect::<Vec<String>>()
                    .join("\n"),
            );
        }

        if !selectors.is_empty() {
            settings.push(
                selectors
                    .iter()
                    .map(|child| child.formatted_node.clone())
                    .collect::<Vec<String>>()
                    .join("\n"),
            );
        }

        let settings = settings.join("\n");
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
    fn format(&self, node: Pair<Rule>, _children: Vec<Child>) -> String {
        let trim_node: String = node
            .as_str()
            .chars()
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
        let setting = children
            .iter()
            .map(|child| child.formatted_node.clone())
            .collect::<Vec<String>>()
            .join(" ");
        formatted_node.push_str(setting.as_str());
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
struct EventIdDefaultRule;

impl FormattingRule for crate::formatting::rules::EventIdDefaultRule {
    fn format(&self, node: Pair<Rule>, _children: Vec<Child>) -> String {
        "@".to_string() + node.as_str().trim().trim_start_matches("@").trim()
    }
}

#[derive(Clone)]
struct SettingsEventBindingDefaultRule;

impl FormattingRule for crate::formatting::rules::SettingsEventBindingDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        children.get(0).unwrap().formatted_node.clone()
            + ": "
            + &children.get(1).unwrap().formatted_node
            + ","
    }
}

#[derive(Clone)]
struct ListMultiLineRule;

impl FormattingRule for ListMultiLineRule {
    fn is_applicable(&self, children: Vec<Child>) -> bool {
        has_multi_line_children(&children) || children_longer_than_line_limit(&children)
    }

    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        let functions = children
            .iter()
            .map(|child| child.formatted_node.clone())
            .collect::<Vec<String>>()
            .join(",\n");
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
        formatted_node.push_str(greedy_append_with_line_limit(children, ", ").as_str());
        formatted_node.push_str("]");
        formatted_node
    }
}

#[derive(Clone)]
struct TupleMultiLineRule;

impl FormattingRule for TupleMultiLineRule {
    fn is_applicable(&self, children: Vec<Child>) -> bool {
        has_multi_line_children(&children) || children_longer_than_line_limit(&children)
    }

    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        let elements = children
            .iter()
            .map(|child| child.formatted_node.clone())
            .collect::<Vec<String>>()
            .join(",\n");
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
        formatted_node.push_str(greedy_append_with_line_limit(children, ", ").as_str());
        formatted_node.push_str(")");
        formatted_node
    }
}

#[derive(Clone)]
struct IdentifierCallMultiLineRule;

impl FormattingRule for IdentifierCallMultiLineRule {
    fn is_applicable(&self, children: Vec<Child>) -> bool {
        has_multi_line_children(&children) || children_longer_than_line_limit(&children)
    }

    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        for child in children {
            if child.node_type == Rule::pascal_identifier || child.node_type == Rule::identifier {
                if formatted_node.len() > 0 {
                    formatted_node.push_str("::");
                }
                formatted_node.push_str(&child.formatted_node);
            } else if child.node_type == Rule::literal_enum_args_list
                || child.node_type == Rule::xo_enum_or_function_args_list
            {
                formatted_node.push_str("(\n");
                let indented_child = indent_every_line_of_string(child.formatted_node.clone());
                formatted_node.push_str(&indented_child);
                formatted_node.push_str(")");
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
            } else if child.node_type == Rule::literal_enum_args_list
                || child.node_type == Rule::xo_enum_or_function_args_list
            {
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
    fn is_applicable(&self, children: Vec<Child>) -> bool {
        has_multi_line_children(&children) || children_longer_than_line_limit(&children)
    }

    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        let elements = children
            .iter()
            .map(|child| child.formatted_node.clone())
            .collect::<Vec<String>>()
            .join(",\n");
        formatted_node.push_str(&elements);
        formatted_node
    }
}

#[derive(Clone)]
struct ArgsListDefaultRule;

impl FormattingRule for ArgsListDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        formatted_node.push_str(greedy_append_with_line_limit(children, ", ").as_str());
        formatted_node
    }
}

#[derive(Clone)]
struct ExpressionBodyMultiLineRule;

impl FormattingRule for ExpressionBodyMultiLineRule {
    fn is_applicable(&self, children: Vec<Child>) -> bool {
        has_multi_line_children(&children) || children_longer_than_line_limit(&children)
    }

    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        let mut current_line = String::new();
        let mut first_line = true;
        for child in children {
            if is_prefix(&child) {
                current_line.push_str(&child.formatted_node);
            } else if is_infix(&child) {
                formatted_node.push_str("\n");
                current_line = String::new();
                current_line.push_str(&child.formatted_node);
                current_line.push_str(" ");
            } else if is_primary_operand(&child) {
                current_line.push_str(&child.formatted_node);
                if first_line {
                    formatted_node.push_str(&current_line);
                    first_line = false;
                } else {
                    let indented_current_line = indent_every_line_of_string(current_line);
                    formatted_node.push_str(&indented_current_line);
                }
                current_line = String::new();
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
        let literal_number_unit = if let Some(unit) = children
            .iter()
            .find(|child| child.node_type == Rule::literal_number_unit)
        {
            unit.formatted_node.clone()
        } else {
            String::new()
        };

        formatted_node
            .push_str(format!("({}){}", children[0].formatted_node, literal_number_unit).as_str());
        formatted_node
    }
}

#[derive(Clone)]
struct ObjectDefaultRule;

impl FormattingRule for ObjectDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        let pascal_identifier = if let Some(identifier) = children.iter().find(|child| {
            child.node_type == Rule::pascal_identifier || child.node_type == Rule::identifier
        }) {
            identifier.formatted_node.clone() + " "
        } else {
            String::new()
        };

        let settings_pairs = children
            .iter()
            .filter(|child| {
                child.node_type == Rule::xo_object_settings_key_value_pair
                    || child.node_type == Rule::settings_key_value_pair
                    || child.node_type == Rule::comment
            })
            .map(|child| child.formatted_node.clone())
            .collect::<Vec<String>>()
            .join("\n");
        let indented_settings_pairs = indent_every_line_of_string(settings_pairs);
        formatted_node
            .push_str(format!("{}{{\n{}\n}}", pascal_identifier, indented_settings_pairs).as_str());
        formatted_node
    }
}

#[derive(Clone)]
struct XoObjectSettingsKeyValuePairDefaultRule;

impl FormattingRule for XoObjectSettingsKeyValuePairDefaultRule {
    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        let mut formatted_node = String::new();
        let setting = children
            .iter()
            .map(|child| child.formatted_node.clone())
            .collect::<Vec<String>>()
            .join(" ");
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
        formatted_node
            .push_str(format!("for {} in {} {{\n{}\n}}", sfpd, sfs, inner_nodes_indented).as_str());
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

    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        return children
            .iter()
            .map(|child| child.formatted_node.clone())
            .collect::<Vec<String>>()
            .join("");
    }
}

#[derive(Clone)]
struct PrintRule;

impl FormattingRule for PrintRule {
    fn format(&self, node: Pair<Rule>, _children: Vec<Child>) -> String {
        return node.as_str().trim().to_string();
    }
}

#[derive(Clone)]
struct IgnoreRule;

impl FormattingRule for IgnoreRule {
    fn format(&self, _node: Pair<Rule>, _children: Vec<Child>) -> String {
        String::new()
    }
}

#[derive(Clone)]
struct PanicRule;

impl FormattingRule for PanicRule {
    fn format(&self, node: Pair<Rule>, _children: Vec<Child>) -> String {
        let (l, c) = node.as_span().start_pos().line_col();
        panic!(
            "Cannot format pax. {:?} issue at line: {} column: {}",
            node.as_rule(),
            l,
            c
        );
    }
}

#[derive(Clone)]
struct WrapExpressionRule;

impl FormattingRule for WrapExpressionRule {
    fn is_applicable(&self, children: Vec<Child>) -> bool {
        if children.len() == 1 {
            if children[0].node_type == Rule::expression_body {
                return true;
            }
        }
        return false;
    }

    fn format(&self, _node: Pair<Rule>, children: Vec<Child>) -> String {
        format!("{{{}}}", children[0].formatted_node)
    }
}

#[derive(Clone)]
struct DoNotIndentRule;

impl FormattingRule for DoNotIndentRule {
    fn format(&self, node: Pair<Rule>, _children: Vec<Child>) -> String {
        let value: String = node.as_str().trim().to_string();
        let mut formatted_value = Vec::new();
        for (i, line) in value.lines().enumerate() {
            let mut line_str = line.to_string();
            if i > 0 {
                line_str.insert_str(0, DO_NOT_INSERT_TAB_MARKER);
            }
            formatted_value.push(line_str);
        }
        formatted_value.join("\n")
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
    let mut current_formatted_line = String::new();
    let mut first_line = true;
    for (i, child) in children.iter().enumerate() {
        let child_lines: Vec<&str> = child.formatted_node.split('\n').collect();
        let first_line_of_child = child_lines[0].len();
        if i > 0 {
            current_formatted_line.push_str(seperator);
        }

        if current_formatted_line.len() + first_line_of_child > LINE_LIMIT {
            if current_formatted_line != "" {
                if !first_line {
                    current_formatted_line =
                        indent_every_line_of_string(current_formatted_line.clone());
                }
                formatted_node.push_str(&current_formatted_line);
                formatted_node.push_str("\n");
                first_line = false;
                current_formatted_line = String::new();
            }
            current_formatted_line.push_str(&child.formatted_node.clone());
        } else {
            current_formatted_line.push_str(&child.formatted_node);
        }
    }
    if !first_line {
        current_formatted_line = indent_every_line_of_string(current_formatted_line.clone());
    }
    formatted_node.push_str(&current_formatted_line);
    formatted_node
}

fn n_indentation_string(n: usize) -> String {
    " ".repeat(INDENTATION).repeat(n)
}

pub fn indent_every_line_of_string(string: String) -> String {
    let mut result = String::new();
    for line in string.lines() {
        if !line.contains(DO_NOT_INSERT_TAB_MARKER) {
            result.push_str(&n_indentation_string(1));
        }
        result.push_str(line);
        result.push_str("\n");
    }
    result.trim_end_matches("\n").to_string()
}
