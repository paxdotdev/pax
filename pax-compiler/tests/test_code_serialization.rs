use std::fs::{self, File};
use std::io::{self, Read};

use pax_compiler::code_serialization::press_code_serialization_template;
use pax_compiler::formatting::format_pax_template;
use pax_manifest::PaxManifest;

#[test]
fn test_code_serializaton() {
    let website_pax_result = read_file_to_string("tests/data/website.pax");
    assert!(website_pax_result.is_ok(), "Failed to read website.pax");
    let original_pax = website_pax_result.unwrap();
    let formatted_original_pax = format_pax_template(original_pax);

    let manifest_result = read_file_to_string("tests/data/manifest.json");
    assert!(manifest_result.is_ok(), "Failed to read manifest.json");
    let manifest_json = manifest_result.unwrap();

    let manifest: PaxManifest =
        serde_json::from_str(&manifest_json).expect(&format!("Malformed JSON: {}", &manifest_json));

    let main_component = manifest
        .components
        .get(&manifest.main_component_type_id)
        .unwrap();

    let serialized_pax = press_code_serialization_template(main_component.clone());
    let formatted_serialized_pax = format_pax_template(serialized_pax.clone());

    assert_eq!(
        formatted_serialized_pax.unwrap(),
        formatted_original_pax.unwrap()
    );
}

fn read_file_to_string(file_path: &str) -> io::Result<String> {
    let mut file = File::open(file_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}
