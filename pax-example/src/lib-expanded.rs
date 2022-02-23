#[macro_use]
extern crate lazy_static;

use pax::*;
use pax::api::{ArgsCoproduct, ArgsTick, Property};

use pax_std::primitives::{Group, Rectangle};

#[derive(Copy, Clone, Default)]
pub struct DeeperStruct {
    a: i64,
    b: &'static str,
}

pub mod pax_types {
    pub mod pax_std {
        pub mod primitives {
            pub use pax_std::primitives::Rectangle;
            pub use pax_std::primitives::RectangleProperties;
            pub use pax_std::primitives::Group;
            pub use pax_std::primitives::GroupProperties;
        }
        pub mod types {
            pub use pax_std::types::Color;
            pub use pax_std::types::StrokeProperties;
            pub use pax_std::types::Size;
        }
    }
    pub use pax::api::Transform;

    pub use crate::RootProperties;
    //plus other relevant.
}

//#[pax] was here
pub struct RootProperties {
    pub num_clicks: Box<dyn pax::api::Property<i64>>,
    pub current_rotation: Box<dyn pax::api::Property<f64>>,
    pub deeper_struct: Box<dyn pax::api::Property<DeeperStruct>>,
}

impl RootProperties {
    pub fn handle_tick(&mut self, args: ArgsTick) {
        pax::log(&format!("pax::log from frame {}", args.frame));
        self.current_rotation.set(self.current_rotation.get() + 0.10);
    }

    // pub fn _dispatch(&mut self, args: ArgsCoproduct)
}

#[cfg(feature = "parser")]
use pax::internal::message::ComponentDefinition;
#[cfg(feature = "parser")]
use parser;
#[cfg(feature = "parser")]
use parser::ManifestContext;
#[cfg(feature = "parser")]
use std::collections::HashMap;
#[cfg(feature = "parser")]
use std::collections::HashSet;
#[cfg(feature = "parser")]
use std::path::{Path, PathBuf};
#[cfg(feature = "parser")]
use std::{env, fs};
#[cfg(feature = "parser")]
use pax::internal::message::{SettingsValueDefinition, PaxManifest,SettingsLiteralBlockDefinition};

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
    let (ctx, _) = Root::parse_to_manifest(ctx);

    //TODO: should be able to de-dupe PaxManifest and ManifestContext data structures
    let manifest = PaxManifest {
        components: ctx.component_definitions,
        root_component_id: ctx.root_component_id,
    };

    println!("serialized bytes: {:?}", manifest.serialize());

    let tcp_port = std::env::var("PAX_TCP_CALLBACK_PORT").expect("TCP callback port not provided");
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

                println!("{:?}", ctx);

                (ctx, source_id.to_string())
            }
            _ => (ctx, source_id.to_string()), //early return; this file has already been parsed
        }
    }
}
//
// impl RootProperties {
//
//     //ideally, this would accept &mut self
//     pub fn handle_tick(&mut self, evt: pax::api::ArgsTick) {
//
//         &self.num_clicks.set(*self.num_clicks.get() + 1);
//         // let mut num_clicks = (*props).num_clicks;
//         // num_clicks.set(num_clicks.get() + 1);
//
//     }
//     // pub fn handle_tick(&mut props: RootProperties , evt: pax::api::EventTick) {
//     //     let mut num_clicks = (*props).num_clicks;
//     //     num_clicks.set(num_clicks.get() + 1)
//     // }
// }
//
