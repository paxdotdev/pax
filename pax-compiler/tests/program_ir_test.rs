use std::path::PathBuf;

use pax_compiler::static_analysis::build_manifest;
use pax_manifest::program_ir::ProgramIR;

#[test]
fn increment_static_manifest_round_trips_through_program_ir_binary() {
    let workspace_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("pax-compiler should have a workspace parent")
        .to_path_buf();
    let project_path = workspace_dir.join("examples/src/increment");

    let manifest = build_manifest(&project_path).expect("increment manifest should build");
    let ir = ProgramIR::from_manifest(&manifest);
    let bytes = pax_manifest::program_ir::binary::to_vec(&ir).expect("program ir should serialize");
    let decoded = pax_manifest::program_ir::binary::from_slice(&bytes)
        .expect("program ir should deserialize");

    assert_eq!(decoded.main_component_type_id, ir.main_component_type_id);
    assert_eq!(decoded.components.len(), ir.components.len());
    assert_eq!(decoded.type_table.len(), ir.type_table.len());
    assert_eq!(decoded.assets_dirs, ir.assets_dirs);
    assert!(decoded.components.values().all(|component| component
        .template
        .as_ref()
        .map(|template| template.get_file_path().is_none())
        .unwrap_or(true)));
}
