mod rules;

use crate::parsing::{PaxParser, Rule};
use color_eyre::eyre::{self, Report};
use pest::Parser;
use std::fs;
use std::path::Path;
use syn::visit::Visit;
use syn::{parse_file, Attribute, Lit, Meta, NestedMeta};

pub fn format_pax_template(code: String) -> Result<String, eyre::Report> {
    let pax_component_definition = PaxParser::parse(Rule::pax_component_definition, code.as_str())?
        .next()
        .unwrap();
    let formatted_code = rules::apply_formatting_rules(pax_component_definition);
    Ok(formatted_code)
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
        if let Ok(formatted_template) = format_pax_template(template.clone()) {
            modified_content = modified_content.replace(&template, &formatted_template);
        }
    }

    fs::write(path, modified_content)?;
    Ok(())
}

struct InlinedTemplateFinder {
    templates: Vec<String>,
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
            if let Ok(Meta::List(meta_list)) = i.parse_meta() {
                for nested_meta in meta_list.nested {
                    if let NestedMeta::Lit(Lit::Str(lit_str)) = nested_meta {
                        self.templates.push(lit_str.value());
                    }
                }
            }
        }
    }
}
