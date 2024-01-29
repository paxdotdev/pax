use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use toml_edit::{Array, Document, Item, Table};

pub fn add_additional_dependencies_to_cargo_toml(
    dest: &PathBuf,
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
            array.push("pax-core/designtime");
            array.push("pax-cartridge/designtime");
            array.push("pax-runtime-api/designtime");
            doc["features"]["designtime"] = toml_edit::value(array);
            doc["dependencies"]["pax-designtime"] = pax_designtime_dependency;
        }
        "pax-core" | "pax-runtime-api" => {
            let mut array = Array::default();
            array.push("pax-designtime");
            doc["features"]["designtime"] = toml_edit::value(array);
            doc["dependencies"]["pax-designtime"] = pax_designtime_dependency;
        }
        "pax-lang" => {
            let mut array = Array::default();
            array.push("pax-runtime-api/designtime");
            doc["features"]["designtime"] = toml_edit::value(array);
        }
        "pax-cartridge" => {
            let mut array = Array::default();
            array.push("serde_json");
            array.push("include_dir");
            array.push("pax-designtime");
            array.push("pax-core/designtime");
            array.push("pax-runtime-api/designtime");
            doc["features"]["designtime"] = toml_edit::value(array);

            let mut array = Array::default();
            array.push("designtime");
            doc["features"]["default"] = toml_edit::value(array);

            doc["dependencies"]["serde_json"] =
                Item::from_str(r#"{ version = "1.0.95", optional = true }"#)?;
            doc["dependencies"]["include_dir"] =
                Item::from_str(r#"{ version = "0.7.3", features = ["glob"], optional = true }"#)?;
            doc["dependencies"]["pax-designtime"] = pax_designtime_dependency;
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
