use pax_manifest::*;
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let mut m: PaxManifest = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    let root = m
        .components
        .keys()
        .find(|id| id.get_unique_identifier() == "crate::Paxflix")
        .unwrap()
        .clone();
    m.main_component_type_id = root;
    m.components.retain(|id, _| {
        id.get_unique_identifier().starts_with("crate::")
            && id.get_unique_identifier() != "crate::Shell"
    });
    m.type_table.retain(|id, _| {
        id.get_unique_identifier().contains("crate::")
            && id.get_unique_identifier() != "crate::Shell"
    });
    m.assets_dirs.clear();
    let bytes = binary::to_vec(&m).unwrap();
    assert_eq!(
        bytes,
        binary::to_vec(&binary::from_slice(&bytes).unwrap()).unwrap()
    );
    std::fs::write(&args[2], bytes).unwrap();
    std::fs::write(&args[3], serde_json::to_vec(&m).unwrap()).unwrap();
}
