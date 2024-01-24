use include_dir::{include_dir, Dir};
use pax_manifest::{HostCrateInfo, PaxManifest};
#[allow(unused_imports)]
use serde_derive::{Deserialize, Serialize};
#[allow(unused_imports)]
use serde_json;
use tera::{Context, Tera};

#[allow(unused)]
static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/cartridge_generation");
static DESIGN_CARTRIDGE_TEMPLATE: &str = "designtime-cartridge.tera";


pub fn generate_designtime_cartridge(manifest: PaxManifest, host_crate_info: &HostCrateInfo) -> String {
    let args = TemplateArgsCodegenDesigntimeCartridge {};
    press_template_codegen_designtime_cartridge(args)
}




#[derive(Serialize)]
pub struct TemplateArgsCodegenDesigntimeCartridge {}

pub fn press_template_codegen_designtime_cartridge(
    args: TemplateArgsCodegenDesigntimeCartridge,
) -> String {
    let mut tera = Tera::default();

    let template = TEMPLATE_DIR
        .get_file(DESIGN_CARTRIDGE_TEMPLATE)
        .unwrap()
        .contents_utf8()
        .unwrap();

    tera.add_raw_template(DESIGN_CARTRIDGE_TEMPLATE, template)
        .expect("Failed to add designtime-cartridge.tera");

    tera.render(
        DESIGN_CARTRIDGE_TEMPLATE,
        &Context::from_serialize(args).unwrap(),
    )
    .expect("Failed to render template")
}
