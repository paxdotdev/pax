//! Android's text projection, independent of native layout and the baked format.
//!
//! Offsets use UTF-16, as Android spans do. The generated source-markup format
//! deliberately has a separate decoder: CommonMark treats its pre block as raw
//! HTML, and HTML text renderers can collapse the source's significant spaces.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde::Serialize;

const CODE_PREFIX: &str = "<pre data-pax-code-markup=\"example-host\" ";
pub(crate) const MAX_SOURCE_BYTES: usize = 1024 * 1024;
const MAX_SPANS: usize = 65_536;
const MAX_DEPTH: usize = 256;

#[derive(Debug, Serialize)]
pub(crate) struct Document {
    text: String,
    spans: Vec<Span>,
    #[serde(skip)]
    offset: usize,
    #[serde(skip)]
    link_bytes: usize,
}

#[derive(Debug, Serialize)]
struct Span {
    start: usize,
    end: usize,
    #[serde(flatten)]
    style: Style,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum Style {
    Bold,
    Italic,
    Strike,
    Code,
    Heading { level: u8 },
    Link { url: String },
    Color { value: String },
    Quote,
    Indent,
}

impl Document {
    fn new() -> Self {
        Self {
            text: String::new(),
            spans: Vec::new(),
            offset: 0,
            link_bytes: 0,
        }
    }
    fn append(&mut self, text: &str) {
        self.offset += text.encode_utf16().count();
        self.text.push_str(text);
    }
    fn breaks(&mut self, count: usize) {
        if !self.text.is_empty() {
            let existing = self.text.bytes().rev().take_while(|b| *b == b'\n').count();
            for _ in existing..count {
                self.append("\n");
            }
        }
    }
    fn span(&mut self, start: usize, style: Style) -> Result<(), String> {
        if self.spans.len() == MAX_SPANS {
            return Err("Android rich text exceeds 65536 spans".into());
        }
        if start < self.offset {
            self.spans.push(Span {
                start,
                end: self.offset,
                style,
            });
        }
        Ok(())
    }
}

pub(crate) fn parse(source: &str) -> Result<Document, String> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err("Android rich text exceeds 1 MiB".into());
    }
    if source.starts_with(CODE_PREFIX) {
        return highlighted_code(source);
    }
    let mut out = Document::new();
    let mut stack = Vec::new();
    let mut lists = Vec::<Option<u64>>::new();
    // Other extensions (tables, footnotes, math) need their own native layout
    // contracts. Leave them as ordinary text rather than silently losing cells.
    for event in Parser::new_ext(source, Options::ENABLE_STRIKETHROUGH) {
        match event {
            Event::Start(tag) => {
                if stack.len() == MAX_DEPTH {
                    return Err("Android Markdown nesting exceeds 256".into());
                }
                let style = match &tag {
                    Tag::Paragraph => {
                        // A loose-list paragraph follows its bullet immediately.
                        if !matches!(stack.last(), Some((TagEnd::Item, _, _))) {
                            out.breaks(2);
                        }
                        None
                    }
                    Tag::Heading { level, .. } => {
                        out.breaks(2);
                        Some(Style::Heading {
                            level: *level as u8,
                        })
                    }
                    Tag::CodeBlock(_) => {
                        out.breaks(2);
                        Some(Style::Code)
                    }
                    Tag::BlockQuote(_) => {
                        out.breaks(2);
                        Some(Style::Quote)
                    }
                    Tag::List(start) => {
                        out.breaks(if lists.is_empty() { 2 } else { 1 });
                        lists.push(*start);
                        None
                    }
                    Tag::Item => {
                        out.breaks(1);
                        Some(Style::Indent)
                    }
                    Tag::Emphasis => Some(Style::Italic),
                    Tag::Strong => Some(Style::Bold),
                    Tag::Strikethrough => Some(Style::Strike),
                    Tag::Link { dest_url, .. } => {
                        // Reference links can reuse one very long destination
                        // thousands of times. Bound the expanded payload before
                        // cloning it into spans, not only the authored source.
                        out.link_bytes += dest_url.len();
                        if out.link_bytes > MAX_SOURCE_BYTES {
                            return Err("Android rich-text link targets exceed 1 MiB".into());
                        }
                        Some(Style::Link {
                            url: dest_url.to_string(),
                        })
                    }
                    _ => None,
                };
                stack.push((tag.to_end(), out.offset, style));
                if tag == Tag::Item {
                    match lists.last_mut() {
                        Some(Some(number)) => {
                            out.append(&format!("{number}. "));
                            *number = number.saturating_add(1);
                        }
                        _ => out.append("• "),
                    }
                }
            }
            Event::End(tag) => {
                let (_, start, style) = stack.pop().ok_or("Unbalanced Markdown events")?;
                if let Some(style) = style {
                    out.span(start, style)?;
                }
                match tag {
                    TagEnd::Paragraph
                    | TagEnd::Heading(_)
                    | TagEnd::CodeBlock
                    | TagEnd::BlockQuote(_) => out.breaks(2),
                    TagEnd::Item => out.breaks(1),
                    TagEnd::List(_) => {
                        lists.pop();
                        out.breaks(if lists.is_empty() { 2 } else { 1 });
                    }
                    _ => {}
                }
            }
            Event::Code(text) => {
                let start = out.offset;
                out.append(&text);
                out.span(start, Style::Code)?;
            }
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => out.append(&text),
            Event::SoftBreak => out.append(" "),
            Event::HardBreak => out.append("\n"),
            Event::Rule => {
                out.breaks(2);
                out.append("―――");
                out.breaks(2);
            }
            _ => {}
        }
    }
    while out.text.ends_with('\n') {
        out.text.pop();
        out.offset -= 1;
    }
    for span in &mut out.spans {
        span.end = span.end.min(out.offset);
    }
    out.spans.retain(|span| span.start < span.end);
    Ok(out)
}

fn highlighted_code(source: &str) -> Result<Document, String> {
    use quick_xml::{events::Event as Xml, Reader};
    let mut reader = Reader::from_str(source);
    let mut out = Document::new();
    let mut stack = Vec::new();
    let mut root = false;
    loop {
        match reader
            .read_event()
            .map_err(|e| format!("Invalid Pax code markup: {e}"))?
        {
            Xml::Start(tag) => {
                let name = tag.name();
                if (stack.is_empty() && (root || name.as_ref() != "pre"))
                    || (!stack.is_empty() && name.as_ref() != "span")
                {
                    return Err("Pax code markup supports one pre element and nested spans".into());
                }
                if stack.len() == MAX_DEPTH {
                    return Err("Pax code markup nesting exceeds 256".into());
                }
                root = true;
                let mut color = None;
                for attribute in tag.attributes() {
                    let attribute = attribute.map_err(|e| e.to_string())?;
                    if attribute.key.as_ref() == "style" {
                        let value = attribute
                            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                            .map_err(|e| e.to_string())?;
                        for declaration in value.split(';') {
                            if let Some((key, value)) = declaration.split_once(':') {
                                let value = value.trim();
                                if key.trim() == "color"
                                    && value.len() == 7
                                    && value.starts_with('#')
                                    && value[1..].bytes().all(|b| b.is_ascii_hexdigit())
                                {
                                    color = Some(Style::Color {
                                        value: value.into(),
                                    });
                                }
                            }
                        }
                    }
                }
                stack.push((out.offset, color));
            }
            Xml::End(_) => {
                let (start, color) = stack.pop().ok_or("Unexpected code-markup closing tag")?;
                // Outer colors must be applied first; nested token colors win.
                if let Some(color) = color {
                    out.span(start, color)?;
                }
            }
            Xml::Text(text) => {
                if stack.is_empty() {
                    return Err("Text outside Pax code-markup root".into());
                }
                out.append(&text.xml10_content());
            }
            Xml::GeneralRef(reference) => {
                if stack.is_empty() {
                    return Err("Entity outside Pax code-markup root".into());
                }
                if let Some(character) = reference.resolve_char_ref().map_err(|e| e.to_string())? {
                    out.append(&character.to_string());
                } else {
                    let entity = reference.into_inner();
                    out.append(match entity.as_ref() {
                        "amp" => "&",
                        "lt" => "<",
                        "gt" => ">",
                        "quot" => "\"",
                        "apos" => "'",
                        _ => return Err(format!("Unknown Pax code-markup entity: {entity}")),
                    });
                }
            }
            Xml::Eof => break,
            _ => return Err("Unsupported Pax code-markup element".into()),
        }
    }
    if !root || !stack.is_empty() {
        return Err("Incomplete Pax code markup".into());
    }
    out.spans.reverse();
    // ExampleHost supplies its chosen code font through TextStyle. Preserve
    // that bundled/remote face instead of replacing it with Android's generic
    // monospace family (which ordinary Markdown code spans use).
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn slice<'a>(doc: &'a Document, span: &Span) -> String {
        String::from_utf16(&doc.text.encode_utf16().collect::<Vec<_>>()[span.start..span.end])
            .unwrap()
    }

    #[test]
    fn commonmark_unicode_nested_styles_links_and_escaped_literals() {
        let doc =
            parse("🦀 **bold *inside*** and `a < b` [go](/next?q=a&b=c) \\*literal\\*").unwrap();
        assert_eq!(doc.text, "🦀 bold inside and a < b go *literal*");
        assert!(doc.spans.iter().any(|s| matches!(s.style, Style::Bold)
            && slice(&doc, s) == "bold inside"
            && s.start == 3));
        assert!(doc
            .spans
            .iter()
            .any(|s| matches!(s.style, Style::Italic) && slice(&doc, s) == "inside"));
        assert!(doc.spans.iter().any(
            |s| matches!(&s.style, Style::Link { url } if url == "/next?q=a&b=c")
                && slice(&doc, s) == "go"
        ));
    }

    #[test]
    fn blocks_lists_code_and_image_alternative_text_are_preserved() {
        let doc = parse("# Title\n\n3. first\n4. second\n\n> quoted\n\n```rust\n  a < b\n```\n\n![description](image.png)").unwrap();
        assert_eq!(
            doc.text,
            "Title\n\n3. first\n4. second\n\nquoted\n\n  a < b\n\ndescription"
        );
        assert!(doc
            .spans
            .iter()
            .any(|s| matches!(s.style, Style::Heading { level: 1 })));
        assert!(doc
            .spans
            .iter()
            .any(|s| matches!(s.style, Style::Code) && slice(&doc, s) == "  a < b\n"));
        assert_eq!(
            doc.spans
                .iter()
                .filter(|s| matches!(s.style, Style::Indent))
                .count(),
            2
        );
    }

    #[test]
    fn source_markup_preserves_whitespace_entities_and_color_precedence() {
        let doc = parse(concat!("<pre data-pax-code-markup=\"example-host\" style=\"color:#eeeeee;\">",
            "  &lt;Text text=&quot;🦀 &amp;lt;&quot;/&gt;\n",
            "<span style=\"color:#112233;\">outer <span style=\"color:#445566;\">&#x1f980;</span></span>\n</pre>")).unwrap();
        assert_eq!(doc.text, "  <Text text=\"🦀 &lt;\"/>\nouter 🦀\n");
        let colors: Vec<_> = doc
            .spans
            .iter()
            .filter_map(|s| {
                if let Style::Color { value } = &s.style {
                    Some(value.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(colors, ["#eeeeee", "#112233", "#445566"]);
        assert!(doc.spans.iter().any(
            |s| matches!(&s.style, Style::Color { value } if value == "#445566")
                && slice(&doc, s) == "🦀"
        ));
    }

    #[test]
    fn reference_links_bound_expanded_destinations() {
        let doc = parse("[one][r] [two][r]\n\n[r]: /destination").unwrap();
        assert_eq!(doc.text, "one two");
        assert_eq!(doc.link_bytes, 2 * "/destination".len());
        let source = format!(
            "[one][r] [two][r] [three][r]\n\n[r]: /{}",
            "x".repeat(MAX_SOURCE_BYTES / 2)
        );
        assert!(source.len() < MAX_SOURCE_BYTES);
        assert_eq!(
            parse(&source).unwrap_err(),
            "Android rich-text link targets exceed 1 MiB"
        );
    }

    #[test]
    fn malformed_generated_markup_is_reported_and_other_html_is_literal() {
        assert!(parse("<pre data-pax-code-markup=\"example-host\" ><span>x</pre>").is_err());
        assert!(parse("<pre data-pax-code-markup=\"example-host\" >&unknown;</pre>").is_err());
        assert_eq!(
            parse("<script>example</script>").unwrap().text,
            "<script>example</script>"
        );
        assert!(parse(&"x".repeat(MAX_SOURCE_BYTES + 1)).is_err());
    }
}
