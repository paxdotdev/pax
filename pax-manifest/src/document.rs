//! Source-only document contracts and documentation directives.
use pax_message::serde::{Deserialize, Serialize};

/// Static extraction behavior owned by a component declaration.
/// This metadata does not participate in runtime mounting or binary programs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(crate = "pax_message::serde", rename_all = "snake_case")]
pub enum DocumentKind {
    Text,
    Link,
    Children,
    Mask,
    Settings,
}

impl std::str::FromStr for DocumentKind {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "text" => Ok(Self::Text),
            "link" => Ok(Self::Link),
            "children" => Ok(Self::Children),
            "mask" => Ok(Self::Mask),
            "settings" => Ok(Self::Settings),
            _ => Err(format!("unknown document kind `{value}`")),
        }
    }
}

/// A reserved standalone comment affecting only generated document content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentDirective {
    Ignore,
    Heading(u8),
}

impl DocumentDirective {
    /// Ordinary comments have no directive. The exact `///#` namespace is reserved.
    pub fn parse(comment: &str) -> Result<Option<Self>, String> {
        let Some(value) = comment.trim().strip_prefix("///#") else {
            return Ok(None);
        };
        match value {
            "ignore" => Ok(Some(Self::Ignore)),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => Ok(Some(Self::Heading(value.as_bytes()[1] - b'0'))),
            _ => Err(format!("unknown documentation directive `///#{value}`; expected ignore or h1–h6 (no expressions or arguments)")),
        }
    }
}

/// Validate placement while source scopes and comment spans are still available.
#[cfg(feature = "parsing")]
pub fn validate_directives(
    pair: pax_language::Pair<'_, pax_language::Rule>,
    source: &str,
) -> Result<(), String> {
    use pax_language::{Pair, Rule};
    fn walk(pair: Pair<'_, Rule>, source: &str, allowed: bool) -> Result<(), String> {
        if pair.as_rule() == Rule::comment {
            if DocumentDirective::parse(pair.as_str())?.is_some() {
                let (line, col) = pair.as_span().start_pos().line_col();
                if !allowed
                    || !source[..pair.as_span().start()]
                        .rsplit('\n')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .is_empty()
                {
                    return Err(format!("documentation directive at {line}:{col} must be a standalone template comment before a node"));
                }
            }
            return Ok(());
        }
        let sequence = matches!(
            pair.as_rule(),
            Rule::pax_component_definition | Rule::inner_nodes
        );
        if sequence {
            let mut pending = None;
            for child in pair.into_inner() {
                let child = if child.as_rule() == Rule::root_tag_pair {
                    child.into_inner().next().unwrap()
                } else {
                    child
                };
                if child.as_rule() == Rule::comment {
                    walk(child.clone(), source, true)?;
                    if let Some(directive) = DocumentDirective::parse(child.as_str())? {
                        let location = child.as_span().start_pos().line_col();
                        if pending.replace((directive, location)).is_some() {
                            return Err(format!(
                                "conflicting documentation directives before {}:{}",
                                location.0, location.1
                            ));
                        }
                    }
                } else {
                    if let Some((_, (line, col))) = pending.take() {
                        if !matches!(
                            child.as_rule(),
                            Rule::matched_tag
                                | Rule::self_closing_tag
                                | Rule::statement_control_flow
                        ) {
                            return Err(format!("orphan documentation directive at {line}:{col}; cannot cross a section boundary"));
                        }
                    }
                    walk(child, source, false)?;
                }
            }
            if let Some((_, (line, col))) = pending {
                return Err(format!("orphan documentation directive at {line}:{col}; expected a following template node in the same scope"));
            }
        } else {
            for child in pair.into_inner() {
                walk(child, source, false)?;
            }
        }
        Ok(())
    }
    walk(pair, source, false)
}

/// Validate heading receivers in every template, including unreachable routes.
pub fn validate_heading_targets(manifest: &crate::PaxManifest) -> Result<(), String> {
    use crate::*;
    fn all_child_ids(template: &ComponentTemplate) -> Vec<TemplateNodeId> {
        let mut result = vec![];
        let mut pending = template.get_root();
        while let Some(id) = pending.pop() {
            if let Some(children) = template.get_children(&id) {
                result.extend(children.iter().cloned());
                pending.extend(children);
            }
        }
        result
    }

    for component in manifest.components.values() {
        let Some(template) = &component.template else {
            continue;
        };
        let mut lists = vec![template.get_root()];
        for node in template.get_nodes() {
            if let Some(cf) = &node.control_flow_settings {
                lists.extend(cf.conditional_branches.iter().map(|b| b.child_ids.clone()));
                lists.extend(cf.route_branches.iter().map(|b| b.child_ids.clone()));
            }
        }
        // Every ordinary parent, including an if/repeat body, owns an attachment scope.
        for id in template
            .get_root()
            .into_iter()
            .chain(all_child_ids(template))
        {
            if let Some(children) = template.get_children(&id) {
                lists.push(children);
            }
        }
        for ids in lists {
            let mut pending = None;
            for id in ids {
                let Some(node) = template.get_node(&id) else {
                    continue;
                };
                if let Some(comment) = &node.raw_comment_string {
                    if let Some(d) = DocumentDirective::parse(comment)? {
                        pending = Some(d);
                    }
                    continue;
                }
                if matches!(pending.take(), Some(DocumentDirective::Heading(_)))
                    && manifest
                        .components
                        .get(&node.type_id)
                        .and_then(|c| c.document)
                        != Some(DocumentKind::Text)
                {
                    return Err(format!(
                        "{}: heading directives require a text component, received {}",
                        component.type_id, node.type_id
                    ));
                }
            }
        }
    }
    Ok(())
}

#[cfg(all(test, feature = "parsing"))]
mod tests {
    use super::*;
    fn validate(source: &str) -> Result<(), String> {
        let pair =
            pax_language::parse_pax_str(pax_language::Rule::pax_component_definition, source)
                .unwrap();
        validate_directives(pair, source)
    }
    #[test]
    fn directives_are_narrow_and_scope_bound() {
        assert!(validate("///#h2\n// explanation\n<Text>Title</Text>").is_ok());
        assert!(validate("//#anything\n/// prose\n<Text/>").is_ok());
        for source in [
            "///#h7\n<Text/>",
            "///#h2\n///#ignore\n<Text/>",
            "<Group>\n///#ignore\n</Group><Text/>",
            "///#ignore\n@settings {}\n<Text/>",
            "<Text/> ///#ignore\n<Text/>",
            "@settings {\n///#ignore\n}",
        ] {
            assert!(validate(source).is_err(), "{source}");
        }
    }
    #[test]
    fn formatting_keeps_directives_with_their_nodes() {
        let source =
            "<Group>\n///#h2\n<Text>Title</Text>\n///#ignore\n<Text>Example</Text>\n</Group>";
        let formatted = pax_language::formatting::format_pax_template(source.into()).unwrap();
        assert!(validate(&formatted).is_ok());
        assert!(formatted.find("///#h2").unwrap() < formatted.find("Title").unwrap());
        assert!(formatted.find("///#ignore").unwrap() < formatted.find("Example").unwrap());
    }
}
