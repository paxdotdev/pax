//! Bounded, source-only extraction. No runtime, application helpers, or Rust defaults run here.
use color_eyre::eyre::{eyre, Result};
use pax_language::interpreter::{PaxAccessor, PaxExpression, PaxPrimary};
use pax_manifest::document::{DocumentDirective, DocumentKind};
use pax_manifest::*;
use pax_runtime_api::PaxValue;
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

type Values = HashMap<String, Option<PaxValue>>;
const MAX_NODES: usize = 50_000;
const MAX_DEPTH: usize = 64;
const MAX_REPEAT: usize = 256;
const MAX_TEXT_BYTES: usize = 1_048_576;

#[derive(Debug)]
pub(crate) struct StaticDocument {
    pub html: String,
    pub diagnostics: BTreeSet<String>,
}

struct Scope<'a> {
    component: &'a ComponentDefinition,
    values: Values,
    slots: Vec<(Rc<Scope<'a>>, Vec<TemplateNodeId>)>,
}
struct Extractor<'a> {
    manifest: &'a PaxManifest,
    diagnostics: BTreeSet<String>,
    visited: usize,
    imported_fields: BTreeSet<String>,
}

pub(crate) fn extract(manifest: &PaxManifest, path: &str) -> Result<StaticDocument> {
    pax_manifest::document::validate_heading_targets(manifest).map_err(|e| eyre!(e))?;
    let component = manifest
        .components
        .get(&manifest.main_component_type_id)
        .ok_or_else(|| eyre!("Static document: missing main component"))?;
    let mut extractor = Extractor {
        manifest,
        diagnostics: BTreeSet::new(),
        visited: 0,
        imported_fields: imported_fields(manifest),
    };
    let scope = Rc::new(Scope {
        component,
        values: Values::new(),
        slots: vec![],
    });
    let segments: Vec<_> = path.split('/').filter(|s| !s.is_empty()).collect();
    let html = extractor.component(&scope, &segments, 0)?;
    Ok(StaticDocument {
        html,
        diagnostics: extractor.diagnostics,
    })
}

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

impl<'a> Extractor<'a> {
    fn note(&mut self, scope: &Scope, node: &TemplateNodeDefinition, reason: &str) {
        let position = node
            .selector_info
            .source_location
            .as_ref()
            .map(|p| format!(":{}:{}", p.start_line_col.0 + 1, p.start_line_col.1 + 1))
            .unwrap_or_default();
        self.diagnostics
            .insert(format!("{}{position}: {reason}", scope.component.type_id));
    }
    fn component(&mut self, scope: &Rc<Scope<'a>>, path: &[&str], depth: usize) -> Result<String> {
        let Some(template) = &scope.component.template else {
            return Ok(String::new());
        };
        self.nodes(scope, &template.get_root(), path, depth + 1)
    }
    fn nodes(
        &mut self,
        scope: &Rc<Scope<'a>>,
        ids: &[TemplateNodeId],
        path: &[&str],
        depth: usize,
    ) -> Result<String> {
        if depth > MAX_DEPTH || self.visited >= MAX_NODES {
            self.diagnostics.insert(format!(
                "{}: static document traversal limit reached",
                scope.component.type_id
            ));
            return Ok(String::new());
        }
        let template = scope.component.template.as_ref().unwrap();
        let mut output = String::new();
        let mut pending = None;
        for id in ids {
            let Some(node) = template.get_node(id) else {
                continue;
            };
            if let Some(comment) = &node.raw_comment_string {
                if let Some(d) = DocumentDirective::parse(comment).map_err(|e| eyre!(e))? {
                    pending = Some(d);
                }
                continue;
            }
            let directive = pending.take();
            if directive == Some(DocumentDirective::Ignore) {
                continue;
            }
            self.visited += 1;
            if self.visited > MAX_NODES {
                self.note(scope, node, "static document node limit reached");
                break;
            }
            output.push_str(&self.node(scope, id, node, directive, path, depth + 1)?);
        }
        Ok(output)
    }
    fn node(
        &mut self,
        scope: &Rc<Scope<'a>>,
        id: &TemplateNodeId,
        node: &TemplateNodeDefinition,
        directive: Option<DocumentDirective>,
        path: &[&str],
        depth: usize,
    ) -> Result<String> {
        let template = scope.component.template.as_ref().unwrap();
        let children = template.get_children(id).unwrap_or_default();
        if let Some(cf) = &node.control_flow_settings {
            match node.type_id.get_pax_type() {
                PaxType::Slot => {
                    let slots = if let Some(index) = &cf.slot_index_expression {
                        if scope.slots.iter().any(|(parent, ids)| {
                            ids.iter().any(|id| {
                                parent
                                    .component
                                    .template
                                    .as_ref()
                                    .unwrap()
                                    .get_node(id)
                                    .is_some_and(|n| n.control_flow_settings.is_some())
                            })
                        }) {
                            self.note(
                                scope,
                                node,
                                "indexed slot with control-flow projection; content omitted",
                            );
                            return Ok(String::new());
                        }
                        let Some(index) =
                            eval(&index.expression, &scope.values).and_then(index_value)
                        else {
                            self.note(
                                scope,
                                node,
                                "unresolved slot index; projected content omitted",
                            );
                            return Ok(String::new());
                        };
                        scope.slots.get(index).into_iter().collect::<Vec<_>>()
                    } else {
                        scope.slots.iter().collect()
                    };
                    let mut output = String::new();
                    for (parent, ids) in slots {
                        output.push_str(&self.nodes(parent, ids, path, depth)?);
                    }
                    return Ok(output);
                }
                PaxType::If => {
                    if cf.conditional_branches.is_empty() {
                        return match cf
                            .condition_expression
                            .as_ref()
                            .and_then(|e| eval(&e.expression, &scope.values))
                            .and_then(bool_value)
                        {
                            Some(true) => self.nodes(scope, &children, path, depth),
                            Some(false) => Ok(String::new()),
                            None => {
                                self.note(scope, node, "unresolved condition; branch omitted");
                                Ok(String::new())
                            }
                        };
                    }
                    for branch in &cf.conditional_branches {
                        let take = branch
                            .condition_expression
                            .as_ref()
                            .map(|e| eval(&e.expression, &scope.values).and_then(bool_value))
                            .unwrap_or(Some(true));
                        match take {
                            Some(true) => return self.nodes(scope, &branch.child_ids, path, depth),
                            Some(false) => {}
                            None => {
                                self.note(scope, node, "unresolved condition; conditional omitted");
                                return Ok(String::new());
                            }
                        }
                    }
                    return Ok(String::new());
                }
                PaxType::Repeat => {
                    let values = cf
                        .repeat_source_expression
                        .as_ref()
                        .and_then(|e| eval(&e.expression, &scope.values))
                        .and_then(iterable);
                    let Some(values) = values else {
                        self.note(
                            scope,
                            node,
                            "unresolved or oversized repeat; content omitted",
                        );
                        return Ok(String::new());
                    };
                    let mut output = String::new();
                    for (index, value) in values.into_iter().enumerate() {
                        let mut env = scope.values.clone();
                        match &cf.repeat_predicate_definition {
                            Some(ControlFlowRepeatPredicateDefinition::ElemId(name)) => {
                                env.insert(name.clone(), Some(value));
                            }
                            Some(ControlFlowRepeatPredicateDefinition::ElemIdIndexId(name, i)) => {
                                env.insert(name.clone(), Some(value));
                                env.insert(i.clone(), Some(PaxValue::Numeric(index.into())));
                            }
                            None => break,
                        }
                        let iteration = Rc::new(Scope {
                            component: scope.component,
                            values: env,
                            slots: scope.slots.clone(),
                        });
                        output.push_str(&self.nodes(&iteration, &children, path, depth)?);
                    }
                    return Ok(output);
                }
                PaxType::Router => {
                    let selected = cf
                        .route_branches
                        .iter()
                        .filter(|b| !b.default)
                        .find_map(|b| route_remainder(b.path.as_deref()?, path).map(|n| (b, n)))
                        .or_else(|| cf.route_branches.iter().find(|b| b.default).map(|b| (b, 0)));
                    return if let Some((branch, consumed)) = selected {
                        self.nodes(scope, &branch.child_ids, &path[consumed..], depth)
                    } else {
                        Ok(String::new())
                    };
                }
                _ => {}
            }
        }
        let Some(component) = self.manifest.components.get(&node.type_id) else {
            return Ok(String::new());
        };
        let props = properties(scope, node, &self.imported_fields);
        match component.document {
            Some(DocumentKind::Text) => {
                let text_field = self
                    .manifest
                    .type_table
                    .get(&component.type_id)
                    .and_then(|t| t.property_definitions.iter().find(|p| p.flags.is_content))
                    .map(|p| p.name.as_str())
                    .unwrap_or("text");
                let Some(PaxValue::String(text)) = props.get(text_field).and_then(Option::as_ref)
                else {
                    self.note(scope,node,"unresolved text (Rust defaults, helpers, imported settings, and lifecycle state are not evaluated)");
                    return Ok(String::new());
                };
                if text.is_empty() {
                    return Ok(String::new());
                }
                if text.len() > MAX_TEXT_BYTES {
                    self.note(scope, node, "static text exceeds 1 MiB; content omitted");
                    return Ok(String::new());
                }
                let markdown = match props.get("markdown") {
                    None => false, // Part of the Text extraction contract, matching Text's default.
                    Some(Some(PaxValue::Bool(value))) => *value,
                    _ => {
                        self.note(scope, node, "unresolved markdown mode; text omitted");
                        return Ok(String::new());
                    }
                };
                let heading = if let Some(DocumentDirective::Heading(level)) = directive {
                    Some(level)
                } else {
                    None
                };
                let body = if markdown {
                    markdown_html(text, heading.is_some())
                } else {
                    escape(text)
                };
                let tag = heading
                    .map(|l| format!("h{l}"))
                    .unwrap_or_else(|| if markdown { "div" } else { "p" }.into());
                Ok(format!("<{tag}>{body}</{tag}>\n"))
            }
            Some(DocumentKind::Link) => {
                let content = self.nodes(scope, &children, path, depth)?;
                if content.is_empty() {
                    return Ok(content);
                }
                if content.contains("<a ") {
                    self.note(
                        scope,
                        node,
                        "nested links; retaining child links without an outer anchor",
                    );
                    return Ok(content);
                }
                match props.get("url").and_then(Option::as_ref) {
                    Some(PaxValue::String(url)) if safe_url(url) => {
                        Ok(format!("<a href=\"{}\">{content}</a>\n", escape(url)))
                    }
                    _ => {
                        self.note(
                            scope,
                            node,
                            "unresolved or unsafe link URL; retaining its text without a link",
                        );
                        Ok(content)
                    }
                }
            }
            Some(DocumentKind::Children) => self.nodes(scope, &children, path, depth),
            Some(DocumentKind::Mask) => {
                // Keep comments attached to the first actual child. The mask-source subtree is not content.
                let end = children.iter().position(|id| {
                    template
                        .get_node(id)
                        .is_some_and(|n| n.raw_comment_string.is_none())
                });
                self.nodes(scope, &children[..end.map_or(0, |i| i + 1)], path, depth)
            }
            Some(DocumentKind::Settings) => Ok(String::new()),
            None if component.route_branch.is_some() => self.nodes(scope, &children, path, depth),
            None => {
                let mut slots = vec![];
                let mut attached = vec![];
                for id in children {
                    let is_comment = template
                        .get_node(&id)
                        .is_some_and(|n| n.raw_comment_string.is_some());
                    attached.push(id);
                    if !is_comment {
                        slots.push((scope.clone(), std::mem::take(&mut attached)));
                    }
                }
                let inner = Rc::new(Scope {
                    component,
                    values: props,
                    slots,
                });
                self.component(&inner, path, depth)
            }
        }
    }
}

// Identical literal/parameter/catch-all matching to Router; a catch-all leaves its tail for nested routers.
fn route_remainder(pattern: &str, path: &[&str]) -> Option<usize> {
    let mut consumed = 0;
    for part in pattern.split('/').filter(|s| !s.is_empty()) {
        if part == "*" {
            return Some(consumed);
        }
        let actual = path.get(consumed)?;
        if !part.starts_with(':') && part != *actual {
            return None;
        }
        consumed += 1;
    }
    (consumed == path.len()).then_some(consumed)
}

fn properties(scope: &Scope, node: &TemplateNodeDefinition, imported: &BTreeSet<String>) -> Values {
    let mut values = Values::new();
    let classes = node
        .selector_info
        .class_binding
        .as_ref()
        .map(|v| value(v, &scope.values));
    let classes: Option<Vec<String>> = match classes {
        None => Some(vec![]),
        Some(Some(PaxValue::String(c))) => Some(vec![c]),
        Some(Some(PaxValue::Vec(cs))) => cs
            .into_iter()
            .map(|c| {
                if let PaxValue::String(s) = c {
                    Some(s)
                } else {
                    None
                }
            })
            .collect(),
        _ => None,
    };
    let mut blocks = vec![];
    selector_blocks(
        scope.component.settings.as_deref().unwrap_or_default(),
        &scope.values,
        true,
        &mut blocks,
    );
    // Runtime precedence: type, ordered classes, id, imports, then inline values.
    for (selector, fields, known) in &blocks {
        if matches!(selector, SelectorExpr::Type(_))
            && node.selector_info.matches(&node.type_id, selector)
        {
            apply(&mut values, fields, &scope.values, *known);
        }
    }
    if let Some(classes) = classes {
        for class in classes {
            for (selector, fields, known) in &blocks {
                if selector == &SelectorExpr::Class(class.clone()) {
                    apply(&mut values, fields, &scope.values, *known);
                }
            }
        }
    } else {
        for (selector, fields, _) in &blocks {
            if matches!(selector, SelectorExpr::Class(_)) {
                apply(&mut values, fields, &scope.values, false);
            }
        }
    }
    for (selector, fields, known) in &blocks {
        if matches!(selector, SelectorExpr::Id(_))
            && node.selector_info.matches(&node.type_id, selector)
        {
            apply(&mut values, fields, &scope.values, *known);
        }
    }
    // ImportSettings can export runtime-selected providers to distant consumers. Until that
    // scope is modeled here, any field such a provider could override is unknown, not a guess.
    for field in imported {
        values.insert(field.clone(), None);
    }
    apply(
        &mut values,
        node.settings.as_deref().unwrap_or_default(),
        &scope.values,
        true,
    );
    values
}
fn apply(values: &mut Values, fields: &[SettingElement], env: &Values, known: bool) {
    for field in fields {
        if let SettingElement::Setting(name, v) = field {
            values.insert(
                name.token_value.clone(),
                if known { value(v, env) } else { None },
            );
        }
    }
}
fn selector_blocks<'a>(
    settings: &'a [SettingsBlockElement],
    env: &Values,
    known: bool,
    out: &mut Vec<(SelectorExpr, &'a [SettingElement], bool)>,
) {
    for setting in settings {
        match setting {
            SettingsBlockElement::SelectorBlock(selector, block) => {
                if let Ok(s) = SelectorExpr::parse(&selector.token_value) {
                    out.push((s, &block.elements, known));
                }
            }
            SettingsBlockElement::Conditional(block) => {
                let mut uncertain = !known;
                for branch in &block.branches {
                    let take = branch
                        .condition_expression
                        .as_ref()
                        .map(|e| eval(&e.expression, env).and_then(bool_value))
                        .unwrap_or(Some(true));
                    if take == Some(false) {
                        continue;
                    }
                    uncertain |= take.is_none();
                    selector_blocks(&branch.elements, env, !uncertain, out);
                    if take == Some(true) {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
}
fn imported_fields(manifest: &PaxManifest) -> BTreeSet<String> {
    fn providers(
        manifest: &PaxManifest,
        id: &TypeId,
        seen: &mut BTreeSet<String>,
        fields: &mut BTreeSet<String>,
    ) {
        if !seen.insert(id.to_string()) {
            return;
        }
        let Some(component) = manifest.components.get(id) else {
            return;
        };
        let mut blocks = vec![];
        selector_blocks(
            component.settings.as_deref().unwrap_or_default(),
            &Values::new(),
            false,
            &mut blocks,
        );
        for (_, block, _) in blocks {
            for field in block {
                if let SettingElement::Setting(name, _) = field {
                    fields.insert(name.token_value.clone());
                }
            }
        }
        if let Some(t) = &component.template {
            for node in t.get_nodes() {
                providers(manifest, &node.type_id, seen, fields);
            }
        }
    }
    let mut result = BTreeSet::new();
    let mut seen = BTreeSet::new();
    for component in manifest.components.values() {
        if let Some(t) = &component.template {
            for id in t.get_root().into_iter().chain(all_child_ids(t)) {
                if t.get_node(&id)
                    .and_then(|n| manifest.components.get(&n.type_id))
                    .and_then(|c| c.document)
                    == Some(DocumentKind::Settings)
                {
                    let mut descendants = t.get_children(&id).unwrap_or_default();
                    while let Some(child) = descendants.pop() {
                        if let Some(n) = t.get_node(&child) {
                            providers(manifest, &n.type_id, &mut seen, &mut result);
                        }
                        descendants.extend(t.get_children(&child).unwrap_or_default());
                    }
                }
            }
        }
    }
    result
}

fn value(v: &ValueDefinition, env: &Values) -> Option<PaxValue> {
    match v {
        ValueDefinition::LiteralValue(v) => Some(v.clone()),
        ValueDefinition::Expression(e) => eval(&e.expression, env),
        ValueDefinition::Identifier(i) => env
            .get(i.name.trim_start_matches("self."))
            .cloned()
            .flatten(),
        ValueDefinition::List(v) => Some(PaxValue::Vec(
            v.iter().map(|v| value(v, env)).collect::<Option<_>>()?,
        )),
        // Two-way bindings can be mutated outside the template and are intentionally unknown.
        _ => None,
    }
}
fn eval(expression: &PaxExpression, env: &Values) -> Option<PaxValue> {
    match expression {
        PaxExpression::Primary(p) => match p.as_ref() {
            PaxPrimary::Literal(v) => Some(v.clone()),
            PaxPrimary::Grouped(e, None) => eval(e, env),
            PaxPrimary::Identifier(i, accessors) => {
                let mut v = env
                    .get(i.name.trim_start_matches("self."))
                    .cloned()
                    .flatten()?;
                for a in accessors {
                    v = match (v, a) {
                        (PaxValue::Vec(v), PaxAccessor::Tuple(i)) => v.get(*i)?.clone(),
                        (PaxValue::Vec(v), PaxAccessor::List(e)) => {
                            v.get(index_value(eval(e, env)?)?)?.clone()
                        }
                        (PaxValue::Object(v), PaxAccessor::Struct(k)) => {
                            v.into_iter().find(|(n, _)| n == k)?.1
                        }
                        _ => return None,
                    };
                }
                Some(v)
            }
            PaxPrimary::List(v) | PaxPrimary::Tuple(v) => Some(PaxValue::Vec(
                v.iter().map(|e| eval(e, env)).collect::<Option<_>>()?,
            )),
            PaxPrimary::Object(v) => Some(PaxValue::Object(
                v.iter()
                    .map(|(k, e)| Some((k.clone(), eval(e, env)?)))
                    .collect::<Option<_>>()?,
            )),
            PaxPrimary::Range(a, b) => Some(PaxValue::Range(
                Box::new(eval(a, env)?),
                Box::new(eval(b, env)?),
            )),
            _ => None,
        },
        PaxExpression::Ternary(t) => eval(
            if bool_value(eval(t.condition(), env)?)? {
                t.then_branch()
            } else {
                t.else_branch()
            },
            env,
        ),
        PaxExpression::Prefix(p) => match (p.operator_name(), eval(p.rhs(), env)?) {
            ("!", PaxValue::Bool(v)) => Some(PaxValue::Bool(!v)),
            _ => None,
        },
        PaxExpression::Infix(i) => {
            let a = eval(i.lhs(), env)?;
            if i.operator_name() == "&&" && a == PaxValue::Bool(false) {
                return Some(a);
            }
            if i.operator_name() == "||" && a == PaxValue::Bool(true) {
                return Some(a);
            }
            let b = eval(i.rhs(), env)?;
            match (i.operator_name(), a, b) {
                ("+", PaxValue::String(a), PaxValue::String(b))
                    if a.len().checked_add(b.len())? <= MAX_TEXT_BYTES =>
                {
                    Some(PaxValue::String(a + &b))
                }
                ("==", a, b) => Some(PaxValue::Bool(a == b)),
                ("!=", a, b) => Some(PaxValue::Bool(a != b)),
                ("&&", PaxValue::Bool(a), PaxValue::Bool(b)) => Some(PaxValue::Bool(a && b)),
                ("||", PaxValue::Bool(a), PaxValue::Bool(b)) => Some(PaxValue::Bool(a || b)),
                _ => None,
            }
        }
        _ => None,
    }
}
fn bool_value(v: PaxValue) -> Option<bool> {
    if let PaxValue::Bool(v) = v {
        Some(v)
    } else {
        None
    }
}
fn index_value(v: PaxValue) -> Option<usize> {
    if let PaxValue::Numeric(n) = v {
        let n = n.to_float();
        if n >= 0. && n.fract() == 0. && n < usize::MAX as f64 {
            return Some(n as usize);
        }
    }
    None
}
fn iterable(v: PaxValue) -> Option<Vec<PaxValue>> {
    match v {
        PaxValue::Vec(v) if v.len() <= MAX_REPEAT => Some(v),
        PaxValue::Range(a, b) => {
            let a = index_value(*a)?;
            let b = index_value(*b)?;
            if b.saturating_sub(a) > MAX_REPEAT {
                return None;
            }
            Some((a..b).map(|n| PaxValue::Numeric(n.into())).collect())
        }
        _ => None,
    }
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn safe_url(url: &str) -> bool {
    if url.chars().any(|c| c.is_control() || c.is_whitespace()) || url.contains('\\') {
        return false;
    }
    if let Ok(parsed) = reqwest::Url::parse(url) {
        matches!(parsed.scheme(), "http" | "https" | "mailto" | "tel")
    } else {
        !url.starts_with("//")
            && !url
                .split(['/', '?', '#'])
                .next()
                .unwrap_or("")
                .contains(':')
    }
}
fn markdown_html(text: &str, inline: bool) -> String {
    use pulldown_cmark::{Event, Parser, Tag, TagEnd};
    let mut links = Vec::new();
    let events = Parser::new(text).filter_map(|event| match event {
        Event::Html(_) | Event::InlineHtml(_) => None,
        Event::Start(Tag::Image { .. }) | Event::End(TagEnd::Image) => None,
        Event::Start(ref tag)
            if inline
                && !matches!(
                    tag,
                    Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Link { .. }
                ) =>
        {
            None
        }
        Event::End(tag)
            if inline
                && !matches!(
                    tag,
                    TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link
                ) =>
        {
            Some(Event::Text(" ".into()))
        }
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) => {
            let allowed = safe_url(&dest_url);
            links.push(allowed);
            allowed.then_some(Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }))
        }
        Event::End(TagEnd::Link) => links
            .pop()
            .unwrap_or(false)
            .then_some(Event::End(TagEnd::Link)),
        other => Some(other),
    });
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, events);
    html
}

/// Native HTML disclosure works before JS and remains reachable after Pax mounts.
pub(crate) fn package(html: &str, content: &str, path: &str) -> String {
    const START: &str = "<!-- pax-static-document:start -->";
    const END: &str = "<!-- pax-static-document:end -->";
    let mut html = html.to_owned();
    if let (Some(a), Some(b)) = (html.find(START), html.find(END)) {
        html.replace_range(a..b + END.len(), "");
    }
    if content.is_empty() {
        return html;
    }
    let block = format!(
        r#"{START}
<style>
#pax-page-text {{ position:fixed; z-index:2147483647; bottom:12px; left:12px; max-width:calc(100vw - 24px); color:#eee; background:#171717; border:1px solid #666; border-radius:6px; font:14px/1.6 system-ui,sans-serif; }}
#pax-page-text > summary {{ cursor:pointer; padding:6px 12px; position:sticky; top:0; background:#171717; }}
#pax-page-text[open] {{ inset:12px; overflow:auto; overscroll-behavior:contain; }}
#pax-page-text article {{ max-width:70ch; padding:12px 24px 32px; margin:auto; }}
#pax-page-text a {{ color:#a8d9ff; }}
#pax-page-text p {{ white-space:pre-wrap; }}
#pax-page-text :focus-visible {{ outline:2px solid #a8d9ff; outline-offset:3px; }}
</style>
<details id="pax-page-text" data-route="{}"><summary>Read page text</summary><article aria-label="Page text">
{}
</article></details>
{END}"#,
        escape(path),
        content
    );
    // Parse the reading view before the blocking bundle script, so even a slow
    // or failed script download cannot hold the source content behind it.
    let lower = html.to_ascii_lowercase();
    let at = lower
        .find("<body")
        .and_then(|start| lower[start..].find('>').map(|end| start + end + 1))
        .unwrap_or(0);
    html.insert_str(at, &block);
    html
}

#[cfg(test)]
mod tests {
    use super::*;
    use pax_manifest::parsing::{
        parse_settings_from_component_definition_string,
        parse_template_from_component_definition_string, TemplateNodeParseContext,
    };
    use std::collections::BTreeMap;

    fn fixture(templates: &[(&str, &str)]) -> PaxManifest {
        let mut kinds = vec![
            ("Text", Some(DocumentKind::Text)),
            ("Group", Some(DocumentKind::Children)),
            ("Link", Some(DocumentKind::Link)),
            ("Mask", Some(DocumentKind::Mask)),
            ("ImportSettings", Some(DocumentKind::Settings)),
            ("Route", None),
        ];
        kinds.extend(templates.iter().map(|(name, _)| (*name, None)));
        let ids: HashMap<_, _> = kinds
            .iter()
            .map(|(name, _)| {
                (
                    name.to_string(),
                    TypeId::build_singleton(&format!("test::{name}"), Some(name)),
                )
            })
            .collect();
        let descriptors = HashMap::from([(
            ids["Route"].clone(),
            RouteBranchDescriptor {
                path_property: "path".into(),
                default_property: "default".into(),
                modal: false,
            },
        )]);
        let mut components = BTreeMap::new();
        for (name, kind) in kinds {
            let type_id = ids[name].clone();
            let source = templates.iter().find(|(n, _)| *n == name).map(|(_, s)| *s);
            let mut component = ComponentDefinition {
                document: kind,
                type_id: type_id.clone(),
                is_main_component: name == "App",
                is_primitive: kind.is_some(),
                is_struct_only_component: false,
                module_path: "test".into(),
                primitive_instance_import_path: None,
                template: None,
                settings: None,
                timelines: vec![],
                camera: None,
                route_branch: descriptors.get(&type_id).cloned(),
            };
            if let Some(source) = source {
                let pair = pax_language::parse_pax_str(
                    pax_language::Rule::pax_component_definition,
                    source,
                )
                .unwrap();
                component.settings = Some(parse_settings_from_component_definition_string(
                    pair.clone(),
                ));
                let mut ctx = TemplateNodeParseContext {
                    template: ComponentTemplate::new(type_id.clone(), None),
                    pascal_identifier_to_type_id_map: ids.clone(),
                    route_branch_descriptors: descriptors.clone(),
                    content_properties: HashMap::from([(ids["Text"].clone(), "text".into())]),
                };
                parse_template_from_component_definition_string(&mut ctx, source, pair);
                component.template = Some(ctx.template);
            }
            components.insert(type_id, component);
        }
        PaxManifest {
            components,
            main_component_type_id: ids["App"].clone(),
            type_table: HashMap::new(),
            assets_dirs: vec![],
            engine_import_path: "pax_engine".into(),
        }
    }
    #[test]
    fn props_slots_directives_and_unknown_branches() {
        let m = fixture(&[
            (
                "App",
                r#"
            <Card title="A & B">
                ///#h2
                // still attached through projection
                <Text>Projected title</Text>
                ///#ignore
                <Text>Hidden demo</Text>
                <Text>Tail</Text>
            </Card>
            if self.loading { <Text>Unknown branch</Text> } else { <Text>Not a default</Text> }
            if true { <Text>Known branch</Text> }
        "#,
            ),
            ("Card", r#"<Text>{self.title}</Text><Group>slot()</Group>"#),
        ]);
        let d = extract(&m, "/").unwrap();
        assert!(d.html.contains("A &amp; B"), "{}", d.html);
        assert!(d.html.contains("<h2>Projected title</h2>"));
        assert!(d.html.contains("Tail") && d.html.contains("Known branch"));
        assert!(
            !d.html.contains("Hidden demo")
                && !d.html.contains("Unknown branch")
                && !d.html.contains("Not a default")
        );
        assert_eq!(d.diagnostics.len(), 1, "{:?}", d.diagnostics);
    }
    #[test]
    fn numbered_slots_do_not_count_comments() {
        let m = fixture(&[
            (
                "App",
                "<Card>\n///#h3\n<Text>First</Text><Text>Second</Text></Card>",
            ),
            ("Card", "slot(1) slot(0)"),
        ]);
        let d = extract(&m, "/").unwrap();
        assert!(d.html.find("Second").unwrap() < d.html.find("<h3>First").unwrap());
    }
    #[test]
    fn bounded_repeat_and_selector_precedence() {
        let m = fixture(&[
            (
                "App",
                r#"
            <Text class=["a", "b"] id=chosen/>
            <Text class="a" text="Inline"/>
            <Items values=["One", "Two"]/>
            for i in 0..1000000 { <Text>Too much</Text> }
            @settings {
                .b { text: "B" }
                .a { text: "A" }
                #chosen { text: "ID" }
            }
        "#,
            ),
            ("Items", "for v in self.values { <Text>{v}</Text> }"),
        ]);
        let d = extract(&m, "/").unwrap();
        assert!(
            d.html.contains(">ID<") && d.html.contains(">Inline<"),
            "{}",
            d.html
        );
        assert!(d.html.contains(">One<") && d.html.contains(">Two<"));
        assert!(!d.html.contains("Too much"));
        assert_eq!(d.diagnostics.len(), 1);
    }
    #[test]
    fn unknown_settings_override_without_guessing_and_inline_can_resolve_them() {
        let m = fixture(&[(
            "App",
            r#"
            <Text class="copy"/><Text class="copy" text="Known"/>
            @settings { .copy { text: "A" } if self.state { .copy {text: "B"} } }
        "#,
        )]);
        let d = extract(&m, "/").unwrap();
        assert_eq!(d.html, "<p>Known</p>\n");
        assert_eq!(d.diagnostics.len(), 1);
    }
    #[test]
    fn imported_content_is_unknown_until_overridden_inline() {
        let m = fixture(&[
            (
                "App",
                r#"
            <ImportSettings>if self.theme_enabled { <Theme/> }</ImportSettings>
            <Text class="copy"/>
            <Text class="copy">Inline survives</Text>
            @settings { .copy { text: "Local value" } }
        "#,
            ),
            (
                "Theme",
                r#"@settings { .copy { text: {self.runtime_copy} } }"#,
            ),
        ]);
        let d = extract(&m, "/").unwrap();
        assert_eq!(d.html, "<p>Inline survives</p>\n");
        assert_eq!(d.diagnostics.len(), 1);
    }

    #[test]
    fn nested_routes_are_isolated_and_masks_exclude_the_source() {
        let m = fixture(&[(
            "App",
            r#"
            <Router>
                <Route path="/"><Text>Home</Text></Route>
                <Route path="/docs/*"><Router>
                    <Route path="intro"><Mask><Text>Introduction</Text><Text>Mask source</Text></Mask></Route>
                    <Route default=true><Text>Docs fallback</Text></Route>
                </Router></Route>
                <Route default=true><Text>Missing</Text></Route>
            </Router>
        "#,
        )]);
        assert_eq!(extract(&m, "/").unwrap().html, "<p>Home</p>\n");
        assert_eq!(
            extract(&m, "/docs/intro").unwrap().html,
            "<p>Introduction</p>\n"
        );
        assert_eq!(
            extract(&m, "/docs/other").unwrap().html,
            "<p>Docs fallback</p>\n"
        );
        assert_eq!(extract(&m, "/other").unwrap().html, "<p>Missing</p>\n");
    }
    #[test]
    fn escaping_markdown_and_link_safety() {
        let m = fixture(&[(
            "App",
            r#"
            <Link url="javascript:alert(1)"><Text>&lt;script&gt;&amp;</Text></Link>
            <Link url="https://example.com/?a=1&b=2"><Text>Safe</Text></Link>
            <Text markdown=true>**Bold** [Docs](https://example.com) [Bad](javascript:bad)</Text>
        "#,
        )]);
        let d = extract(&m, "/").unwrap();
        assert!(!d.html.contains("javascript:") && !d.html.contains("<script>"));
        assert!(d.html.contains("&lt;script&gt;&amp;"));
        assert!(d.html.contains("<strong>Bold</strong>"));
        assert!(d.html.contains("href=\"https://example.com/?a=1&amp;b=2\""));
    }
    #[test]
    fn heading_on_a_container_is_an_error_even_if_ignored_by_another_route() {
        let m = fixture(&[("App", "///#h2\n<Group><Text>Wrong target</Text></Group>")]);
        assert!(extract(&m, "/")
            .unwrap_err()
            .to_string()
            .contains("text component"));
    }
    #[test]
    fn packaging_is_idempotent_and_does_not_gate_mount() {
        let d = extract(
            &fixture(&[("App", "<Text>Read without JavaScript</Text>")]),
            "/",
        )
        .unwrap();
        let html = "<html><body><div id=\"mount\"></div></body></html>";
        let once = package(html, &d.html, "/");
        let twice = package(&once, &d.html, "/");
        assert_eq!(once, twice);
        assert!(
            once.contains("<details id=\"pax-page-text\" data-route=\"/\">")
                && once.contains("Read without JavaScript")
        );
        assert!(once.contains("<div id=\"mount\"></div>"));
    }
    #[test]
    fn document_contracts_and_directives_do_not_enter_baked_programs() {
        let m = fixture(&[("App", "///#h1\n<Text>Title</Text>")]);
        let program = pax_manifest::program_ir::ProgramIR::from(&m);
        let encoded = pax_manifest::program_ir::binary::to_vec(&program).unwrap();
        let decoded = pax_manifest::program_ir::binary::from_slice(&encoded).unwrap();
        assert_eq!(
            pax_manifest::program_ir::binary::to_vec(&decoded).unwrap(),
            encoded
        );
        let template = decoded.components[&m.main_component_type_id]
            .template
            .as_ref()
            .unwrap();
        assert!(template
            .get_nodes()
            .iter()
            .all(|node| node.raw_comment_string.is_none()));
        let text_id = TypeId::build_singleton("test::Text", Some("Text"));
        let rich = serde_json::to_value(&m.components[&text_id]).unwrap();
        let baked = serde_json::to_value(&decoded.components[&text_id]).unwrap();
        assert_eq!(rich["document"], "text");
        assert!(baked.get("document").is_none());
    }
}
