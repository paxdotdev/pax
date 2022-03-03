#[macro_use]
extern crate lazy_static;


use pax::*;

pub mod components;
pub mod types;
pub mod spread;

pub use components::*;

use pax::api::Property;

pub mod primitives {
    use pax::pax_primitive;


    #[pax_primitive("./pax-std-primitives", crate::GroupInstance)]
    pub struct Group {
        pub transform: pax::api::Transform
    }

    pub struct GroupProperties {
    }


    /// Simple way to represent whether a spread should render
    /// vertically or horizontally
    pub enum SpreadDirection {
        Vertical,
        Horizontal,
    }

    impl Default for SpreadDirection {
        fn default() -> Self {
            SpreadDirection::Horizontal
        }
    }

    pub struct SpreadCellProperties {
        pub x_px: f64,
        pub y_px: f64,
        pub width_px: f64,
        pub height_px: f64,
    }

    pub struct SpreadProperties {

        pub direction:  Box<dyn pax::api::Property<SpreadDirection>>,
        pub cell_count: Box<dyn pax::api::Property<usize>>,
        pub gutter_width: Box<dyn pax::api::Property<pax::api::Size>>,

        //These two data structures act as "sparse maps," where
        //the first element in the tuple is the index of the cell/gutter to
        //override and the second is the override value.  In the absence
        //of overrides (`vec![]`), cells and gutters will divide space
        //evenly.
        //TODO: these should probably be Expressable
        pub overrides_cell_size: Vec<(usize, pax::api::Size)>,
        pub overrides_gutter_size: Vec<(usize, pax::api::Size)>,

        //storage for memoized layout calc
        //TODO: any way to make this legit private while supporting `..Default::default()` ergonomics?
        pub _cached_computed_layout_spec: Vec<Rc<SpreadCellProperties>>,
    }



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
    use std::rc::Rc;
    #[cfg(feature = "parser")]
    use pax::internal::message::{ComponentDefinition, PaxManifest, SettingsLiteralBlockDefinition, SettingsValueDefinition};
    use crate::types;
    #[cfg(feature = "parser")]
    lazy_static! {
        static ref source_id: String = parser::create_uuid();
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



    #[pax_primitive("./pax-std-primitives", crate::RectangleInstance)]
    pub struct Rectangle {
        pub stroke: types::Stroke,
        pub fill: types::Color,
    }

    pub struct RectangleProperties {
        pub stroke: Box<dyn pax::api::Property<types::StrokeProperties>>,
        pub fill: Box<dyn pax::api::Property<types::Color>>,
    }

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
    impl Rectangle {
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
