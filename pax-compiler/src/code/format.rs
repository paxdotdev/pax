use core::panic;

use color_eyre::eyre;
use pest::{iterators::Pair, Parser};

use crate::parsing::{PaxParser, Rule};

use super::rules;

// const FOR_TEMPLATE: &str = "for {} in {} {\n{}\n}";
// const IF_TEMPLATE: &str = "for {} in {} {\n{}\n}";
// const SLOT_TEMPLATE: &str = "slot {}";
// const SETTINGS_TEMPLATE: &str = "@settings {\n{}\n}";
// const HANDLERS_TEMPLATE: &str = "@handlers {\n{}\n}";


// struct Formatter {
//     soft_line_limit: usize,
// }

// impl Formatter {
//     fn new() -> Self {
//         Formatter {
//             soft_line_limit: 120,
//         }
//     }

//     fn indent_every_line_of_string(&self, string: String) -> String {
//         let mut result = String::new();
//         for line in string.lines() {
//             result.push_str(&self.n_indentation_string(1));
//             result.push_str(line);
//             result.push_str("\n");
//         }
//         result
//     }

//     fn construct_high_level_node(&mut self, current_line: String, children: Vec<String>, spaces: bool) -> String {
//         let mut formatted_result = String::new();
//         let mut current_line_length = current_line.len();
//         let mut start_on_new_Line = false;

//         formatted_result.push_str(&current_line);
//         for child in children {
//             let child_lines: Vec<&str> = child.split('\n').collect();
//             let first_line_length = child_lines[0].len();
//             let mut started_new_line = false;

//             // If the first line of the child can fit in the current line within the soft limit, add it
//             if !start_on_new_Line && current_line_length + first_line_length + 1 <= self.soft_line_limit {
//                 if current_line_length > 0 && spaces {
//                     formatted_result.push(' ');
//                 }
//                 formatted_result.push_str(child_lines[0]);
//                 current_line_length += first_line_length + 1; // +1 for the space
//                 start_on_new_Line = false;
//             } else {
//                 // Otherwise, start a new line and reset the line length count
//                 formatted_result.push('\n');
//                 formatted_result.push_str(&self.n_indentation_string(1));
//                 formatted_result.push_str(child_lines[0]);
//                 current_line_length = first_line_length;
//                 started_new_line = true;
//             }

//             if child_lines.len() > 1 {
//                 for &line in &child_lines[1..] {
//                     formatted_result.push('\n');
//                     if started_new_line {
//                         formatted_result.push_str(&self.n_indentation_string(1));
//                     }
//                     formatted_result.push_str(line);
//                 }
//                 start_on_new_Line = true;
//             }
//         }
//         formatted_result
//     }

//     fn format_node(&mut self, node: Pair<Rule>) -> String {
//         let mut result = String::new();
//         match node.as_rule() {
//             Rule::pax_component_definition => {
//                 let mut tags: Vec<String>= Vec::new();
//                 let mut settings: Vec<String> = Vec::new();
//                 let mut handlers: Vec<String> = Vec::new();
//                 let mut current_section = 1;
//                 for child in node.into_inner() {
//                     match child.as_rule() {
//                         Rule::root_tag_pair => {
//                             current_section = 1;
//                             tags.push(self.format_node(child));
//                         },
//                         Rule::settings_block_declaration => {
//                             current_section = 2;
//                             settings.push(self.format_node(child));

//                         },
//                         Rule::handlers_block_declaration => {
//                             current_section = 3;
//                             handlers.push(self.format_node(child));
//                         },
//                         Rule::block_level_error => {
//                             match current_section {
//                                 1 => tags.push(self.format_node(child)),
//                                 2 => settings.push(self.format_node(child)),
//                                 3 => handlers.push(self.format_node(child)),
//                                 _ => {}
//                             }
//                         },
//                         _ => {},
//                     }
//                 }

//                 if tags.len() > 0 {
//                     result.push_str(&tags.join("\n"));
//                 }
//                 if settings.len() > 0 {
//                     if tags.len() > 0 {
//                         result.push_str("\n");
//                         result.push_str("\n");
//                         result.push_str("helo");
//                     }
//                     result.push_str(&settings.join("\n"));
//                 }
//                 if handlers.len() > 0 {
//                     if tags.len() > 0 || settings.len() > 0 {
//                         result.push_str("\n");
//                         result.push_str("\n");
//                     }
//                     result.push_str(&handlers.join("\n"));
//                 }
//             }
//             Rule::root_tag_pair => {
//                 let child = node.into_inner().next().unwrap();
//                 result.push_str(&self.format_node(child));
//             }
//             Rule::EOI => {}
//             Rule::WHITESPACE => {}
//             Rule::empty => {}
//             Rule::any_tag_pair => {}
//             Rule::COMMENT => result.push_str(node.as_str()),
//             Rule::string => result.push_str(node.as_str()),
//             Rule::inner => result.push_str(node.as_str()),
//             Rule::char => result.push_str(node.as_str()),
//             Rule::selector => result.push_str(node.as_str().trim()),
//             Rule::identifier => result.push_str(node.as_str().trim()),
//             Rule::pascal_identifier => result.push_str(node.as_str().trim()),
//             Rule::attribute_event_id => result.push_str(node.as_str().trim()),
//             Rule::literal_number_with_unit => result.push_str(node.as_str().trim()),
//             Rule::literal_number => result.push_str(node.as_str().trim()),
//             Rule::literal_number_integer => result.push_str(node.as_str().trim()),
//             Rule::literal_number_float => result.push_str(node.as_str()),
//             Rule::literal_number_unit => result.push_str(node.as_str()),
//             Rule::literal_boolean => result.push_str(node.as_str()),
//             Rule::handler_key_value_pair_error => result.push_str(node.as_str()),
//             Rule::literal_tuple_access => result.push_str(node.as_str()),
//             Rule::expression_body_error => result.push_str(node.as_str()),
//             Rule::xo_prefix => result.push_str(node.as_str()),
//             Rule::xo_neg => result.push_str(node.as_str()),
//             Rule::xo_bool_not => result.push_str(node.as_str()),
//             Rule::xo_infix => result.push_str(node.as_str()),
//             Rule::xo_add => result.push_str(node.as_str()),
//             Rule::xo_bool_and => result.push_str(node.as_str()),
//             Rule::xo_bool_or => result.push_str(node.as_str()),
//             Rule::xo_div => result.push_str(node.as_str()),
//             Rule::xo_exp => result.push_str(node.as_str()),
//             Rule::xo_mod => result.push_str(node.as_str()),
//             Rule::xo_mul => result.push_str(node.as_str()),
//             Rule::xo_rel_eq => result.push_str(node.as_str()),
//             Rule::xo_rel_gt => result.push_str(node.as_str()),
//             Rule::xo_rel_gte => result.push_str(node.as_str()),
//             Rule::xo_rel_lt => result.push_str(node.as_str()),
//             Rule::xo_rel_lte => result.push_str(node.as_str()),
//             Rule::xo_rel_neq => result.push_str(node.as_str()),
//             Rule::xo_sub => result.push_str(node.as_str()),
//             Rule::xo_tern_then => result.push_str(node.as_str()),
//             Rule::xo_tern_else => result.push_str(node.as_str()),
//             Rule::xo_range => result.push_str(node.as_str()),
//             Rule::xo_range_exclusive => result.push_str(node.as_str()),
//             Rule::xo_symbol => result.push_str(node.as_str().trim()),
//             Rule::statement_for_predicate_declaration => result.push_str(node.as_str().trim()),
//             Rule::statement_for_source => result.push_str(node.as_str().trim()),
//             Rule::block_level_error => {
//                 result.push_str(node.as_str());
//                 result.push_str("\n");
//             }
//             Rule::tag_error => {
//                 result.push_str(node.as_str());
//                 result.push_str("\n");
//             }
//             Rule::open_tag_error => {
//                 result.push_str(node.as_str());
//                 result.push_str("\n");
//             }
//             Rule::inner_tag_error => {
//                 result.push_str(node.as_str());
//                 result.push_str("\n");
//             }
//             Rule::attribute_key_value_pair_error => {
//                 result.push_str(node.as_str());
//             }
//             Rule::selector_block_error => {
//                 result.push_str(node.as_str());
//             }
//             Rule::open_tag => {
//                 let mut first_line = String::new();
//                 first_line.push_str("<");
//                 let mut children = node.into_inner();
//                 let pascal_identifier = children.next().unwrap();
//                 first_line.push_str(pascal_identifier.as_str());
//                 let mut child_results: Vec<String> = Vec::new();
//                 for child in children {
//                     child_results.push(self.format_node(child));
//                 }
//                 result.push_str(&self.construct_high_level_node(first_line, child_results, true));
//                 result.push_str(">");
//             }
//             Rule::closing_tag => {
//                 let trim_node: String = node.as_str().chars()
//                                          .filter(|c| !c.is_whitespace())
//                                          .collect();
//                 result.push_str(&trim_node);
//             }
//             Rule::self_closing_tag => {
//                 let mut first_line = String::new();
//                 first_line.push_str("<");
//                 let mut children = node.into_inner();
//                 let pascal_identifier = children.next().unwrap();
//                 first_line.push_str(pascal_identifier.as_str());
//                 let mut child_results: Vec<String> = Vec::new();
//                 for child in children {
//                     child_results.push(self.format_node(child));
//                 }
//                 result.push_str(&self.construct_high_level_node(first_line, child_results, true));
//                 result.push_str("/>");
//             }
//             Rule::matched_tag => {
//                 let mut children = node.into_inner();
//                 let open_tag = children.next().unwrap();
//                 let formatted_open_tag = self.format_node(open_tag);
//                 result.push_str(&formatted_open_tag);
//                 result.push_str("\n");
//                 let inner_nodes = children.next().unwrap();
//                 let formatted_inner_nodes = self.format_node(inner_nodes);
//                 result.push_str(&formatted_inner_nodes);
//                 let closing_tag = children.next().unwrap();
//                 let formatted_closing_tag = self.format_node(closing_tag);
//                 result.push_str(&formatted_closing_tag);
//             }
//             Rule::inner_nodes => {
//                 let x: Rule= node.as_rule();
//                 let child = node.clone().into_inner().peek().unwrap();
//                 match child.as_rule() {
//                     Rule::node_inner_content => {
//                         result.push_str(&self.format_node(child.into_inner().next().unwrap()));
//                     }
//                     _ => {
//                         for pair in node.into_inner() {
//                             result.push_str(&self.format_node(pair));
//                             result.push_str("\n");
//                         }
//                         result = self.indent_every_line_of_string(result);
//                     }
//                 }
//             }
//             Rule::attribute_key_value_pair => {
//                 let child = node.clone().into_inner().peek().unwrap();
//                 match child.as_rule() {
//                     Rule::attribute_event_binding => {
//                         let attribute_event_binding = node.into_inner().next().unwrap();
//                         result.push_str(&self.format_node(attribute_event_binding))
//                     }
//                     _ => {
//                         let mut setting_binding = node.into_inner();
//                         let key = setting_binding.next().unwrap();
//                         let key_formatted = self.format_node(key);
//                         let value = setting_binding.next().unwrap();
//                         let value_formatted = self.format_node(value);
//                         let eq = "=".to_string();
//                         let binding = vec![key_formatted, eq, value_formatted];
//                         let node = self.construct_high_level_node(String::new(), binding, false);
//                         result.push_str(&node);
//                     }
//                 }
//             }
//             Rule::attribute_event_binding => {
//                 let mut children = node.into_inner();
//                 let event_id = children.next().unwrap();
//                 let event_id_formatted = self.format_node(event_id);
//                 let eq = "=".to_string();
//                 let value = children.next().unwrap();
//                 let value_formatted = self.format_node(value);
//                 let binding = vec![event_id_formatted, eq, value_formatted];
//                 let node = self.construct_high_level_node(String::new(), binding, false);
//                 result.push_str(&node);
//             }
//             Rule::any_template_value => {
//                 let child = node.into_inner().next().unwrap();
//                 result.push_str(&self.format_node(child));
//             }
//             Rule::node_inner_content => {
//                 let child = node.into_inner().next().unwrap();
//                 result.push_str(&self.format_node(child));
//             }
//             Rule::settings_block_declaration => {
//                 let children = node.into_inner();
//                 result.push_str("@settings {\n");
//                 for child in children {
//                     let formatted_child = self.format_node(child);
//                     let indented_child = self.indent_every_line_of_string(formatted_child);
//                     result.push_str(&indented_child);
//                     result.push_str("\n");
//                 }
//                 result = result.trim_end_matches('\n').to_string();
//                 result.push_str("\n");
//                 result.push_str("}");
//             }
//             Rule::selector_block => {
//                 let mut children = node.into_inner();
//                 let raw_selector = children.next().unwrap();
//                 let raw_selector_formatted = self.format_node(raw_selector);
//                 result.push_str(&raw_selector_formatted);
//                 result.push_str(" ");
//                 let literal_object = children.next().unwrap();
//                 let literal_object_formatted = self.format_node(literal_object);
//                 result.push_str(&literal_object_formatted);
//             }
//             Rule::literal_object => {
//                 let mut children = node.into_inner();
//                 match children.peek().unwrap().as_rule() {
//                     Rule::pascal_identifier => {
//                         let formatted_pascal_identifier =
//                             self.format_node(children.next().unwrap());
//                         result.push_str(&formatted_pascal_identifier);
//                         result.push_str(" ");
//                     }
//                     _ => {}
//                 };
//                 result.push_str("{");
//                 let settings_value_pairs = children.next().unwrap();

//                 for child in settings_value_pairs.into_inner() {
//                     result.push_str("\n");
//                     let formatted_child = self.format_node(child);
//                     let indented_child = self.indent_every_line_of_string(formatted_child);
//                     result.push_str(&indented_child);
//                     result.push_str(",");
//                 }

//                 result.push_str("\n");
//                 result.push_str("}");
//             }
//             Rule::settings_key_value_pair => {
//                 let mut children = node.into_inner();
//                 let key = children.next().unwrap();
//                 let key_formatted = self.format_node(key);
//                 let value = children.next().unwrap();
//                 let value_formatted = self.format_node(value);
//                 let binding = vec![key_formatted, value_formatted];
//                 let node: String = self.construct_high_level_node(String::new(), binding, true);
//                 result.push_str(&node);
//             }
//             Rule::settings_key => {
//                 result.push_str(node.as_str());
//                 result.push_str(" ");
//             }
//             Rule::settings_value => {
//                 let child = node.into_inner().next().unwrap();
//                 result.push_str(&self.format_node(child));
//             }
//             Rule::handlers_block_declaration => {
//                 let children = node.into_inner();
//                 result.push_str("@handlers {\n");
//                 for child in children {
//                     let formatted_child = self.format_node(child);
//                     let indented_child = self.indent_every_line_of_string(formatted_child);
//                     result.push_str(&indented_child);
//                     result.push_str("\n");
//                 }
//                 result = result.trim_end_matches('\n').to_string();
//                 result.push_str("\n");
//                 result.push_str("}");
//             }
//             Rule::handlers_key_value_pair => {
//                 let mut children = node.into_inner();
//                 let key = children.next().unwrap();
//                 let key_formatted = self.format_node(key);
//                 let value = children.next().unwrap();
//                 let value_formatted = self.format_node(value);
//                 let pair = vec![key_formatted, value_formatted];
//                 let node = self.construct_high_level_node(String::new(), pair, true);
//                 result.push_str(&node);
//                 result.push_str(",");
//             }
//             Rule::handlers_key => result.push_str(node.as_str()),
//             Rule::handlers_value => {
//                 let mut children = node.into_inner();
//                 let child = children.next().unwrap();
//                 let formatted_value = self.format_node(child);
//                 let no_deliminator_value = formatted_value.trim_end_matches(',');
//                 result.push_str(no_deliminator_value);
//             }
//             Rule::literal_function => result.push_str(node.as_str()),
//             Rule::function_list => {
//                 let children = node.into_inner();
//                 let mut list = Vec::new();
//                 for child in children {
//                     list.push(self.format_node(child));
//                 }
//                 let node = self.construct_high_level_node("[".to_string(), list, false);
//                 result.push_str(&node);
//                 result.push_str("]");
//             }
//             Rule::literal_value => {
//                 let mut children = node.into_inner();
//                 let child = children.next().unwrap();
//                 result.push_str(&self.format_node(child));
//             }
//             Rule::literal_tuple => {
//                 let mut children = node.into_inner();
//                 let first_child = children.next().unwrap();
//                 let mut first_child_formatted = self.format_node(first_child);
//                 first_child_formatted.push(',');
//                 let mut children_to_format: Vec<String> = Vec::new();
//                 children_to_format.push(first_child_formatted);
//                 for child in children {
//                     let val = child.into_inner().next().unwrap();
//                     let val_formatted = self.format_node(val) + ",";
//                     children_to_format.push(val_formatted);
//                 }
//                 let last_item = children_to_format.last_mut().unwrap();
//                 *last_item = last_item.trim_end_matches(',').to_string();
//                 let node = self.construct_high_level_node("(".to_string(), children_to_format, true);
//                 result.push_str(&node);
//                 result.push_str(")");
//             }
//             Rule::literal_enum_value => {
//                 let mut first_line = String::new();
//                 let mut children = node.into_inner();
//                 let pascal_identifier = children.next().unwrap();
//                 let pascal_identifier_formatted = self.format_node(pascal_identifier);
//                 first_line.push_str(&pascal_identifier_formatted);
//                 let mut children_to_format: Vec<String> = Vec::new();
//                 for child in children {
//                     match child.as_rule() {
//                         Rule::pascal_identifier => {
//                             children_to_format.push(child.as_str().to_string());
//                         }
//                         Rule::literal_enum_args_list => {
//                             let list_formatted = self.format_node(child) + ")";
//                             children_to_format.push("(".to_string());
//                             children_to_format.push(list_formatted);
//                         }
//                         _ => {}
//                     }
//                 }
//                 let node = self.construct_high_level_node(first_line, children_to_format, false);
//                 result.push_str(&node);
//             }
//             Rule::literal_enum_args_list => {
//                 let mut children = node.into_inner();
//                 let mut children_to_format: Vec<String> = Vec::new();
//                 let first_child = children.next().unwrap();
//                 let first_child_formatted = self.format_node(first_child) + ",";
//                 children_to_format.push(first_child_formatted);
//                 for child in children {
//                     let val = child.into_inner().next().unwrap();
//                     let val_formatted = self.format_node(val) + ",";
//                     children_to_format.push(val_formatted);
//                 }
//                 let last_item = children_to_format.last_mut().unwrap();
//                 *last_item = last_item.trim_end_matches(',').to_string();
//                 let node = self.construct_high_level_node(String::new(), children_to_format, false);
//                 result.push_str(&node);
//             }
//             Rule::expression_body => {
//                 let mut children = node.clone().into_inner();
//                 let mut prefix = String::new();
//                 panic!("{:?}", node);
//                 while children.peek().unwrap().as_rule() == Rule::xo_neg 
//                     || children.peek().unwrap().as_rule() == Rule::xo_bool_not {
//                     let child = children.next().unwrap();
//                     prefix.push_str(child.as_str());
//                 }
//                 let first_primary = children.next().unwrap();
//                 let first_primary_formatted = self.format_node(first_primary);
//                 let full_first = prefix + &first_primary_formatted;
//                 let mut children_to_format: Vec<String> = Vec::new();
//                 children_to_format.push(full_first);
//                 let mut current_operator_operand = String::new();
//                 while children.peek().is_some(){
//                     match children.peek().unwrap().as_rule() {
//                         Rule::xo_add |
//                         Rule::xo_bool_and |
//                         Rule::xo_bool_or |
//                         Rule::xo_div |
//                         Rule::xo_exp |
//                         Rule::xo_mod |
//                         Rule::xo_mul |
//                         Rule::xo_rel_eq |
//                         Rule::xo_rel_gt |
//                         Rule::xo_rel_gte |
//                         Rule::xo_rel_lt |
//                         Rule::xo_rel_lte |
//                         Rule::xo_rel_neq |
//                         Rule::xo_sub |
//                         Rule::xo_tern_then |
//                         Rule::xo_tern_else => {
//                             let infix = children.next().unwrap();
//                             let infix_formatted = self.format_node(infix);
//                             current_operator_operand.push_str(&infix_formatted.trim());
//                             current_operator_operand.push_str(" ");
//                         }
//                         Rule::xo_neg | Rule::xo_bool_not => {
//                             let pre = children.next().unwrap();;
//                             current_operator_operand.push_str(pre.as_str());
//                         },
//                         Rule::expression_grouped 
//                         | Rule::xo_function_call 
//                         | Rule::xo_object 
//                         | Rule::xo_range 
//                         | Rule::xo_tuple 
//                         | Rule::xo_list 
//                         | Rule::xo_literal  
//                         | Rule::xo_symbol => {
//                             let next_primary = children.next().unwrap();
//                             let next_primary_formatted = self.format_node(next_primary);
//                             current_operator_operand.push_str(&next_primary_formatted);
//                             children_to_format.push(current_operator_operand);
//                             current_operator_operand = String::new();
//                         }
//                         _ => {}
//                     }
//                 }
//                 let node = self.construct_high_level_node(String::new(), children_to_format, true) + "}";
//                 let wrapped_node =  self.construct_high_level_node(String::new(), vec!["{".to_string(), node], false);
//                 result.push_str(&wrapped_node);
//             }
//             Rule::expression_wrapped => {
//                 let exp = node.into_inner().next().unwrap();
//                 let formatted_exp = self.format_node(exp) + "}";
//                 let children_to_format: Vec<String> = vec!["{".to_string(), formatted_exp];
//                 let node = self.construct_high_level_node(String::new(), children_to_format, false);
//                 result.push_str(&node);
//             }
//             Rule::expression_grouped => {
//                 let mut children = node.into_inner();
//                 let exp = children.next().unwrap();
//                 let mut formatted_exp = self.format_node(exp) + ")";
//                 let literal_num_units = children.next();
//                 if let Some(literal_num) = literal_num_units {
//                     formatted_exp.push_str(&self.format_node(literal_num));
//                 }
//                 let children_to_format: Vec<String> = vec!["(".to_string(), formatted_exp];
//                 let node = self.construct_high_level_node(String::new(), children_to_format, false);
//                 result.push_str(&node);
//             }
//             Rule::xo_primary => {
//                 result.push_str(&self.format_node(node.into_inner().next().unwrap()))
//             }
//             Rule::xo_literal => {
//                 result.push_str(&self.format_node(node.into_inner().next().unwrap()))
//             }
//             Rule::xo_object => {
//                 let mut children = node.into_inner();
//                 match children.peek().unwrap().as_rule() {
//                     Rule::identifier => {
//                         let formatted_pascal_identifier =
//                             self.format_node(children.next().unwrap());
//                         result.push_str(&formatted_pascal_identifier);
//                         result.push_str(" ");
//                     }
//                     _ => {}
//                 };
//                 result.push_str("{");


//                 let settings_value_pairs = children.next().unwrap();

//                 for child in settings_value_pairs.into_inner() {
//                     result.push_str("\n");
//                     let formatted_child = self.format_node(child);
//                     let indented_child = self.indent_every_line_of_string(formatted_child);
//                     result.push_str(&indented_child);
//                     result.push_str(",");
//                 }

//                 result.push_str("\n");
//                 result.push_str("}");
//             }
//             Rule::xo_object_settings_key_value_pair => {
//                 let mut children = node.into_inner();
//                 let key: Pair<'_, Rule> = children.next().unwrap();
//                 let key_formatted = self.format_node(key);
//                 let value: Pair<'_, Rule> = children.next().unwrap();
//                 let value_formatted = self.format_node(value);
//                 let binding = vec![key_formatted, value_formatted];
//                 let node: String = self.construct_high_level_node(String::new(), binding, true);
//                 result.push_str(&node);
//             }
//             Rule::xo_tuple => {
//                 let mut children = node.into_inner();
//                 let first_child = children.next().unwrap();
//                 let mut first_child_formatted = self.format_node(first_child);
//                 first_child_formatted.push(',');
//                 let mut children_to_format: Vec<String> = Vec::new();
//                 children_to_format.push(first_child_formatted);
//                 for child in children {
//                     let val = child.into_inner().next().unwrap();
//                     let val_formatted = self.format_node(val) + ",";
//                     children_to_format.push(val_formatted);
//                 }
//                 let last_item = children_to_format.last_mut().unwrap();
//                 *last_item = last_item.trim_end_matches(',').to_string();
//                 let node = self.construct_high_level_node("(".to_string(), children_to_format, true);
//                 result.push_str(&node);
//                 result.push_str(")");
//             }
//             Rule::xo_list => {
//                 let mut children = node.into_inner();
//                 let first_child = children.next().unwrap();
//                 let mut first_child_formatted = self.format_node(first_child);
//                 first_child_formatted.push(',');
//                 let mut children_to_format: Vec<String> = Vec::new();
//                 children_to_format.push(first_child_formatted);
//                 for child in children {
//                     let val = child.into_inner().next().unwrap();
//                     let val_formatted = self.format_node(val) + ",";
//                     children_to_format.push(val_formatted);
//                 }
//                 let last_item = children_to_format.last_mut().unwrap();
//                 *last_item = last_item.trim_end_matches(',').to_string();
//                 let node = self.construct_high_level_node("[".to_string(), children_to_format, true);
//                 result.push_str(&node);
//                 result.push_str("]");
//             }
//             Rule::xo_function_call => {
//                 let mut children = node.into_inner();
//                 let identifier = children.next().unwrap();
//                 let mut identifier_formatted = self.format_node(identifier);
//                 let mut children_to_format: Vec<String> = Vec::new();

//                 while children.peek().is_some() {
//                     match children.peek().unwrap().as_rule() {
//                         Rule::identifier => {
//                             identifier_formatted.push_str("::");
//                             identifier_formatted.push_str(children.next().unwrap().as_str());
//                         }
//                         Rule::xo_function_args_list => {
//                             let args = children.next().unwrap();
//                             let args_formatted = self.format_node(args) + ")";
//                             identifier_formatted.push_str("(");
//                             children_to_format.push(args_formatted);
//                         }
//                         _ => {}
//                     }
//                 }
//                 let node = self.construct_high_level_node(identifier_formatted, children_to_format, false);
//                 result.push_str(&node);
//             }
//             Rule::xo_function_args_list => {
//                 let mut children = node.into_inner();
//                 let mut children_to_format: Vec<String> = Vec::new();
//                 let first_child = children.next().unwrap();
//                 let first_child_formatted = self.format_node(first_child) + ",";
//                 children_to_format.push(first_child_formatted);
//                 for child in children {
//                     let val = child.into_inner().next().unwrap();
//                     let val_formatted = self.format_node(val) + ",";
//                     children_to_format.push(val_formatted);
//                 }
//                 let last_item = children_to_format.last_mut().unwrap();
//                 *last_item = last_item.trim_end_matches(',').to_string();
//                 let node = self.construct_high_level_node(String::new(), children_to_format, true);
//                 result.push_str(&node);
//             }
//             Rule::statement_control_flow => {
//                 let mut children = node.into_inner();
//                 let child = children.next().unwrap();
//                 result.push_str(&self.format_node(child));
//             }
//             Rule::statement_if => {
//                 let mut first_line = String::new();
//                 let mut children = node.into_inner();
//                 let mut children_to_format: Vec<String> = Vec::new();
//                 let exp = children.next().unwrap();
//                 let exp_formatted = self.format_node(exp) + " {";
//                 let inner_nodes = children.next().unwrap();
//                 let inner_nodes_formatted = self.format_node(inner_nodes);
//                 first_line.push_str("if");
//                 children_to_format.push(exp_formatted);
//                 children_to_format.push(inner_nodes_formatted);
//                 result.push_str("}");
//             }
//             Rule::statement_for => {
//                 let mut first_line = String::new();
//                 let mut children = node.into_inner();
//                 let mut children_to_format: Vec<String> = Vec::new();
//                 let statement_for_predicate_declaration = children.next().unwrap();
//                 let sfpd_formatted = self.format_node(statement_for_predicate_declaration);
//                 let statement_for_source = children.next().unwrap();
//                 let sfs_formatted = self.format_node(statement_for_source).trim().to_string() + " {";
//                 first_line.push_str("for");
//                 children_to_format.push(sfpd_formatted);
//                 children_to_format.push("in".to_string());
//                 children_to_format.push(sfs_formatted);
//                 result.push_str(&self.construct_high_level_node(first_line, children_to_format, true));
//                 result.push_str("\n");
//                 let inner_nodes = children.next().unwrap();
//                 let inner_nodes_formatted = self.format_node(inner_nodes);
//                 result.push_str(&inner_nodes_formatted);
//                 result.push_str("}");
//             }
//             Rule::statement_slot => {
//                 let exp = node.into_inner().next().unwrap();
//                 let exp_formatted = self.format_node(exp);
//                 let children_to_format: Vec<String> = vec![exp_formatted];
//                 let node = self.construct_high_level_node("slot".to_string(), children_to_format, false);
//                 result.push_str(&node);
//             }
//         }
//         result
//     }

//     fn n_indentation_string(&self, n: usize) -> String {
//         "    ".repeat(n)
//     }
// }

pub fn pax_format(code: String) -> Result<String, eyre::Report> {
    let pax_component_definition = PaxParser::parse(Rule::pax_component_definition, code.as_str())?
        .next()
        .unwrap();
    let formatted_code = rules::apply_formatting_rules(pax_component_definition);
    Ok(formatted_code)
}
