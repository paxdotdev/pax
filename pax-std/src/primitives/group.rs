use std::cell::RefCell;
use std::rc::Rc;

use pax_core::{RenderNode, RenderNodePtr, RenderNodePtrList, Transform};
use pax_core::rendering::Size2D;

/// Gathers a set of children underneath a single render node:
/// useful for composing transforms and simplifying render trees.
/// #[pax primitive]
pub struct Group {
    pub children: Rc<RefCell<Vec<RenderNodePtr>>>,
    pub id: String,
    pub transform: Rc<RefCell<Transform>>,

}


#[cfg(feature="parser")]
use pax_message::ComponentDefinition;
#[cfg(feature="parser")]
use parser;
#[cfg(feature="parser")]
use std::collections::HashSet;
#[cfg(feature="parser")]
use std::{env, fs};
#[cfg(feature="parser")]
use std::path::{Path, PathBuf};
#[cfg(feature="parser")]
use parser::ManifestContext;
#[cfg(feature="parser")]
lazy_static! {
    static ref source_id : String = parser::get_uuid();
}
#[cfg(feature="parser")]
//GENERATE pascal_identifier
impl Group {
    pub fn parse_to_manifest(mut ctx: ManifestContext) -> ManifestContext {

        match ctx.visited_source_ids.get(&source_id as &str) {
            None => {
                //First time visiting this file/source — parse the relevant contents
                //then recurse through child nodes, unrolled here in the macro as
                //parsed from the template
                ctx.visited_source_ids.insert(source_id.clone());

                //GENERATE: gen explict_path value with macro
                let explicit_path : Option<String> = None;
                //TODO: support inline pax as an alternative to file
                //GENERATE: inject pascal_identifier instead of CONSTANT
                let PASCAL_IDENTIFIER = "Group";
                //GENERATE: handle_file vs. handle_primitive
                let component_definition_for_this_file = parser::handle_primitive(PASCAL_IDENTIFIER);
                ctx.component_definitions.push(component_definition_for_this_file);
                //GENERATE:
                //Lead node; no template, no pax file, no children to generate

                ctx
            },
            _ => {ctx} //early return; this file has already been parsed
        }

    }
}


impl RenderNode for Group {
    fn get_rendering_children(&self) -> RenderNodePtrList {
        Rc::clone(&self.children)
    }
    fn get_size(&self) -> Option<Size2D> { None }
    fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) { bounds }
    fn get_transform(&mut self) -> Rc<RefCell<Transform>> { Rc::clone(&self.transform) }
}
