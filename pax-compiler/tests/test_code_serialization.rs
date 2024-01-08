use std::env;
use std::fs::File;
use std::io::{self, Read};

use std::sync::{Arc, Mutex};

use pax_compiler::code_serialization::serialize_component_to_file;
use pax_compiler::formatting::format_file;
use pax_compiler::helpers::clear_inlined_template;
use pax_compiler::run_parser_binary;
use pax_manifest::PaxManifest;

fn setup_test_project() {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let pkg_dir = current_dir.join("tests/data/code_serialization/serialization_test_project");
    let _ = std::process::Command::new("./pax")
        .current_dir(pkg_dir)
        .arg("build")
        .output()
        .expect("Failed to execute command");
}

fn clean_test_project() {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let pkg_dir = current_dir.join("tests/data/code_serialization/serialization_test_project");
    let _ = std::process::Command::new("./pax")
        .current_dir(pkg_dir)
        .arg("clean")
        .output()
        .expect("Failed to execute command");
}

#[test]
fn test_code_serializaton() {
    setup_test_project();

    // Get path to test project
    let current_dir = env::current_dir().expect("Failed to get current directory");
    // Join the current directory with the relative path
    let path = current_dir.join("tests/data/code_serialization/serialization_test_project");
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
    let process_child_ids: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(vec![]));
    let output = run_parser_binary(path_str, Arc::clone(&process_child_ids));

    let out = String::from_utf8(output.stdout).unwrap();
    let manifest: PaxManifest =
        serde_json::from_str(&out).expect(&format!("Malformed JSON from parser: {}", &out));

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
    clean_test_project();
}

fn read_file_to_string(file_path: &str) -> io::Result<String> {
    let mut file = File::open(file_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}
