use colored::Colorize;
use core::panic;
use serde_derive::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

use similar::{ChangeTag, TextDiff};
use syn::{parse_file, spanned::Spanned, visit::Visit, Item};
use tera::{Context, Tera};

use include_dir::{include_dir, Dir};

use crate::{pax_runtime_api::PaxValue, ComponentDefinition, ExpressionInfo, PaxManifest, PaxType};
use pax_lang::{
    formatting::{format_file, format_pax_template},
    helpers::{replace_by_line_column, InlinedTemplateFinder},
};

#[allow(unused)]
static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/code_serialization");
#[allow(unused)]
static MANIFEST_CODE_SERIALIZATION_TEMPLATE: &str = "manifest-code-serialization.tera";
#[allow(unused)]
static MACROS_TEMPLATE: &str = "macros.tera";
#[allow(unused)]
static RUST_FILE_SERIALIZATION_TEMPLATE: &str = "rust-file-serialization.tera";

fn to_pax_value(args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    match args.get("value") {
        Some(val) => {
            let value: Result<PaxValue, serde_json::Error> = serde_json::from_value(val.clone());
            if let Ok(value) = value {
                return Ok(tera::Value::String(value.to_string()));
            }
            Err(tera::Error::msg("Failed to deserialize value to PaxValue"))
        }
        None => Err(tera::Error::msg(
            "No value provided to to_pax_value function",
        )),
    }
}

fn to_pax_expression(args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    match args.get("value") {
        Some(val) => {
            let value: Result<ExpressionInfo, serde_json::Error> =
                serde_json::from_value(val.clone());
            if let Ok(value) = value {
                return Ok(tera::Value::String(value.expression.to_string()));
            }
            Err(tera::Error::msg(format!(
                "Failed to deserialize value to PaxExpression: {:?}",
                val
            )))
        }
        None => Err(tera::Error::msg(
            "No value provided to to_pax_expression function",
        )),
    }
}

/// Serialize a component to a string
pub fn press_code_serialization_template(args: ComponentDefinition) -> Result<String, String> {
    let mut tera = Tera::default();

    tera.register_function("to_pax_value", to_pax_value);
    tera.register_function("to_pax_expression", to_pax_expression);

    // Add macros template
    let macros_file = TEMPLATE_DIR
        .get_file(MACROS_TEMPLATE)
        .ok_or_else(|| format!("Failed to get template file: {}", MACROS_TEMPLATE))?;

    let macros_template_contents = macros_file
        .contents_utf8()
        .ok_or_else(|| format!("Failed to read template contents: {}", MACROS_TEMPLATE))?;

    tera.add_raw_template(MACROS_TEMPLATE, macros_template_contents)
        .map_err(|err| format!("Failed to add template '{}': {}", MACROS_TEMPLATE, err))?;

    // Add manifest code serialization template
    let manifest_file = TEMPLATE_DIR
        .get_file(MANIFEST_CODE_SERIALIZATION_TEMPLATE)
        .ok_or_else(|| {
            format!(
                "Failed to get template file: {}",
                MANIFEST_CODE_SERIALIZATION_TEMPLATE
            )
        })?;

    let manifest_template_contents = manifest_file.contents_utf8().ok_or_else(|| {
        format!(
            "Failed to read template contents: {}",
            MANIFEST_CODE_SERIALIZATION_TEMPLATE
        )
    })?;

    tera.add_raw_template(
        MANIFEST_CODE_SERIALIZATION_TEMPLATE,
        manifest_template_contents,
    )
    .map_err(|err| {
        format!(
            "Failed to add template '{}': {}",
            MANIFEST_CODE_SERIALIZATION_TEMPLATE, err
        )
    })?;

    // Serialize context
    let context = tera::Context::from_serialize(args)
        .map_err(|err| format!("Failed to serialize context: {}", err))?;

    // Render template
    let template = tera
        .render(MANIFEST_CODE_SERIALIZATION_TEMPLATE, &context)
        .map_err(|err| format!("Failed to render template: {}", err))?;

    eprintln!("TEMPLATE: {template}");
    // Format template
    let formatted_template = format_pax_template(template)
        .map_err(|err| format!("Failed to format template: {}", err))?;

    Ok(formatted_template)
}

pub fn diff(old_content: &str, new_content: &str) -> Option<String> {
    let diff = TextDiff::from_lines(old_content, new_content);
    let mut all_diffs = vec![];
    for change in diff.iter_all_changes() {
        let output = match change.tag() {
            ChangeTag::Delete => Some(format!("-{}", change).red()),
            ChangeTag::Insert => Some(format!("+{}", change).green()),
            ChangeTag::Equal => None,
        };

        if let Some(o) = output {
            all_diffs.push(format!("{}", o));
        }
    }
    if all_diffs.len() > 0 {
        Some(all_diffs.join(""))
    } else {
        None
    }
}

pub fn diff_html(old_content: &str, new_content: &str) -> Option<String> {
    let diff = TextDiff::from_lines(old_content, new_content);
    let mut all_diffs = vec![];

    for change in diff.iter_all_changes() {
        //handle edge-cases where the change is empty
        if change.to_string().trim() == "" {
            continue;
        }

        let output = match change.tag() {
            ChangeTag::Delete => Some(format!(
                "<br/><span style=\"color: red;\">---<code>{}</code></span>",
                html_escape::encode_text(&change.to_string())
            )),
            ChangeTag::Insert => Some(format!(
                "<span style=\"color: green;\">+++<code>{}</code></span>",
                html_escape::encode_text(&change.to_string())
            )),
            ChangeTag::Equal => None,
        };

        if let Some(o) = output {
            all_diffs.push(o);
        }
    }

    if !all_diffs.is_empty() {
        Some(all_diffs.join(""))
    } else {
        None
    }
}

/// Serialize a component to a file
/// Replaces entire .pax file and replaces inlined attribute directly for .rs files
pub fn serialize_component_to_file(component: &ComponentDefinition, file_path: String) {
    let path = Path::new(&file_path);
    let pascal_identifier = component.type_id.get_pascal_identifier().unwrap();
    let serialized_component = press_code_serialization_template(component.clone()).unwrap();

    // adds rust file if we're serializing a new component
    serialize_new_component_rust_file(component, file_path.clone());

    match path.extension().and_then(|s| s.to_str()) {
        Some("pax") => {
            let old_content = fs::read_to_string(&file_path).unwrap_or_default();
            let mut file = File::create(&file_path).expect("Failed to create file");
            file.write_all(serialized_component.as_bytes())
                .expect("Failed to write to file");

            // Print the diff
            if let Some(diff) = diff(&old_content, &serialized_component) {
                println!("{}", diff);
            }
        }
        Some("rs") => write_inlined_pax(serialized_component, path, pascal_identifier),
        _ => panic!("Unsupported file extension."),
    }
}

pub fn serialize_main_component(manifest: &PaxManifest, repo_root: &str) {
    let mc = manifest.components.get(&manifest.main_component_type_id);
    if let Some(mc) = mc {
        if let Some(template) = &mc.template {
            if let Some(file_path) = &template.get_file_path() {
                let suffix = file_path.split_once("www.pax.dev/").unwrap().1;
                let file_path = format!("{}/{}", repo_root, suffix);
                serialize_component_to_file(mc, file_path.clone());
            }
        }
    }
}

pub fn serialize_main_component_to_string(manifest: &PaxManifest) -> String {
    let mc = manifest.components.get(&manifest.main_component_type_id);
    if let Some(mc) = mc {
        if let Some(_) = &mc.template {
            return press_code_serialization_template(mc.clone()).unwrap();
        }
    }
    "".to_string()
}

fn write_inlined_pax(serialized_component: String, path: &Path, pascal_identifier: String) {
    let content = fs::read_to_string(path).expect("Failed to read file");
    let ast = parse_file(&content).expect("Failed to parse file");
    let mut finder = InlinedTemplateFinder::new(content.clone());
    finder.visit_file(&ast);

    let template = finder
        .templates
        .iter()
        .find(|t| t.struct_name == pascal_identifier);

    if let Some(data) = template {
        let new_template = format!("(\n{}\n)", serialized_component);
        let modified_content =
            replace_by_line_column(&content, data.start, data.end, new_template).unwrap();
        fs::write(path, modified_content).expect("Failed to write to file");
    }
}

pub fn serialize_new_component_rust_file(comp_def: &ComponentDefinition, pax_file_path: String) {
    if let PaxType::BlankComponent { pascal_identifier } = comp_def.type_id.get_pax_type() {
        let path = PathBuf::from(&pax_file_path);
        let pax_file_name = path.file_name().unwrap().to_str().unwrap();
        let src = path.parent().unwrap();
        let entry_point = src.join("lib.rs");
        let rust_file_path = pax_file_path.replace(".pax", ".rs");

        let rust_file_serialization = RustFileSerialization {
            pax_path: pax_file_name.to_string(),
            pascal_identifier: pascal_identifier.clone(),
        };
        let rust_file_serialization =
            press_rust_file_serialization_template(rust_file_serialization);
        fs::write(rust_file_path, rust_file_serialization).expect("Failed to write to file");
        add_mod_and_use_if_missing(
            Path::new(&entry_point),
            pascal_identifier,
            &pax_file_name.replace(".pax", ""),
        )
        .expect("Failed to add mod and use");
    }
}

/// Adds mod and use for newly created componen
fn add_mod_and_use_if_missing(
    file_name: &Path,
    pascal_identifier: &str,
    rust_file_name: &str,
) -> io::Result<()> {
    let file_content = fs::read_to_string(file_name)?;
    let syntax_tree = parse_file(&file_content).expect("Failed to parse file");

    if file_content.contains(&format!("pub mod {};", rust_file_name))
        || file_content.contains(&format!("use {}::{};", rust_file_name, pascal_identifier))
    {
        // Lines already present, no need to add them again.
        return Ok(());
    }

    // Initialize with the full content; this might be replaced based on finding the last use statement.
    let mut new_content = file_content.clone();

    // Track the last position where a `use` statement ended.
    let mut last_use_end_pos = None;

    for item in syntax_tree.items {
        if let Item::Use(item_use) = item {
            last_use_end_pos = Some(item_use.span().end());
        }
    }

    let insertion_content = format!(
        "pub mod {};\nuse {}::{};",
        rust_file_name, rust_file_name, pascal_identifier
    );

    // Insert the mod and use statements after the last use statement if found, or prepend if no use statements.
    match last_use_end_pos {
        Some(pos) => {
            insert_at_line(&mut new_content, pos.line, &insertion_content);
        }
        None => {
            // If no use statements are found, simply prepend the mod and use lines.
            new_content = format!("{}{}", insertion_content.trim_end(), new_content);
        }
    }

    fs::write(file_name, new_content)?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustFileSerialization {
    pub pax_path: String,
    pub pascal_identifier: String,
}

/// Serialize a new component rust file
pub fn press_rust_file_serialization_template(args: RustFileSerialization) -> String {
    let mut tera = Tera::default();

    tera.add_raw_template(
        RUST_FILE_SERIALIZATION_TEMPLATE,
        TEMPLATE_DIR
            .get_file(RUST_FILE_SERIALIZATION_TEMPLATE)
            .unwrap()
            .contents_utf8()
            .unwrap(),
    )
    .expect("Failed to add rust-file-serialization.tera");

    let context = Context::from_serialize(args).unwrap();

    // Serialize rust
    tera.render(RUST_FILE_SERIALIZATION_TEMPLATE, &context)
        .expect("Failed to render template")
}

fn insert_at_line(s: &mut String, line_number: usize, content_to_insert: &str) {
    // Split the string into lines
    let mut lines: Vec<&str> = s.lines().collect();

    // Check if the specified line number is within the bounds of the lines vector
    if line_number <= lines.len() {
        // Insert the content at the specified line number
        lines.insert(line_number, content_to_insert);
    } else {
        // If the line number is beyond the existing lines, append the content instead
        lines.push(content_to_insert);
    }
    // Rejoin the lines and update the original string
    *s = lines.join("\n");
}
