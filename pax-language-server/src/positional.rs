use std::collections::HashSet;

use lsp_types::Position;
use pax_compiler::parsing::Rule;
use pest::iterators::Pair;

#[derive(Debug, Clone)]
pub struct PositionalNode {
    start: Position,
    end: Position,
    pub node_type: NodeType,
}

#[derive(Debug, Clone)]
pub enum NodeType {
    Identifier(IdentifierData),
    Tag(TagData),
    Settings,
    SelectorBlock,
    Handlers,
    LiteralFunction(FunctionData),
    LiteralEnumValue(EnumValueData),
    AttributeKeyValuePair(AttributeData),
    AttributeKeyValuePairError(),
    XoFunctionCall(FunctionCallData),
}

#[derive(Debug, Clone)]
pub struct IdentifierData {
    pub identifier: String,
    pub is_pascal_identifier: bool,
}

#[derive(Debug, Clone)]
pub struct TagData {
    pub pascal_identifier: String,
}

#[derive(Debug, Clone)]
pub struct FunctionData {
    pub function_name: String,
}

#[derive(Debug, Clone)]
pub struct EnumValueData {
    pub enum_name: String,
    pub property_name: String,
}

#[derive(Debug, Clone)]
pub struct AttributeData {
    pub identifier: String,
}

#[derive(Debug, Clone)]
pub struct FunctionCallData {
    pub struct_name: String,
    pub function_name: String,
}

fn pair_to_positions(pair: &Pair<Rule>) -> (Position, Position) {
    let span = pair.as_span();
    let start = Position {
        line: (span.start_pos().line_col().0 - 1) as u32,
        character: (span.start_pos().line_col().1 - 1) as u32,
    };
    let end = Position {
        line: (span.end_pos().line_col().0 - 1) as u32,
        character: (span.end_pos().line_col().1 - 1) as u32,
    };
    (start, end)
}

pub fn extract_positional_nodes(
    pair: Pair<'_, Rule>,
    nodes: &mut Vec<PositionalNode>,
    ids: &mut HashSet<String>,
    classes: &mut HashSet<String>,
) {
    let (start, end) = pair_to_positions(&pair);
    let rule = pair.as_rule();
    let as_str = pair.as_str();
    let mut inner = pair.into_inner();

    match rule {
        Rule::handlers_block_declaration => {
            nodes.push(PositionalNode {
                start,
                end,
                node_type: NodeType::Handlers,
            });
            return;
        }
        Rule::settings_block_declaration => {
            nodes.push(PositionalNode {
                start,
                end,
                node_type: NodeType::Settings,
            });
        }
        Rule::selector_block => {
            let selector = &inner.clone().next().unwrap().as_str().to_string();
            if selector.starts_with(".") {
                classes.insert(selector.replace(".", ""));
            } else if selector.starts_with("#") {
                ids.insert(selector.replace("#", ""));
            }
            nodes.push(PositionalNode {
                start,
                end,
                node_type: NodeType::SelectorBlock,
            });
            return;
        }
        Rule::open_tag | Rule::open_tag_error | Rule::tag_error | Rule::self_closing_tag => {
            if let Some(inner_pair) = inner.find(|p| p.as_rule() == Rule::pascal_identifier) {
                let identifier = inner_pair.as_str().to_string();
                nodes.push(PositionalNode {
                    start,
                    end,
                    node_type: NodeType::Tag(TagData {
                        pascal_identifier: identifier.clone(),
                    }),
                });
                nodes.push(PositionalNode {
                    start,
                    end,
                    node_type: NodeType::Identifier(IdentifierData {
                        identifier,
                        is_pascal_identifier: true,
                    }),
                });
            }
        }
        Rule::closing_tag => {
            let identifier = as_str
                .to_string()
                .replace("<", "")
                .replace("/", "")
                .replace(">", "");
            nodes.push(PositionalNode {
                start,
                end,
                node_type: NodeType::Tag(TagData {
                    pascal_identifier: identifier.clone(),
                }),
            });
            nodes.push(PositionalNode {
                start,
                end,
                node_type: NodeType::Identifier(IdentifierData {
                    identifier,
                    is_pascal_identifier: true,
                }),
            });
        }
        Rule::pascal_identifier => {
            let identifier = as_str.to_string();
            nodes.push(PositionalNode {
                start,
                end,
                node_type: NodeType::Identifier(IdentifierData {
                    identifier,
                    is_pascal_identifier: true,
                }),
            });
        }
        Rule::identifier => {
            let identifier = as_str.to_string();
            nodes.push(PositionalNode {
                start,
                end,
                node_type: NodeType::Identifier(IdentifierData {
                    identifier,
                    is_pascal_identifier: false,
                }),
            });
        }
        Rule::literal_function => {
            let function_name = as_str.to_string().replace("self.", "").replace(",", "");
            nodes.push(PositionalNode {
                start,
                end,
                node_type: NodeType::LiteralFunction(FunctionData { function_name }),
            });
        }
        Rule::literal_enum_value => {
            let inner_pairs = &inner;
            let (enum_name, property_name): (String, String);
            if inner_pairs.len() < 3 {
                enum_name = inner_pairs
                    .clone()
                    .nth_back(1)
                    .unwrap()
                    .as_str()
                    .to_string()
                    .replace("::", "");
                property_name = inner_pairs
                    .clone()
                    .nth_back(0)
                    .unwrap()
                    .as_str()
                    .to_string()
                    .replace("::", "");
            } else {
                enum_name = inner_pairs
                    .clone()
                    .nth_back(2)
                    .unwrap()
                    .as_str()
                    .to_string()
                    .replace("::", "");
                property_name = inner_pairs
                    .clone()
                    .nth_back(1)
                    .unwrap()
                    .as_str()
                    .to_string()
                    .replace("::", "");
            }
            nodes.push(PositionalNode {
                start,
                end,
                node_type: NodeType::LiteralEnumValue(EnumValueData {
                    enum_name,
                    property_name,
                }),
            });
        }
        Rule::attribute_key_value_pair => {
            let pair_as_string = as_str.to_string();
            let kv_pair = pair_as_string.split_once("=");
            if let Some((key, value)) = kv_pair {
                if key == "id" {
                    ids.insert(value.to_string());
                } else if key == "class" {
                    classes.insert(value.to_string());
                }
                nodes.push(PositionalNode {
                    start,
                    end,
                    node_type: NodeType::AttributeKeyValuePair(AttributeData {
                        identifier: key.to_string(),
                    }),
                });
            }
        }
        Rule::attribute_key_value_pair_error => {
            nodes.push(PositionalNode {
                start,
                end,
                node_type: NodeType::AttributeKeyValuePairError(),
            });
        }
        Rule::xo_function_call => {
            let inner_pairs = &inner;
            let mut struct_name = "Self".to_string();
            let secondary_name;
            if inner_pairs.len() < 3 {
                secondary_name = inner_pairs
                    .clone()
                    .nth_back(1)
                    .unwrap()
                    .as_str()
                    .to_string()
                    .replace("::", "");
            } else {
                struct_name = inner_pairs
                    .clone()
                    .nth_back(2)
                    .unwrap()
                    .as_str()
                    .to_string()
                    .replace("::", "");
                secondary_name = inner_pairs
                    .clone()
                    .nth_back(1)
                    .unwrap()
                    .as_str()
                    .to_string()
                    .replace("::", "");
            }
            nodes.push(PositionalNode {
                start,
                end,
                node_type: NodeType::XoFunctionCall(FunctionCallData {
                    struct_name,
                    function_name: secondary_name,
                }),
            });
        }
        _ => {}
    }

    for inner_pair in inner {
        extract_positional_nodes(inner_pair, nodes, ids, classes);
    }
}

pub fn find_nodes_at_position(pos: Position, nodes: &Vec<PositionalNode>) -> Vec<PositionalNode> {
    nodes
        .iter()
        .filter(|&node| is_position_within_node(&pos, node))
        .cloned()
        .collect()
}

fn is_position_within_node(pos: &Position, node: &PositionalNode) -> bool {
    // Check if the given position lies within the start and end of the node
    (node.start.line < pos.line
        || (node.start.line == pos.line && node.start.character <= pos.character))
        && (node.end.line > pos.line
            || (node.end.line == pos.line && node.end.character >= pos.character))
}

pub fn find_priority_node(nodes: &Vec<PositionalNode>) -> Option<&PositionalNode> {
    let mut found_literal_function: Option<&PositionalNode> = None;
    let mut found_xo_function_call: Option<&PositionalNode> = None;
    let mut found_literal_enum_value: Option<&PositionalNode> = None;
    let mut found_attribute_key_value_pair: Option<&PositionalNode> = None;
    let mut found_identifier: Option<&PositionalNode> = None;

    for node in nodes.iter() {
        match &node.node_type {
            NodeType::LiteralFunction(_) => {
                found_literal_function = Some(node);
            }
            NodeType::LiteralEnumValue(_) => {
                found_literal_enum_value = Some(node);
            }
            NodeType::XoFunctionCall(_) => {
                found_xo_function_call = Some(node);
            }
            NodeType::AttributeKeyValuePair(_) => {
                found_attribute_key_value_pair = Some(node);
            }
            NodeType::Identifier(_) => {
                found_identifier = Some(node);
            }
            _ => {}
        }
    }

    found_literal_function
        .or(found_xo_function_call)
        .or(found_literal_enum_value)
        .or(found_attribute_key_value_pair)
        .or(found_identifier)
}

pub fn find_relevant_tag(nodes: &Vec<PositionalNode>) -> Option<&PositionalNode> {
    for node in nodes.iter().rev() {
        if let NodeType::Tag(_) = &node.node_type {
            return Some(node);
        }
    }
    None
}

pub fn has_attribute_error(nodes: &Vec<PositionalNode>) -> bool {
    for node in nodes.iter() {
        if let NodeType::AttributeKeyValuePairError() = &node.node_type {
            return true;
        }
    }
    false
}

pub fn find_relevant_ident(nodes: &Vec<PositionalNode>) -> Option<&PositionalNode> {
    for node in nodes.iter().rev() {
        if let NodeType::Identifier(_) = &node.node_type {
            return Some(node);
        }
    }
    None
}

pub fn is_inside_settings_block(nodes: &Vec<PositionalNode>) -> bool {
    for node in nodes.iter() {
        if let NodeType::Settings = &node.node_type {
            return true;
        }
    }
    false
}

pub fn is_inside_handlers_block(nodes: &Vec<PositionalNode>) -> bool {
    for node in nodes.iter() {
        if let NodeType::Handlers = &node.node_type {
            return true;
        }
    }
    false
}

pub fn is_inside_selector_block(nodes: &Vec<PositionalNode>) -> bool {
    for node in nodes.iter() {
        if let NodeType::SelectorBlock = &node.node_type {
            return true;
        }
    }
    false
}
