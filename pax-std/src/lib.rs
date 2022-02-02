#[macro_use]
extern crate lazy_static;


use pax::*;

pub mod components;

pub use components::*;

pub mod primitives {
    use pax::pax_primitive;

    pax_primitive!(Group, "./pax-std-primitives");
    ///GENERATES THE FOLLOWING:
    pub struct Group {}
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
    use pax_message::{ComponentDefinition, SettingsValueDefinition, PaxManifest, SettingsLiteralBlockDefinition};
    #[cfg(feature = "parser")]
    lazy_static! {
        static ref source_id: String = parser::get_uuid();
    }
    #[cfg(feature = "parser")]
    impl Group {
        pub fn parse_to_manifest(mut ctx: ManifestContext) -> (ManifestContext, String) {
            match ctx.visited_source_ids.get(&source_id as &str) {
                None => {
                    //First time visiting this file/source — parse the relevant contents
                    //then recurse through child nodes, unrolled here in the macro as
                    //parsed from the template
                    ctx.visited_source_ids.insert(source_id.clone());

                    //GENERATE: gen explict_path value with macro
                    let explicit_path: Option<String> = None;
                    //TODO: support inline pax as an alternative to file
                    //GENERATE: inject pascal_identifier instead of CONSTANT
                    let PASCAL_IDENTIFIER = "Group";
                    //GENERATE: handle_file vs. handle_primitive
                    let component_definition_for_this_file = parser::handle_primitive(PASCAL_IDENTIFIER, module_path!(), &source_id as &str);
                    ctx.component_definitions.push(component_definition_for_this_file);
                    //GENERATE:
                    //Lead node; no template, no pax file, no children to generate

                    (ctx, source_id.to_string())
                },
                _ => { (ctx, source_id.to_string()) } //early return; this file has already been parsed
            }
        }
    }



    pax_primitive!(Rectangle, "./pax-std-primitives");
    ///GENERATES THE FOLLOWING:
    pub struct Rectangle {}
    //
    //TODO: figure out how to de-dupe the imports here vs. the previous pax_primitive!()
    //
    // #[cfg(feature = "parser")]
    // use parser;
    // #[cfg(feature = "parser")]
    // use parser::ManifestContext;
    // #[cfg(feature = "parser")]
    // use std::collections::HashMap;
    // #[cfg(feature = "parser")]
    // use std::collections::HashSet;
    // #[cfg(feature = "parser")]
    // use std::path::{Path, PathBuf};
    // #[cfg(feature = "parser")]
    // use std::{env, fs};
    // #[cfg(feature = "parser")]
    // use pax_message::{ComponentDefinition, SettingsValueDefinition, PaxManifest,SettingsLiteralBlockDefinition};
    // #[cfg(feature = "parser")]
    // lazy_static! {
    //     static ref source_id: String = parser::get_uuid();
    // }
    #[cfg(feature = "parser")]
    impl Group {
        pub fn parse_to_manifest(mut ctx: ManifestContext) -> (ManifestContext, String) {
            match ctx.visited_source_ids.get(&source_id as &str) {
                None => {
                    //First time visiting this file/source — parse the relevant contents
                    //then recurse through child nodes, unrolled here in the macro as
                    //parsed from the template
                    ctx.visited_source_ids.insert(source_id.clone());

                    //GENERATE: gen explict_path value with macro
                    let explicit_path: Option<String> = None;
                    //TODO: support inline pax as an alternative to file
                    //GENERATE: inject pascal_identifier instead of CONSTANT
                    let PASCAL_IDENTIFIER = "Group";
                    //GENERATE: handle_file vs. handle_primitive
                    let component_definition_for_this_file = parser::handle_primitive(PASCAL_IDENTIFIER, module_path!(), &source_id as &str);
                    ctx.component_definitions.push(component_definition_for_this_file);
                    //GENERATE:
                    //Lead node; no template, no pax file, no children to generate

                    (ctx, source_id.to_string())
                },
                _ => { (ctx, source_id.to_string()) } //early return; this file has already been parsed
            }
        }
    }
}