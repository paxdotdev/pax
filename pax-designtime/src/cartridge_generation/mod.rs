use include_dir::{include_dir, Dir};
#[allow(unused_imports)]
use serde_derive::{Deserialize, Serialize};
#[allow(unused_imports)]
use serde_json;
use tera::{Context, Tera};

static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/cartridge_generation");

#[derive(Serialize)]
pub struct TemplateArgsCodegenDesigntimeCartridge {}

#[allow(unused)]
static TEMPLATE_CODEGEN_CARTRIDGE_RENDER_NODE_LITERAL: &str =
    include_str!("../../templates/cartridge_generation/designtime-cartridge.tera");
pub fn press_template_codegen_designtime_cartridge(
    args: TemplateArgsCodegenDesigntimeCartridge,
) -> String {
    let template = TEMPLATE_DIR
        .get_file("designtime-cartridge.tera")
        .unwrap()
        .contents_utf8()
        .unwrap();

    let mut tera = Tera::default();
    tera.add_raw_template("designtime-cartridge", template)
        .unwrap();

    tera.render(
        "designtime-cartridge",
        &Context::from_serialize(args).unwrap(),
    )
    .unwrap()
}
