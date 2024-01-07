use include_dir::{include_dir, Dir};
#[allow(unused_imports)]
use serde_derive::{Deserialize, Serialize};
#[allow(unused_imports)]
use serde_json;
use tera::{Context, Tera};

use pax_manifest::{ExpressionSpec, MappedString, PropertyDefinition};

static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/cartridge_generation");

#[derive(Serialize)]
pub struct TemplateArgsCodegenCartridgeLib {
    /// List of fully qualified import strings, e.g. pax_example::pax_reexports::...
    pub imports: Vec<String>,

    /// List of `const `declarations: full token streams ready to re-write
    pub consts: Vec<String>,

    /// List of compiled expression specs
    pub expression_specs: Vec<ExpressionSpec>,

    /// List of component factory definitions, as pre-assembled literal Strings.
    pub component_factories_literal: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct TemplateArgsCodegenCartridgeComponentFactory {
    pub is_main_component: bool,
    pub snake_case_type_id: String,
    pub component_properties_struct: String,
    pub properties: Vec<(PropertyDefinition, String)>, //PropertyDefinition, FullyQualifiedTypeId
    pub handlers: Vec<(MappedString, Vec<MappedString>)>,
    pub render_nodes_literal: String,
    pub fully_qualified_properties_type: String,
}

#[derive(Serialize)]
pub struct TemplateArgsCodegenCartridgeRenderNodeLiteral {
    pub is_primitive: bool,
    pub snake_case_type_id: String,
    pub primitive_instance_import_path: Option<String>,

    /// Used to generate invocations of event handlers, the `Foo` in `Foo::some_handler()`
    pub containing_component_struct: String,

    pub component_properties_struct: String,
    pub defined_properties: Vec<(MappedString, MappedString)>,
    /// Tuple fields for `common_properties_literal`:
    ///   0: property id (e.g. "width")
    ///   1: property value literal RIL (e.g. "None" or "Some(Rc::new(...))"
    pub common_properties_literal: Vec<(MappedString, MappedString)>,
    pub children_literal: Vec<String>,
    // pub slot_index_literal: MappedString,
    // pub repeat_source_expression_literal_vec: MappedString,
    // pub repeat_source_expression_literal_range: MappedString,
    // pub conditional_boolean_expression_literal: MappedString,
    pub pascal_identifier: String,
    pub type_id_escaped: String,
    pub handlers: Vec<(MappedString, MappedString)>,
    pub fully_qualified_properties_type: String,
}

#[allow(unused)]
static TEMPLATE_CODEGEN_CARTRIDGE_LIB: &str =
    include_str!("../../templates/cartridge_generation/cartridge-lib.tera");
pub fn press_template_codegen_cartridge_lib(args: TemplateArgsCodegenCartridgeLib) -> String {
    let template = TEMPLATE_DIR
        .get_file("cartridge-lib.tera")
        .unwrap()
        .contents_utf8()
        .unwrap();
    Tera::one_off(
        template.into(),
        &tera::Context::from_serialize(args).unwrap(),
        false,
    )
    .unwrap()
}

#[allow(unused)]
static TEMPLATE_CODEGEN_CARTRIDGE_COMPONENT_FACTORY: &str =
    include_str!("../../templates/cartridge_generation/cartridge-component-factory.tera");
pub fn press_template_codegen_cartridge_component_factory(
    args: TemplateArgsCodegenCartridgeComponentFactory,
) -> String {
    let template = TEMPLATE_DIR
        .get_file("cartridge-component-factory.tera")
        .unwrap()
        .contents_utf8()
        .unwrap();
    Tera::one_off(
        template.into(),
        &tera::Context::from_serialize(args).unwrap(),
        false,
    )
    .unwrap()
}

#[allow(unused)]
static TEMPLATE_CODEGEN_CARTRIDGE_RENDER_NODE_LITERAL: &str =
    include_str!("../../templates/cartridge_generation/cartridge-render-node-literal.tera");
pub fn press_template_codegen_cartridge_render_node_literal(
    args: TemplateArgsCodegenCartridgeRenderNodeLiteral,
) -> String {
    let template = TEMPLATE_DIR
        .get_file("cartridge-render-node-literal.tera")
        .unwrap()
        .contents_utf8()
        .unwrap();

    let mut tera = Tera::default();
    tera.add_raw_template("cartridge-render-node-literal", template)
        .unwrap();

    tera.render(
        "cartridge-render-node-literal",
        &Context::from_serialize(args).unwrap(),
    )
    .unwrap()
}
