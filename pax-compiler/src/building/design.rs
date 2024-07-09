use std::fs;
use std::path::Path;
use std::str::FromStr;
use toml_edit::{Array, Document, Item, Table};

pub fn add_additional_dependencies_to_cargo_toml(
    dest: &Path,
    pkg: &str,
) -> Result<(), toml_edit::TomlError> {
    let cargo_toml_path = dest.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path).expect("Failed to read Cargo.toml");

    let mut doc = content
        .parse::<Document>()
        .expect("Failed to parse Cargo.toml");

    let pax_designtime_dependency =
        Item::from_str(r#"{ path = "../pax-designtime", optional = true }"#)?;

    if !doc.contains_key("features") {
        doc["features"] = toml_edit::Item::Table(Table::new());
    }

    match pkg {
        "pax-chassis-web" => {
            let mut array = Array::default();
            array.push("pax-designtime");
            array.push("pax-runtime/designtime");
            doc["features"]["designtime"] = toml_edit::value(array);
            doc["dependencies"]["pax-designtime"] = pax_designtime_dependency;
        }
        "pax-runtime" => {
            let mut array = Array::default();
            array.push("pax-designtime");
            doc["features"]["designtime"] = toml_edit::value(array);
            doc["dependencies"]["pax-designtime"] = pax_designtime_dependency;
        }
        "pax-engine" => {
            let mut array = Array::default();
            array.push("pax-runtime/designtime");
            doc["features"]["designtime"] = toml_edit::value(array);
        }
        "pax-designtime" => {
            doc["dependencies"]["pax-manifest"] = Item::from_str(r#"{ path="../pax-manifest" }"#)?;
        }
        _ => {}
    }

    // Write back to file
    fs::write(cargo_toml_path, doc.to_string()).expect("Failed to write to Cargo.toml");

    Ok(())
}
