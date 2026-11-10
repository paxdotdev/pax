use std::path::PathBuf;

use pax_compiler::static_analysis::build_manifest;

#[test]
fn increment_static_manifest_round_trips_through_binary_manifest() {
    let workspace_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("pax-compiler should have a workspace parent")
        .to_path_buf();
    let project_path = workspace_dir.join("examples/increment");

    let mut manifest = build_manifest(&project_path).expect("increment manifest should build");
    let bytes = pax_manifest::binary::to_vec(&manifest).expect("manifest should serialize");
    let decoded = pax_manifest::binary::from_slice(&bytes).expect("manifest should deserialize");

    // Document extraction contracts are compiler-only and intentionally excluded
    // from the binary manifest; all executable data must still round-trip.
    assert!(manifest
        .components
        .values()
        .any(|component| component.document.is_some()));
    assert!(decoded
        .components
        .values()
        .all(|component| component.document.is_none()));
    for component in manifest.components.values_mut() {
        component.document = None;
    }

    assert_eq!(
        serde_json::to_value(&decoded).expect("decoded manifest should serialize to json"),
        serde_json::to_value(&manifest).expect("source manifest should serialize to json")
    );
}
