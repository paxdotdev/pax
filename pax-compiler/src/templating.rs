
use serde_derive::{Serialize, Deserialize};
use serde_json;
use include_dir::{include_dir, Dir};
use tera::{Context, Tera};
use std::collections::{HashMap, HashSet};

use crate::manifest::{ExpressionSpec, PropertyDefinition, EventDefinition};

static ROOT_PATH : &str = "$CARGO_MANIFEST_DIR/templates";
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
    pub is_root: bool,
    pub snake_case_component_id: String,
    pub component_properties_struct: String,
    pub properties: Vec<PropertyDefinition>,
    pub events: HashMap<String,Vec<String>>,
    pub render_nodes_literal: String,
    pub properties_coproduct_variant: String,
}

#[derive(Serialize)]
pub struct TemplateArgsCodegenCartridgeRenderNodeLiteral {
    pub is_primitive: bool,
    pub snake_case_component_id: String,
    pub primitive_instance_import_path: Option<String>,
    pub properties_coproduct_variant: String,
    pub component_properties_struct: String,
    pub properties: Vec<(String, String)>,
    pub size_ril: [String; 2],
    pub transform_ril: String,
    pub children_literal: Vec<String>,
    pub slot_index_literal: String,
    pub repeat_source_expression_literal: String,
    pub conditional_boolean_expression_literal: String,
    pub active_root: String,
    pub events: HashMap<String,String>,
}

static TEMPLATE_CODEGEN_PROPERTIES_COPRODUCT_LIB : &str = include_str!("../templates/properties-coproduct-lib.tera");
pub fn press_template_codegen_properties_coproduct_lib(args: TemplateArgsCodegenPropertiesCoproductLib ) -> String {
    let template = TEMPLATE_DIR.get_file("properties-coproduct-lib.tera").unwrap().contents_utf8().unwrap();
    Tera::one_off(template.into(), &tera::Context::from_serialize(args).unwrap(), false).unwrap()
}

static TEMPLATE_CODEGEN_CARTRIDGE_LIB : &str = include_str!("../templates/cartridge-lib.tera");
pub fn press_template_codegen_cartridge_lib(args: TemplateArgsCodegenCartridgeLib ) -> String {
    let template = TEMPLATE_DIR.get_file("cartridge-lib.tera").unwrap().contents_utf8().unwrap();
    Tera::one_off(template.into(), &tera::Context::from_serialize(args).unwrap(), false).unwrap()
}

static TEMPLATE_CODEGEN_CARTRIDGE_COMPONENT_FACTORY : &str = include_str!("../templates/cartridge-component-factory.tera");
pub fn press_template_codegen_cartridge_component_factory(args: TemplateArgsCodegenCartridgeComponentFactory) -> String {
    let template = TEMPLATE_DIR.get_file("cartridge-component-factory.tera").unwrap().contents_utf8().unwrap();
    Tera::one_off(template.into(), &tera::Context::from_serialize(args).unwrap(), false).unwrap()
}

static TEMPLATE_CODEGEN_CARTRIDGE_RENDER_NODE_LITERAL : &str = include_str!("../templates/cartridge-render-node-literal.tera");
pub fn press_template_codegen_cartridge_render_node_literal(args: TemplateArgsCodegenCartridgeRenderNodeLiteral) -> String {
    let template = TEMPLATE_DIR.get_file("cartridge-render-node-literal.tera").unwrap().contents_utf8().unwrap();
    Tera::one_off(template.into(), &tera::Context::from_serialize(args).unwrap(), false).unwrap()
}



