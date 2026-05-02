use std::env;
use std::fs::File;
use std::io::{self, Read};

use pax_language::formatting::format_file;
use pax_language::helpers::clear_inlined_template;
use pax_manifest::code_serialization::serialize_component_to_file;

const PATH: &str = "tests/data/code_serialization/serialization_test_project";

#[test]
fn test_code_serializaton() {
    // Get path to test project
    let current_dir = env::current_dir().expect("Failed to get current directory");
    // Join the current directory with the relative path
    let path = current_dir.join(PATH);
    let path_str = path.to_str().expect("Path is not a valid UTF-8 string");

    // Format test project
    let original_file = path.join("src/lib.rs");
    let original_file_path = original_file
        .to_str()
        .expect("Path is not a valid UTF-8 string");
    format_file(original_file_path).expect("Failed to format file");

    // Clean designated output file (clear template)
    let generated_file = path.join("src/generated_lib.rs");
    let generated_file_path = generated_file
        .to_str()
        .expect("Path is not a valid UTF-8 string");
    clear_inlined_template(generated_file_path, "Example");

    // Serialize component to output file
    let manifest =
        pax_compiler::static_analysis::build_manifest(&std::path::PathBuf::from(path_str))
            .expect("static manifest should build");
    let main_component = manifest
        .components
        .get(&manifest.main_component_type_id)
        .unwrap();

    serialize_component_to_file(main_component, generated_file_path.to_string());

    // Check difference between original and generated file
    let original_lib = read_file_to_string(original_file_path).unwrap();
    let generated_lib = read_file_to_string(generated_file_path).unwrap();
    assert_eq!(original_lib, generated_lib);

    // Clean up for next time
    clear_inlined_template(generated_file_path, "Example");
}

fn read_file_to_string(file_path: &str) -> io::Result<String> {
    let mut file = File::open(file_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}
