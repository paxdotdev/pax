mod rules;

use crate::parsing::{PaxParser, Rule};
use color_eyre::eyre::{self, Report};
use pest::Parser;
use std::fs;
use std::path::Path;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{parse_file, Attribute};

pub fn format_pax_template(code: String) -> Result<String, eyre::Report> {
    let pax_component_definition = PaxParser::parse(Rule::pax_component_definition, code.as_str())?
        .next()
        .unwrap();
    Ok(rules::format(pax_component_definition))
}

pub fn format_file(file_path: &str) -> Result<(), Report> {
    let path = Path::new(file_path);

    match path.extension().and_then(|s| s.to_str()) {
        Some("pax") => format_pax_file(path),
        Some("rs") => format_pax_in_rust_file(path),
        _ => Err(Report::msg("Unsupported file extension")),
    }
}

fn format_pax_file(path: &Path) -> Result<(), Report> {
    let content = fs::read_to_string(path)?;
    match format_pax_template(content) {
        Ok(formatted_content) => {
            fs::write(path, formatted_content)?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn format_pax_in_rust_file(path: &Path) -> Result<(), Report> {
    let content = fs::read_to_string(path)?;
    let ast = parse_file(&content)?;

    let mut finder = InlinedTemplateFinder::new();
    finder.visit_file(&ast);

    let mut modified_content = content;
    for template in finder.templates {
        let formatted_template = format_pax_template(template.template)?;
        let new_content = format!("(\n{}\n)", formatted_template);
        modified_content =
            replace_by_line_column(&modified_content, template.start, template.end, new_content);
    }
    fs::write(path, modified_content)?;
    Ok(())
}

#[derive(Debug)]
struct InlinedTemplate {
    start: (usize, usize),
    end: (usize, usize),
    template: String,
}

#[derive(Debug)]
struct InlinedTemplateFinder {
    templates: Vec<InlinedTemplate>,
}

impl InlinedTemplateFinder {
    fn new() -> Self {
        InlinedTemplateFinder {
            templates: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for InlinedTemplateFinder {
    fn visit_attribute(&mut self, i: &'ast Attribute) {
        if i.path.is_ident("inlined") {
            let content = i
                .tokens
                .to_string()
                .trim_start_matches("(")
                .trim_end_matches(")")
                .to_string();
            let start = i.tokens.span().start();
            let end = i.tokens.span().end();

            let inlined_template = InlinedTemplate {
                start: (start.line, start.column + 1),
                end: (end.line, end.column + 1),
                template: content,
            };

            self.templates.insert(0, inlined_template);
        }
    }
}

fn replace_by_line_column(
    input: &str,
    start: (usize, usize),
    end: (usize, usize),
    replacement: String,
) -> String {
    let mut result = String::new();
    let mut current_line = 1;
    let mut current_column = 1;
    let mut start_byte = None;
    let mut end_byte = None;

    for (i, c) in input.char_indices() {
        if current_line == start.0 && current_column == start.1 {
            start_byte = Some(i);
        }
        if current_line == end.0 && current_column == end.1 {
            end_byte = Some(i);
            break;
        }

        if c == '\n' {
            current_line += 1;
            current_column = 1;
        } else {
            current_column += 1;
        }
    }

    if let (Some(start_byte), Some(end_byte)) = (start_byte, end_byte) {
        result.push_str(&input[..start_byte]);
        result.push_str(&replacement);
        result.push_str(&input[end_byte..]);
    } else {
        unreachable!("Failed to find start/end bytes");
    }

    result
}
