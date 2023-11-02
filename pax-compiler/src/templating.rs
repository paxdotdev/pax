use include_dir::{include_dir, Dir};
#[allow(unused_imports)]
use serde_derive::{Deserialize, Serialize};
#[allow(unused_imports)]
use serde_json;
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};
use tera::{Context, Tera};

use crate::manifest::{ExpressionSpec, PropertyDefinition};

static TEMPLATE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

#[derive(Serialize)]
pub struct TemplateArgsCodegenPropertiesCoproductLib {
    //e.g. `Rectangle(pax_example::pax_reexports::pax_std::primitives::Rectangle)`
    //      |-------| |--------------------------------------------------------|
    //      tuple.0   tuple.1
    pub properties_coproduct_tuples: Vec<(String, String)>,

    //e.g. `Stroke(    pax_example::pax_reexports::pax_std::types::Stroke)`
    //      |----|     |--------------------------------------------------------|
    //      tuple.0    tuple.1
    pub types_coproduct_tuples: Vec<(String, String)>,
}

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

#[derive(Serialize)]
pub struct TemplateArgsCodegenCartridgeComponentFactory {
    pub is_main_component: bool,
    pub snake_case_type_id: String,
    pub component_properties_struct: String,
    pub properties: Vec<(PropertyDefinition, String)>, //PropertyDefinition, TypeIdPascalized
    pub events: Vec<(MappedString, Vec<MappedString>)>,
    pub render_nodes_literal: String,
    pub properties_coproduct_variant: String,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct MappedString {
    pub content: String,
    /// Markers used to identify generated code range for source map.
    pub source_map_start_marker: Option<String>,
    pub source_map_end_marker: Option<String>,
}

impl PartialEq for MappedString {
    fn eq(&self, other: &Self) -> bool {
        self.content == other.content
    }
}

impl Eq for MappedString {}

impl Hash for MappedString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.content.hash(state);
    }
}

impl MappedString {
    pub fn none() -> Self {
        MappedString {
            content: "None".to_string(),
            source_map_start_marker: None,
            source_map_end_marker: None,
        }
    }

    pub fn new(content: String) -> Self {
        MappedString {
            content,
            source_map_start_marker: None,
            source_map_end_marker: None,
        }
    }
}

#[derive(Serialize)]
pub struct TemplateArgsCodegenCartridgeRenderNodeLiteral {
    pub is_primitive: bool,
    pub snake_case_type_id: String,
    pub primitive_instance_import_path: Option<String>,
    pub properties_coproduct_variant: String,
    pub component_properties_struct: String,
    pub defined_properties: Vec<(MappedString, MappedString)>,
    //0: property id (e.g. "width"), 1: property value literal RIL (e.g. "None" or "Some(Rc::new(...))"
    pub common_properties_literal: Vec<(MappedString, MappedString)>,
    pub children_literal: Vec<String>,
    pub slot_index_literal: MappedString,
    pub repeat_source_expression_literal_vec: MappedString,
    pub repeat_source_expression_literal_range: MappedString,
    pub conditional_boolean_expression_literal: MappedString,
    pub pascal_identifier: String,
    pub type_id_escaped: String,
    pub events: Vec<(MappedString, MappedString)>,
}

#[allow(unused)]
static TEMPLATE_CODEGEN_PROPERTIES_COPRODUCT_LIB: &str =
    include_str!("../templates/properties-coproduct-lib.tera");
pub fn press_template_codegen_properties_coproduct_lib(
    args: TemplateArgsCodegenPropertiesCoproductLib,
) -> String {
    let template = TEMPLATE_DIR
        .get_file("properties-coproduct-lib.tera")
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
static TEMPLATE_CODEGEN_CARTRIDGE_LIB: &str = include_str!("../templates/cartridge-lib.tera");
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
    include_str!("../templates/cartridge-component-factory.tera");
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
    include_str!("../templates/cartridge-render-node-literal.tera");
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
