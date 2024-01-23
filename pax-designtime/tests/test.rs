
use pax_manifest::PaxManifest;
const INITAL_MANIFEST: &str = include_str!("../initial_manifest.json");
pub fn initialize_designtime() -> pax_designtime::DesigntimeManager {
    let manifest: PaxManifest = serde_json::from_str(INITAL_MANIFEST).unwrap();
    let mut _designtime = pax_designtime::DesigntimeManager::new(manifest);
    // Add factories here
    _designtime
}

#[test]
fn test_code_deserialization_code_gen() {
    initialize_designtime();
}
