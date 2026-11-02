//! Normalize authored Markdown only at the desktop JSON boundary. The public
//! TextPatch and release cartridge representation remain platform independent.
//!
//! Runs cover UTF-8 byte ranges and carry effective inline overrides. Omitted
//! properties inherit the node's current TextStyle, so style-only patches do not
//! require reparsing content. Native hosts own shaping, selection and navigation.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use quick_xml::events::Event as XmlEvent;
use quick_xml::Reader;
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
struct Style {
    #[serde(skip_serializing_if = "Option::is_none")]
    bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    underline: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    strikethrough: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    font_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size_scale: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<[f64; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    link: Option<String>,
    /// Keeps adjacent authored links distinct even when their destinations match.
    #[serde(skip_serializing_if = "Option::is_none")]
    link_id: Option<u64>,
}

#[derive(Debug, Serialize)]
struct Run {
    start: usize,
    end: usize,
    #[serde(flatten)]
    style: Style,
}

#[derive(Debug, Default, Serialize)]
struct Document {
    text: String,
    runs: Vec<Run>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line_height: Option<f64>,
}

#[derive(Debug, PartialEq)]
enum Scope {
    Markdown(TagEnd),
    Html(String),
}

#[derive(Default)]
struct Builder {
    document: Document,
    stack: Vec<(Scope, Style)>,
    lists: Vec<Option<u64>>,
    item_paragraphs: Vec<usize>,
    pending_breaks: usize,
    pre_seen: bool,
    pre_closed: bool,
    link_sequence: u64,
}

impl Builder {
    fn next_link_id(&mut self) -> Result<u64, String> {
        self.link_sequence = self
            .link_sequence
            .checked_add(1)
            .ok_or("desktop rich text link identity overflow")?;
        Ok(self.link_sequence)
    }

    fn style(&self) -> Style {
        self.stack
            .last()
            .map(|entry| entry.1.clone())
            .unwrap_or_default()
    }

    fn push(&mut self, scope: Scope, style: Style) -> Result<(), String> {
        // Bound hostile/deep authored fragments without recursive parsing.
        if self.stack.len() >= 128 {
            return Err("desktop rich text nesting exceeds 128 levels".into());
        }
        self.stack.push((scope, style));
        Ok(())
    }

    fn pop(&mut self, scope: Scope) -> Result<(), String> {
        match self.stack.pop() {
            Some((found, _)) if found == scope => Ok(()),
            _ => Err(format!("unbalanced desktop rich text: expected {scope:?}")),
        }
    }

    fn append_raw(&mut self, text: &str, style: Style) {
        if text.is_empty() {
            return;
        }
        let start = self.document.text.len();
        self.document.text.push_str(text);
        let end = self.document.text.len();
        if let Some(last) = self.document.runs.last_mut() {
            if last.end == start && last.style == style {
                last.end = end;
                return;
            }
        }
        self.document.runs.push(Run { start, end, style });
    }

    fn append(&mut self, text: &str) -> Result<(), String> {
        if self.pre_closed {
            if text.trim().is_empty() {
                return Ok(());
            }
            return Err("desktop rich text pre must be the entire document".into());
        }
        if text.is_empty() {
            return Ok(());
        }
        if self.pending_breaks > 0 && !self.document.text.is_empty() {
            let existing = self
                .document
                .text
                .bytes()
                .rev()
                .take_while(|b| *b == b'\n')
                .count();
            if existing < self.pending_breaks {
                self.append_raw(
                    &"\n".repeat(self.pending_breaks - existing),
                    Style::default(),
                );
            }
        }
        self.pending_breaks = 0;
        self.append_raw(text, self.style());
        Ok(())
    }

    fn separate(&mut self, lines: usize) {
        self.pending_breaks = self.pending_breaks.max(lines);
    }

    fn start_markdown(&mut self, tag: Tag<'_>) -> Result<(), String> {
        let end = tag.to_end();
        let mut style = self.style();
        match tag {
            Tag::Paragraph => {
                if let Some(paragraphs) = self.item_paragraphs.last_mut() {
                    *paragraphs += 1;
                    if *paragraphs > 1 {
                        return Err("desktop Markdown list items support one paragraph and nested simple lists".into());
                    }
                }
            }
            Tag::Heading { level, .. } => {
                self.require_outside_list_block()?;
                self.separate(2);
                style.bold = Some(true);
                style.size_scale = Some(match level as u8 {
                    1 => 2.0,
                    2 => 1.5,
                    3 => 1.17,
                    4 => 1.0,
                    5 => 0.83,
                    _ => 0.67,
                });
            }
            Tag::CodeBlock(_) => {
                self.require_outside_list_block()?;
                self.separate(2);
                style.font_family = Some("monospace".into());
            }
            Tag::HtmlBlock => self.require_outside_list_block()?,
            Tag::List(start) => {
                self.separate(if self.lists.is_empty() { 2 } else { 1 });
                self.lists.push(start);
            }
            Tag::Item => {
                self.item_paragraphs.push(0);
                self.separate(1);
                let depth = self.lists.len();
                let next = self
                    .lists
                    .last_mut()
                    .ok_or("Markdown item outside a list")?;
                let marker = if let Some(number) = next {
                    let marker = format!("{number}. ");
                    *number = number
                        .checked_add(1)
                        .ok_or("Markdown list number overflow")?;
                    marker
                } else {
                    "• ".into()
                };
                self.append(&format!("{}{marker}", "  ".repeat(depth.saturating_sub(1))))?;
            }
            Tag::Emphasis => style.italic = Some(true),
            Tag::Strong => style.bold = Some(true),
            Tag::Strikethrough => style.strikethrough = Some(true),
            Tag::Link {
                dest_url, title, ..
            } => {
                if !title.is_empty() {
                    return Err("desktop rich text link titles are not supported".into());
                }
                validate_link(&dest_url)?;
                if style.link.is_some() {
                    return Err("nested desktop rich text links are not supported".into());
                }
                style.link = Some(dest_url.to_string());
                style.link_id = Some(self.next_link_id()?);
            }
            _ => return Err(format!("unsupported desktop Markdown feature: {end:?}")),
        }
        self.push(Scope::Markdown(end), style)
    }

    fn require_outside_list_block(&self) -> Result<(), String> {
        // The portable representation has scalar text runs, not paragraph
        // containers or hanging indents. Keep simple/nested lists, but reject
        // blocks whose spacing would otherwise separate a marker from its
        // content or discard a paragraph's indentation.
        if !self.item_paragraphs.is_empty() {
            return Err("desktop Markdown headings, code blocks and HTML blocks inside list items are unsupported".into());
        }
        Ok(())
    }

    fn end_markdown(&mut self, end: TagEnd) -> Result<(), String> {
        self.pop(Scope::Markdown(end))?;
        match end {
            TagEnd::Paragraph => self.separate(if self.lists.is_empty() { 2 } else { 1 }),
            TagEnd::Heading(_) | TagEnd::CodeBlock | TagEnd::HtmlBlock => self.separate(2),
            TagEnd::Item => {
                self.item_paragraphs.pop();
                self.separate(1);
            }
            TagEnd::List(_) => {
                self.lists.pop();
                self.separate(if self.lists.is_empty() { 2 } else { 1 });
            }
            _ => {}
        }
        Ok(())
    }

    fn html(&mut self, html: &str) -> Result<(), String> {
        let mut reader = Reader::from_str(html);
        // pulldown emits inline opening/closing tags as separate events. We
        // validate matching scopes ourselves across those fragment boundaries.
        reader.config_mut().allow_unmatched_ends = true;
        reader.config_mut().check_end_names = false;
        loop {
            match reader
                .read_event()
                .map_err(|error| format!("invalid desktop rich text HTML: {error}"))?
            {
                event @ (XmlEvent::Start(_) | XmlEvent::Empty(_)) => {
                    let empty = matches!(event, XmlEvent::Empty(_));
                    let start = match event {
                        XmlEvent::Start(start) | XmlEvent::Empty(start) => start,
                        _ => unreachable!(),
                    };
                    let name = start.name().as_ref().to_ascii_lowercase();
                    let mut attributes = Vec::new();
                    for attribute in start.attributes() {
                        let attribute =
                            attribute.map_err(|e| format!("invalid HTML attribute: {e}"))?;
                        let key = attribute.key.as_ref().to_ascii_lowercase();
                        let raw = attribute.value.as_ref();
                        attributes.push((key, decode_entities(raw)?));
                    }
                    self.start_html(&name, &attributes)?;
                    if empty && name != "br" {
                        self.end_html(&name)?;
                    }
                }
                XmlEvent::End(end) => {
                    let name = end.name().as_ref().to_ascii_lowercase();
                    self.end_html(&name)?;
                }
                XmlEvent::Text(text) => {
                    self.append(&text.xml10_content())?;
                }
                XmlEvent::GeneralRef(reference) => {
                    let name = reference.xml10_content();
                    self.append(&decode_entities(&format!("&{name};"))?)?;
                }
                XmlEvent::Eof => break,
                _ => {
                    return Err(
                        "unsupported desktop rich text HTML declaration/comment/CDATA".into(),
                    )
                }
            }
        }
        Ok(())
    }

    fn start_html(&mut self, name: &str, attributes: &[(String, String)]) -> Result<(), String> {
        if self.pre_closed {
            return Err("desktop rich text pre must be the entire document".into());
        }
        let mut style = self.style();
        match name {
            "span" => {}
            "b" | "strong" => style.bold = Some(true),
            "i" | "em" => style.italic = Some(true),
            "u" => style.underline = Some(true),
            "s" | "del" => style.strikethrough = Some(true),
            "code" => style.font_family = Some("monospace".into()),
            "a" => {
                if style.link.is_some() {
                    return Err("nested desktop rich text links are not supported".into());
                }
            }
            "pre" => {
                if self.pre_seen
                    || !self.document.text.trim().is_empty()
                    || self
                        .stack
                        .iter()
                        .any(|(scope, _)| !matches!(scope, Scope::Markdown(TagEnd::HtmlBlock)))
                {
                    return Err("desktop rich text pre must be the entire document".into());
                }
                self.pre_seen = true;
            }
            "br" => {}
            _ => return Err(format!("unsupported desktop rich text HTML tag: {name}")),
        }
        let mut seen_attributes = std::collections::HashSet::new();
        for (key, value) in attributes {
            if !seen_attributes.insert(key) {
                return Err(format!("duplicate desktop rich text HTML attribute: {key}"));
            }
            match (key.as_str(), name) {
                ("style", "span" | "pre") => self.apply_css(name, value, &mut style)?,
                ("data-pax-code-markup", "pre") if value == "example-host" => {}
                ("href", "a") => {
                    validate_link(value)?;
                    style.link = Some(value.clone());
                }
                _ => {
                    return Err(format!(
                        "unsupported desktop rich text HTML attribute: {name}.{key}"
                    ))
                }
            }
        }
        if name == "a" && style.link.is_none() {
            return Err("desktop rich text anchor requires href".into());
        }
        if name == "a" {
            style.link_id = Some(self.next_link_id()?);
        }
        if name == "br" {
            self.append("\n")
        } else {
            self.push(Scope::Html(name.into()), style)
        }
    }

    fn end_html(&mut self, name: &str) -> Result<(), String> {
        self.pop(Scope::Html(name.into()))?;
        if name == "pre" {
            self.pre_closed = true;
        }
        Ok(())
    }

    fn apply_css(&mut self, tag: &str, css: &str, style: &mut Style) -> Result<(), String> {
        for declaration in css
            .split(';')
            .filter(|declaration| !declaration.trim().is_empty())
        {
            let (name, value) = declaration
                .split_once(':')
                .ok_or("invalid desktop rich text CSS declaration")?;
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim();
            match (name.as_str(), tag) {
                ("color", _) => style.color = Some(parse_color(value)?),
                ("margin", "pre") if value == "0" || value == "0px" => {}
                ("white-space", "pre") if value == "pre" => {}
                ("line-height", "pre") => {
                    let height: f64 = value
                        .strip_suffix("px")
                        .ok_or("desktop rich text line-height requires px")?
                        .parse()
                        .map_err(|_| "invalid desktop rich text line-height")?;
                    if !height.is_finite() || height <= 0.0 || height > 4096.0 {
                        return Err("desktop rich text line-height must be in (0, 4096]px".into());
                    }
                    self.document.line_height = Some(height);
                }
                _ => return Err(format!("unsupported desktop rich text CSS: {tag}.{name}")),
            }
        }
        Ok(())
    }
}

fn parse_color(value: &str) -> Result<[f64; 4], String> {
    let hex = value
        .strip_prefix('#')
        .ok_or("desktop rich text color requires #RGB or #RRGGBB")?;
    if !hex.is_ascii() || !matches!(hex.len(), 3 | 6) {
        return Err("desktop rich text color requires opaque #RGB or #RRGGBB".into());
    }
    let mut rgba = [0.0, 0.0, 0.0, 1.0];
    for (index, channel) in rgba[..3].iter_mut().enumerate() {
        let text = if hex.len() == 3 {
            hex[index..index + 1].repeat(2)
        } else {
            hex[index * 2..index * 2 + 2].into()
        };
        *channel = u8::from_str_radix(&text, 16).map_err(|_| "invalid desktop rich text color")?
            as f64
            / 255.0;
    }
    Ok(rgba)
}

fn decode_entities(value: &str) -> Result<String, String> {
    let mut rest = value;
    let mut decoded = String::new();
    while let Some(index) = rest.find('&') {
        decoded.push_str(&rest[..index]);
        rest = &rest[index..];
        let end = rest
            .find(';')
            .ok_or("unterminated desktop rich text HTML entity")?;
        let entity = &rest[..=end];
        let replacement = html_escape::decode_html_entities(entity);
        if replacement == entity || replacement.contains('\0') {
            return Err(format!("invalid desktop rich text HTML entity: {entity}"));
        }
        decoded.push_str(&replacement);
        rest = &rest[end + 1..];
    }
    decoded.push_str(rest);
    Ok(decoded)
}

fn validate_link(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
        || value.contains('\\')
        || value.starts_with("//")
    {
        return Err("invalid desktop rich text link destination".into());
    }
    let first_segment = value.split(['/', '?', '#']).next().unwrap_or_default();
    if let Some((scheme, _)) = first_segment.split_once(':') {
        if !matches!(
            scheme.to_ascii_lowercase().as_str(),
            "http" | "https" | "mailto"
        ) {
            return Err(format!(
                "unsupported desktop rich text link scheme: {scheme}"
            ));
        }
    }
    Ok(())
}

fn normalize(source: &str) -> Result<Document, String> {
    let mut builder = Builder::default();
    // Parse these extensions explicitly so unsupported structures report an
    // error instead of silently losing table/task/footnote semantics.
    let options = Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;
    for event in Parser::new_ext(source, options) {
        match event {
            Event::Start(tag) => builder.start_markdown(tag)?,
            Event::End(end) => builder.end_markdown(end)?,
            Event::Text(text) => builder.append(&text)?,
            Event::Code(text) => {
                let mut style = builder.style();
                style.font_family = Some("monospace".into());
                builder.push(Scope::Html("inline-code".into()), style)?;
                builder.append(&text)?;
                builder.pop(Scope::Html("inline-code".into()))?;
            }
            Event::Html(html) | Event::InlineHtml(html) => builder.html(&html)?,
            Event::SoftBreak => builder.append(" ")?,
            Event::HardBreak => builder.append("\n")?,
            unsupported => {
                return Err(format!(
                    "unsupported desktop Markdown event: {unsupported:?}"
                ))
            }
        }
    }
    if !builder.stack.is_empty() {
        return Err("unclosed desktop rich text HTML element".into());
    }
    if builder.document.text.contains('\0') {
        return Err("desktop rich text cannot contain NUL".into());
    }
    Ok(builder.document)
}

pub(super) fn enrich_native_messages(messages: &mut Value) -> Result<(), String> {
    for message in messages
        .as_array_mut()
        .ok_or("native messages must be an array")?
    {
        let Some(patch) = message.get_mut("TextUpdate") else {
            continue;
        };
        let content = patch.get("content").and_then(Value::as_str);
        let markdown = patch.get("markdown").and_then(Value::as_bool);
        if content.is_none() && markdown.is_none() {
            continue;
        }
        let (Some(content), Some(markdown)) = (content, markdown) else {
            return Err("desktop TextUpdate content and markdown must be sent together".into());
        };
        let rich_text = if markdown {
            serde_json::to_value(normalize(content).map_err(|error| {
                format!(
                    "TextUpdate {}: {error}",
                    patch.get("id").unwrap_or(&Value::Null)
                )
            })?)
            .map_err(|error| error.to_string())?
        } else {
            Value::Null
        };
        patch
            .as_object_mut()
            .ok_or("TextUpdate must be an object")?
            .insert("rich_text".into(), rich_text);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run_at(document: &Document, needle: &str) -> Style {
        let offset = document.text.find(needle).unwrap();
        document
            .runs
            .iter()
            .find(|run| run.start <= offset && run.end > offset)
            .unwrap()
            .style
            .clone()
    }

    fn assert_coverage(document: &Document) {
        let mut end = 0;
        for run in &document.runs {
            assert_eq!(run.start, end);
            assert!(run.start < run.end);
            assert!(document.text.is_char_boundary(run.start));
            assert!(document.text.is_char_boundary(run.end));
            end = run.end;
        }
        assert_eq!(end, document.text.len());
    }

    #[test]
    fn markdown_inline_unicode_and_nested_restoration() {
        let document =
            normalize("é **bold _😀 italic_ after** `a < b` ~~gone~~ [route](/hello?q=1#x)")
                .unwrap();
        assert_eq!(document.text, "é bold 😀 italic after a < b gone route");
        assert_eq!(run_at(&document, "😀").bold, Some(true));
        assert_eq!(run_at(&document, "😀").italic, Some(true));
        assert_eq!(run_at(&document, "after").italic, None);
        assert_eq!(
            run_at(&document, "a <").font_family.as_deref(),
            Some("monospace")
        );
        assert_eq!(run_at(&document, "gone").strikethrough, Some(true));
        assert_eq!(
            run_at(&document, "route").link.as_deref(),
            Some("/hello?q=1#x")
        );
        assert_coverage(&document);
    }

    #[test]
    fn block_structure_and_code_whitespace() {
        let document = normalize("# Title\n\nparagraph\nsoft  \nhard\n\n3. first\n4. second\n   - nested\n\n```rust\n  let x = 1;\n\n```\n").unwrap();
        assert_eq!(
            document.text,
            "Title\n\nparagraph soft\nhard\n\n3. first\n4. second\n  • nested\n\n  let x = 1;\n\n"
        );
        assert_eq!(run_at(&document, "Title").size_scale, Some(2.0));
        assert_eq!(
            run_at(&document, "let").font_family.as_deref(),
            Some("monospace")
        );
        assert_coverage(&document);
    }

    #[test]
    fn simple_lists_work_but_nested_block_layout_is_rejected() {
        let simple = normalize("- first\n  - **child**\n- second").unwrap();
        assert_eq!(simple.text, "• first\n  • child\n• second");
        assert_eq!(run_at(&simple, "child").bold, Some(true));
        assert_coverage(&simple);
        // CommonMark marks these list items as loose, but each has only one
        // paragraph; no additional block layout needs to be represented.
        let one_paragraph_each = normalize("1. first\n\n2. second").unwrap();
        assert_eq!(one_paragraph_each.text, "1. first\n2. second");
        for source in [
            "- # Title\n- next",
            "- paragraph one\n\n  paragraph two",
            "- outer\n  - child one\n\n    child two",
            "- first\n  - child\n\n  continuation",
            "- ```rust\n  let x = 1;\n  ```",
            "- item\n\n      indented code",
            "- <span>\n  block\n  </span>",
        ] {
            assert!(
                normalize(source).is_err(),
                "accepted unsupported list block: {source}"
            );
        }
    }

    #[test]
    fn generated_example_host_pre_preserves_source_and_colors() {
        // Captured from example_host.rs's actual highlighted_code_markup for
        // `let café = "😀 < & '";\n\n` using its Rust source highlighter.
        let cases: Value = serde_json::from_str(include_str!("rich-text-cases.json")).unwrap();
        let document = normalize(cases[0]["content"].as_str().unwrap()).unwrap();
        assert_eq!(document.text, "let café = \"😀 < & '\";\n\n");
        assert_eq!(document.line_height, Some(18.0));
        assert_eq!(
            run_at(&document, "let").color,
            Some(parse_color("#8c9eff").unwrap())
        );
        assert_eq!(
            run_at(&document, "café").color,
            Some(parse_color("#f5f5f5").unwrap())
        );
        assert_eq!(run_at(&document, "café").font_family, None);
        assert_coverage(&document);
    }

    #[test]
    fn html_nested_spans_restore_parent_and_decode_entities() {
        let document = normalize("<span style=\"color:#f00\">red <span style=\"color:#00f\"><b>blue</b></span> red</span> plain &amp; é").unwrap();
        assert_eq!(document.text, "red blue red plain & é");
        assert_eq!(run_at(&document, "blue").color, Some([0.0, 0.0, 1.0, 1.0]));
        assert_eq!(run_at(&document, "blue").bold, Some(true));
        assert_eq!(
            document
                .runs
                .iter()
                .find(|run| &document.text[run.start..run.end] == " red")
                .unwrap()
                .style
                .color,
            Some([1.0, 0.0, 0.0, 1.0])
        );
        assert_eq!(run_at(&document, "plain").color, None);
        assert_coverage(&document);
    }

    #[test]
    fn html_hyperlinks_and_breaks() {
        let document =
            normalize("<a href=\"https://example.org/?a=1&amp;b=2\"><u>open</u></a><br/>next")
                .unwrap();
        assert_eq!(document.text, "open\nnext");
        assert_eq!(
            run_at(&document, "open").link.as_deref(),
            Some("https://example.org/?a=1&b=2")
        );
        assert_eq!(run_at(&document, "open").underline, Some(true));
        assert_eq!(run_at(&document, "next").link, None);
    }

    #[test]
    fn link_identity_tracks_authored_scope_across_formatting_and_html() {
        let adjacent = normalize("[one](/x)[two](/x)").unwrap();
        assert_eq!(adjacent.runs.len(), 2);
        assert_eq!(run_at(&adjacent, "one").link_id, Some(1));
        assert_eq!(run_at(&adjacent, "two").link_id, Some(2));

        let styled = normalize("[one **two** three](/x)").unwrap();
        assert_eq!(styled.runs.len(), 3);
        assert!(styled.runs.iter().all(|run| run.style.link_id == Some(1)));

        let mixed = normalize("<a href=\"/x\">one <b>two</b> three</a>[four](/x)<a href=\"/x\"><span style=\"color:#f00\">five</span></a>").unwrap();
        for needle in ["one", "two", "three"] {
            assert_eq!(run_at(&mixed, needle).link_id, Some(1));
        }
        assert_eq!(run_at(&mixed, "four").link_id, Some(2));
        assert_eq!(run_at(&mixed, "five").link_id, Some(3));
        assert_coverage(&mixed);
        // Identities are document local, including after a new content patch.
        assert_eq!(
            run_at(&normalize("[new](/x)").unwrap(), "new").link_id,
            Some(1)
        );
    }

    #[test]
    fn invalid_or_unsupported_markup_is_never_silently_plain() {
        for source in [
            "<span>missing",
            "<span><b>x</span></b>",
            "<blink>x</blink>",
            "<span style=\"background:red\">x</span>",
            "<span style=\"color:#1234\">x</span>",
            "<span onclick=\"run()\">x</span>",
            "<pre>&unknown;</pre>",
            "<pre>&#0;</pre>",
            "<pre>&amp</pre>",
            "<pre>x</pre>tail",
            "<pre>x</pre><pre>y</pre>",
            "<pre>x</pre><span></span>",
            "<span style=\"color:#f00\" STYLE=\"color:#00f\">x</span>",
            "<pre style=\"line-height:NaNpx\">x</pre>",
            "<pre style=\"line-height:0px\">x</pre>",
            "<pre style=\"white-space:normal\">x</pre>",
            "<pre>&#xD800;</pre>",
            "<pre>&#x110000;</pre>",
            "<span><a href=\"/x\"><a href=\"/y\">x</a></a></span>",
            "<!-- discarded comment -->",
            "![image](image.png)",
            "> quote",
            "[bad](javascript:alert)",
            "[bad](file:///tmp/a)",
            "[title](https://example.org \"tooltip\")",
            "- [x] task",
            "| a | b |\n| - | - |\n| c | d |",
            "<script>x</script>",
            "<span style=\"color:#zzzzzz\">x</span>",
        ] {
            assert!(
                normalize(source).is_err(),
                "accepted unsupported source: {source}"
            );
        }
    }

    #[test]
    fn shared_native_host_fixtures_match_serialized_contract() {
        let cases: Value = serde_json::from_str(include_str!("rich-text-cases.json")).unwrap();
        for case in cases.as_array().unwrap() {
            let mut messages = json!([{"TextUpdate": {
                "id": 9, "content": case["content"], "markdown": case["markdown"]
            }}]);
            enrich_native_messages(&mut messages).unwrap();
            assert_eq!(
                messages[0]["TextUpdate"]["rich_text"], case["rich_text"],
                "{}",
                case["name"]
            );
        }
    }

    #[test]
    fn content_patch_enriches_style_only_does_not_and_plain_toggle_clears() {
        let mut messages = json!([
            {"TextUpdate":{"id":1,"content":"**hello**","markdown":true}},
            {"TextUpdate":{"id":1,"content":null,"markdown":null,"style":{"font_size":24}}},
            {"TextUpdate":{"id":1,"content":"**hello**","markdown":false}},
            {"ButtonUpdate":{"id":1,"content":"**button**"}}
        ]);
        enrich_native_messages(&mut messages).unwrap();
        assert_eq!(messages[0]["TextUpdate"]["rich_text"]["text"], "hello");
        assert!(messages[1]["TextUpdate"].get("rich_text").is_none());
        assert_eq!(messages[2]["TextUpdate"]["rich_text"], Value::Null);
        assert!(messages[2]["TextUpdate"].get("rich_text").is_some());
        assert!(messages[3]["ButtonUpdate"].get("rich_text").is_none());
        let mut unpaired = json!([{"TextUpdate":{"id":1,"content":"hello"}}]);
        assert!(enrich_native_messages(&mut unpaired).is_err());
    }

    #[test]
    fn links_reject_active_and_ambiguous_schemes() {
        for good in [
            "/about",
            "../docs",
            "#anchor",
            "?tab=1",
            "https://example.org",
            "HTTP://example.org",
            "mailto:a@example.org",
        ] {
            validate_link(good).unwrap();
        }
        for bad in [
            "",
            "//remote/path",
            "a b",
            "java\nscript:run",
            "data:text/html,x",
            "file:///etc/passwd",
            "pax://route",
            "\\remote",
        ] {
            assert!(validate_link(bad).is_err(), "accepted {bad}");
        }
    }
}
