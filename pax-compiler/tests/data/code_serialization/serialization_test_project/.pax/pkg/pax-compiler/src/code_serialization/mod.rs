use std::{fs::{File, self}, io::Write, path::Path};

use syn::{parse_file, visit::Visit};
use tera::{Context, Tera};

use include_dir::{include_dir, Dir};
use pax_manifest::ComponentDefinition;

use crate::{helpers::{InlinedTemplateFinder, replace_by_line_column}, formatting::format_pax_template};

#[allow(unused)]
static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/code_serialization");
#[allow(unused)]
static MANIFEST_CODE_SERIALIZATION_TEMPLATE: &str = "manifest-code-serialization.tera";
#[allow(unused)]
static MACROS_TEMPLATE: &str = "macros.tera";

/// Serialize a component to a string
pub fn press_code_serialization_template(args: ComponentDefinition) -> String {
    let mut tera = Tera::default();

    tera.add_raw_template(
        MACROS_TEMPLATE,
        TEMPLATE_DIR
            .get_file(MACROS_TEMPLATE)
            .unwrap()
            .contents_utf8()
            .unwrap(),
    )
    .expect("Failed to add macros.tera");

    tera.add_raw_template(
        MANIFEST_CODE_SERIALIZATION_TEMPLATE,
        TEMPLATE_DIR
            .get_file(MANIFEST_CODE_SERIALIZATION_TEMPLATE)
            .unwrap()
            .contents_utf8()
            .unwrap(),
    )
    .expect("Failed to add manifest-code-serialization.tera");

    let context = Context::from_serialize(args).unwrap();

    // Serialize component
    let template = tera.render(MANIFEST_CODE_SERIALIZATION_TEMPLATE, &context)
        .expect("Failed to render template");

    // Format component
    format_pax_template(template).expect("Failed to format template")
}


/// Serialize a component to a file
/// Replaces entire .pax file and replaces inlined attribute directly for .rs files
pub fn serialize_component_to_file(component: &ComponentDefinition, file_path: String) {
let path = Path::new(&file_path);
let pascal_identifier = component.pascal_identifier.clone();
let serialized_component = press_code_serialization_template(component.clone());

match path.extension().and_then(|s| s.to_str()) {
    Some("pax") => {
        let mut file = File::create(file_path).expect("Failed to create file");
        file.write_all(serialized_component.as_bytes()).expect("Failed to write to file");
    },
    Some("rs") => write_inlined_pax(serialized_component, path, pascal_identifier),
    _ => panic!("Unsupported file extension."),
}
}

fn write_inlined_pax(serialized_component: String, path: &Path, pascal_identifier: String) {
    let content = fs::read_to_string(path).expect("Failed to read file");
    let ast = parse_file(&content).expect("Failed to parse file");
    let mut finder = InlinedTemplateFinder::new();
    finder.visit_file(&ast);

    let template = finder.templates.iter().find(|t| {
        t.struct_name == pascal_identifier
    });

    if let Some(data) = template {
        let new_template = format!("(\n{}\n)", serialized_component);
        let modified_content = replace_by_line_column(&content, data.start, data.end, new_template);
        fs::write(path, modified_content).expect("Failed to write to file");
    }
}

