#[macro_use]
extern crate lazy_static;

use pax::std::{Group, Rectangle};

pub struct DeeperStruct {
    a: i64,
    b: &'static str,
}

//#[pax] was here
pub struct Root {
    //rewrite to pub `num_clicks : Property<i64>` etc. AND register metadata with dev server
    pub num_clicks: i64,
    pub current_rotation: f64,
    pub deeper_struct: DeeperStruct,
}

#[cfg(feature = "parser")]
use pax::message::ComponentDefinition;
#[cfg(feature = "parser")]
use pax::parser;
#[cfg(feature = "parser")]
use pax::parser::ManifestContext;
#[cfg(feature = "parser")]
use std::collections::HashMap;
#[cfg(feature = "parser")]
use std::collections::HashSet;
#[cfg(feature = "parser")]
use std::path::{Path, PathBuf};
#[cfg(feature = "parser")]
use std::{env, fs};
#[cfg(feature = "parser")]
lazy_static! {
    static ref source_id: String = parser::get_uuid();
}
//generated if lib.rs
#[cfg(feature = "parser")]
pub fn main() {
    let mut ctx = ManifestContext {
        root_component_id: "".into(),
        visited_source_ids: HashSet::new(),
        component_definitions: vec![],
    };
    let _ = Root::parse_to_manifest(ctx);
}

#[cfg(feature = "parser")]
//GENERATE pascal_identifier
impl Root {
    pub fn parse_to_manifest(mut ctx: ManifestContext) -> (ManifestContext, String) {
        match ctx.visited_source_ids.get(&source_id as &str) {
            None => {
                //First time visiting this file/source — parse the relevant contents
                //then recurse through child nodes, unrolled here in the macro as
                //parsed from the template
                ctx.visited_source_ids.insert(source_id.clone());

                //GENERATE: gen explict_path value with macro
                let explicit_path: Option<String> = Some("lib.pax".to_string());
                //TODO: support inline pax as an alternative to file
                let mut template_map: HashMap<String, String> = HashMap::new();

                //GENERATE:
                let (mut ctx, component_id) = Rectangle::parse_to_manifest(ctx);
                template_map.insert("Rectangle".into(), component_id);
                let (mut ctx, component_id) = Group::parse_to_manifest(ctx);
                template_map.insert("Group".into(), component_id);

                //GENERATE: inject pascal_identifier instead of CONSTANT
                let PASCAL_IDENTIFIER = "Root";

                let (mut ctx, component_definition_for_this_file) = parser::handle_file(
                    ctx,
                    file!(),
                    module_path!(),
                    explicit_path,
                    PASCAL_IDENTIFIER,
                    template_map,
                    &source_id as &str,
                );
                ctx.component_definitions
                    .push(component_definition_for_this_file);

                //TODO: need to associate component IDs with template nodes, so that
                //      component tree can be renormalized.
                //      - should source_id and component_id be de-duped?
                //        Note that this would further-separate us from multiple-
                //        components-per-source-file support
                //      - where should the linking occur? will require tangling the ID generation
                //        logic a bit
                //can create a map per-file (here) of pascal_identifier => uuid,
                //which can be passed to template parsing to resolve pascal_identifier => component_id in order to track a tree of
                //component instances (via component_id)

                println!("Generated context {:?}", ctx);

                (ctx, source_id.to_string())
            }
            _ => (ctx, source_id.to_string()), //early return; this file has already been parsed
        }
    }
}

impl Root {
    pub fn new() -> Self {
        Self {
            //Default values.  Could shorthand this into a macro via PAXEL
            num_clicks: 0,
            current_rotation: 0.0,
            deeper_struct: DeeperStruct {
                a: 100,
                b: "Profundo!",
            },
        }
    }
}

//DONE: is all descendent property access via Actions + selectors? `$('#some-desc').some_property`
//      or do we need a way to support declaring desc. properties?
//      We do NOT need a way to declar desc. properties here — because they are declared in the
//      `properties` blocks of .dash
