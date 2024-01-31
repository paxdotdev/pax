use include_dir::{include_dir, Dir};
use pax_manifest::{helpers::{CommonProperty, ComponentInfo}, HostCrateInfo, PaxManifest};
#[allow(unused_imports)]
use serde_derive::{Deserialize, Serialize};
#[allow(unused_imports)]
use serde_json;
use tera::{Context, Tera};

#[allow(unused)]
static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/cartridge_generation");
static DESIGN_CARTRIDGE_TEMPLATE: &str = "designtime-cartridge.tera";
static MACROS_TEMPLATE: &str = "macros.tera";

#[derive(Serialize)]
pub struct TemplateArgsCodegenDesigntimeCartridge {
    components: Vec<ComponentInfo>,
    common_properties: Vec<CommonProperty>,
}


pub fn generate_designtime_cartridge(manifest: &PaxManifest, host_crate_info: &HostCrateInfo) -> String {
    let component_info = manifest.generate_codegen_component_info(host_crate_info);
    let args = TemplateArgsCodegenDesigntimeCartridge { components: component_info, common_properties: CommonProperty::get_as_common_property() };
    press_template_codegen_designtime_cartridge(args)
}


pub fn press_template_codegen_designtime_cartridge(
    args: TemplateArgsCodegenDesigntimeCartridge,
) -> String {
    let mut tera = Tera::default();

    tera.add_raw_template(
        MACROS_TEMPLATE,
        TEMPLATE_DIR
            .get_file(MACROS_TEMPLATE)
            .unwrap()
            .contents_utf8()
            .unwrap(),
    )
    .expect("Failed to add macros.tera");

    tera.add_raw_template(
        DESIGN_CARTRIDGE_TEMPLATE,
        TEMPLATE_DIR
            .get_file(DESIGN_CARTRIDGE_TEMPLATE)
            .unwrap()
            .contents_utf8()
            .unwrap(),
    )
    .expect("Failed to add designtime-cartridge.tera");

    tera.render(
        DESIGN_CARTRIDGE_TEMPLATE,
        &Context::from_serialize(args).unwrap(),
    )
    .expect("Failed to render template")
}
