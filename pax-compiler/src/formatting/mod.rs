mod rules;

use crate::helpers::{replace_by_line_column, InlinedTemplateFinder};
use crate::parsing::{PaxParser, Rule};
use color_eyre::eyre::{self, Report};
use pest::Parser;
use std::fs;
use std::path::Path;
use syn::parse_file;
use syn::visit::Visit;

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

    let mut finder = InlinedTemplateFinder::new(content.clone());
    finder.visit_file(&ast);

    let mut modified_content = content;
    for template in finder.templates {
        let formatted_template = format_pax_template(template.template)?;
        let new_content = format!("(\n{}\n)", formatted_template);
        modified_content =
            replace_by_line_column(&modified_content, template.start, template.end, new_content)
                .unwrap();
    }
    fs::write(path, modified_content)?;
    Ok(())
}
