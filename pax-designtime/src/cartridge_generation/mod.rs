use include_dir::{include_dir, Dir};
use pax_manifest::{
    cartridge_generation::{CommonProperty, ComponentInfo},
    HostCrateInfo, PaxManifest, TypeTable,
};
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
    type_table: TypeTable,
}

pub fn generate_designtime_cartridge(
    manifest: &PaxManifest,
    _host_crate_info: &HostCrateInfo,
) -> String {
    let component_info = manifest.generate_codegen_component_info();
    let args = TemplateArgsCodegenDesigntimeCartridge {
        components: component_info,
        common_properties: CommonProperty::get_as_common_property(),
        type_table: manifest.type_table.clone(),
    };
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