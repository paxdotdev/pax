use std::collections::HashSet;

use lsp_types::Position;
use pax_language::{Pair, Rule};
use pax_runtime_api::PaxValue;

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
    #[allow(unused)]
    Handlers,
    LiteralFunction(FunctionData),
    LiteralEnumValue(EnumValueData),
    AttributeKeyValuePair(AttributeData),
    #[allow(unused)]
    AttributeKeyValuePairError(),
    XoFunctionCall(FunctionCallData),
}

#[derive(Debug, Clone)]
pub struct IdentifierData {
    pub identifier: String,
    pub _is_pascal_identifier: bool,
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

struct SourcePositions {
    line_starts: Vec<usize>,
    utf16_adjustments: Vec<(usize, usize)>,
}

impl SourcePositions {
    fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        let mut utf16_adjustments = Vec::new();
        let mut adjustment = 0;
        for (offset, ch) in source.char_indices() {
            if ch == '\n' {
                line_starts.push(offset + 1);
            }
            if !ch.is_ascii() {
                adjustment += ch.len_utf8() - ch.len_utf16();
                utf16_adjustments.push((offset + ch.len_utf8(), adjustment));
            }
        }
        Self {
            line_starts,
            utf16_adjustments,
        }
    }

    fn utf16_offset(&self, offset: usize) -> usize {
        let index = self
            .utf16_adjustments
            .partition_point(|(end, _)| *end <= offset);
        offset
            - index
                .checked_sub(1)
                .map(|i| self.utf16_adjustments[i].1)
                .unwrap_or(0)
    }

    fn position(&self, offset: usize) -> Position {
        let line = self.line_starts.partition_point(|start| *start <= offset) - 1;
        Position {
            line: line as u32,
            character: (self.utf16_offset(offset) - self.utf16_offset(self.line_starts[line]))
                as u32,
        }
    }
}

pub fn extract_positional_nodes(
    pair: Pair<'_, Rule>,
    nodes: &mut Vec<PositionalNode>,
    ids: &mut HashSet<String>,
    classes: &mut HashSet<String>,
) {
    // Index once: rescanning every prefix is quadratic for many short bodies.
    let positions = SourcePositions::new(pair.as_span().get_input());
    extract_with_positions(pair, nodes, ids, classes, &positions);
}

fn extract_with_positions(
    pair: Pair<'_, Rule>,
    nodes: &mut Vec<PositionalNode>,
    ids: &mut HashSet<String>,
    classes: &mut HashSet<String>,
    positions: &SourcePositions,
) {
    let span = pair.as_span();
    let (start, end) = (
        positions.position(span.start()),
        positions.position(span.end()),
    );
    let rule = pair.as_rule();
    let as_str = pair.as_str();
    let mut inner = pair.into_inner();

    match rule {
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
        Rule::open_tag | Rule::self_closing_tag => {
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
                        _is_pascal_identifier: true,
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
                    _is_pascal_identifier: true,
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
                    _is_pascal_identifier: true,
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
                    _is_pascal_identifier: false,
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
                    // Use the literal decoder so raw delimiters and commas in
                    // strings cannot become part of the indexed class names.
                    let literals = match pax_language::from_pax(value.trim()) {
                        Ok(PaxValue::String(name)) => vec![PaxValue::String(name)],
                        Ok(PaxValue::Vec(values)) => values,
                        _ => vec![],
                    };
                    for literal in literals {
                        if let PaxValue::String(name) = literal {
                            if !name.is_empty() {
                                classes.insert(name);
                            }
                        }
                    }
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
        Rule::xo_enum_or_function_call => {
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
        extract_with_positions(inner_pair, nodes, ids, classes, positions);
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
    let mut found_xo_enum_or_function_call: Option<&PositionalNode> = None;
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
                found_xo_enum_or_function_call = Some(node);
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
        .or(found_xo_enum_or_function_call)
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

#[cfg(test)]
mod tests {
    use super::extract_positional_nodes;
    use pax_language::{parse_pax_str, Rule};
    use std::collections::HashSet;

    #[test]
    fn body_unicode_uses_utf16_columns_and_raw_markup_is_not_indexed() {
        let source = "<Text>🚀é</Text>\n<Text>{r#\"<Hidden>{word}</Hidden>\"#}</Text>";
        let parsed = parse_pax_str(Rule::pax_component_definition, source).unwrap();
        let mut nodes = Vec::new();
        extract_positional_nodes(parsed, &mut nodes, &mut HashSet::new(), &mut HashSet::new());
        let tags: Vec<_> = nodes
            .iter()
            .filter_map(|n| match &n.node_type {
                super::NodeType::Tag(tag) => Some((
                    tag.pascal_identifier.as_str(),
                    n.start.line,
                    n.start.character,
                )),
                _ => None,
            })
            .collect();
        assert_eq!(
            tags,
            [
                ("Text", 0, 0),
                ("Text", 0, 9),
                ("Text", 1, 0),
                ("Text", 1, 36)
            ]
        );
    }

    #[test]
    fn class_index_reads_literal_bindings_and_skips_expressions() {
        let component = parse_pax_str(
            Rule::pax_component_definition,
            r##"
                <Rectangle class=[r"card", "elevated"] />
                <Group class=r#"raw-single"# />
                <Group class={self.dynamic_classes} />
                @settings { .selected { opacity: 1 } }
            "##,
        )
        .unwrap();
        let mut nodes = Vec::new();
        let mut ids = HashSet::new();
        let mut classes = HashSet::new();

        extract_positional_nodes(component, &mut nodes, &mut ids, &mut classes);

        assert_eq!(
            classes,
            HashSet::from([
                "card".to_string(),
                "elevated".to_string(),
                "selected".to_string(),
                "raw-single".to_string(),
            ])
        );
    }
}
