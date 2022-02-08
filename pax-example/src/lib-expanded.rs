#[macro_use]
extern crate lazy_static;

use pax::*;

use pax_std::primitives::{Group, Rectangle};
// use pax::core::{ComponentInstance, HostPlatformContext, RenderNode, RenderNodePtrList, RenderTreeContext, Size2D, Transform};
// use std::cell::RefCell;
// use std::rc::Rc;

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
            pub use pax_std::types::Stroke;
            pub use pax_std::types::Size;
        }
    }

    pub use crate::Root;
    //plus other relevant.
}

//#[pax] was here
pub struct Root {
    //rewrite to pub `num_clicks : Property<i64>` etc. AND register metadata with dev server
    pub num_clicks: i64,
    pub current_rotation: f64,
    pub deeper_struct: DeeperStruct,
}

pub struct RootProperties {
    pub num_clicks: Box<dyn pax::api::Property<i64>>,
    pub current_rotation: Box<dyn pax::api::Property<f64>>,
    pub deeper_struct: Box<dyn pax::api::Property<DeeperStruct>>,
}

//How is this consumed?
// - Chassis reaches into cartridge and calls this method (might need to dyn+impl)
// - Chassis then passes the returned instance to the Engine to start rendering
//
// fn get_root_component_instance() -> Rc<RefCell<ComponentInstance>> {
//     //TODO: spell out literal RIL tree here, accepting data either by env or by TCP
//     //This string is generated to this sourcefile via macro, and the definition tree is traversed at compiletime (macro generation time) to output the following.
//     Rc::new(RefCell::new(ComponentInstance{
//
//     }))
// }


//
// impl Root {
//     pub fn create_instance() -> Rc<RefCell<ComponentInstance>> {
//         let ret = ComponentInstance {
//             template: Rc::new(RefCell::new(vec![])),
//             adoptees: Rc::new(RefCell::new(vec![])),
//             transform: Rc::new(RefCell::new(Default::default())),
//             properties: Rc::new(RefCell::new(PropertiesCoproduct::Root)),
//             timeline: None
//         };
//
//         Rc::new(RefCell::new(ret))
//     }
// }

//Probably don't need to do the whole impl RenderNode chunk!
//can just inflate a ComponentInstance and pass it to PaxEngine
//In Root's case: set as root_component
//
// fn get_instance() -> Rc<RefCell<ComponentInstance>> {
//     // Rc::new(RefCell::new(ComponentInstance {
//     //     template: RenderNodePtrList,
//     //     adoptees: RenderNodePtrList,
//     //     transform: Rc<RefCell<Transform>>,
//     //     properties: Rc<RefCell<PropertiesCoproduct>>,
//     //     timeline: Option<Rc<RefCell<Timeline>>>,
//     // })
// }
// Don't need to do the following for standard userland component -- return a ComponentInstance instead
// pub struct RootInstance {
//     pub size: Size2D,
//     pub transform: Rc<RefCell<Transform>>,
//     pub properties: Rc<Refcell<RootProperties>>,
// }
//
// impl RenderNode for RootInstance {
//     fn get_rendering_children(&self) -> RenderNodePtrList {
//         todo!()
//     }
//
//     fn get_size(&self) -> Option<Size2D> {
//         todo!()
//     }
//
//     fn should_flatten(&self) -> bool {
//         todo!()
//     }
//
//     fn get_size_calc(&self, bounds: (f64, f64)) -> (f64, f64) {
//         todo!()
//     }
//
//     fn get_transform(&mut self) -> Rc<RefCell<Transform>> {
//         todo!()
//     }
//
//     fn compute_properties(&mut self, _rtc: &mut RenderTreeContext) {
//         todo!()
//     }
//
//     fn pre_render(&mut self, _rtc: &mut RenderTreeContext, _hpc: &mut HostPlatformContext) {
//         todo!()
//     }
//
//     fn render(&self, _rtc: &mut RenderTreeContext, _hpc: &mut HostPlatformContext) {
//         todo!()
//     }
//
//     fn post_render(&mut self, _rtc: &mut RenderTreeContext, _hpc: &mut HostPlatformContext) {
//         todo!()
//     }
// }

//
// pub struct Whatever {
//     pub x: |in: i64|{in + 6},
// }
//
// pub struct PropertyX<T>(T);
// impl<T: Copy> PropertyX<T> {
//     fn get (&self) -> T {
//        self.0
//     }
//     fn set (mut self, val: T) {
//         self = Self(val);
//     }
// }
//
// fn test () {
//     let rp = RootProperties{
//         num_clicks: PropertyX(5),
//         current_rotation: PropertyX(24.0),
//         deeper_struct: PropertyX(DeeperStruct{a: 5, b: "meow"})
//     };
//
//     rp.current_rotation.set(456.0);
//     rp.deeper_struct.get();
// }
//  OR -- perhaps invert RootProperties & Root, rewriting Root
//        to wrap .set and .get (thus making it more ergonomic to mutate values in vanilla userland codebase)
//        while generating RootProperties for use in the engine in the bare struct role that Root currently serves
// pub struct RootProperties {
//     pub num_clicks: PropertyX<i64>,
//     pub current_rotation: PropertyX<f64>,
//     pub deeper_struct: PropertyX<DeeperStruct>,
// }
//
// #[derive(Default)]
// pub struct RootPatch {
//     pub num_clicks: ExpressionOption<i64>,
//     pub current_rotation: ExpressionOption<f64>,
//     pub deeper_struct: ExpressionOption<DeeperStruct>,
// }
//
// impl Root {
//     fn apply_patch(patch: RootPatch) {
//         //convert patch to values, incl. handling of expressions/routing through exptable
//     }
// }
//
// impl RootPatch {
//     fn stack_and_override_with(&mut self, other_patch: RootPatch) {
//
//     }
// }

// pub enum ExpressionOption<T> {
//     None,
//     Some(T),
//     Expression(String) //TODO: should be a pointer to exptable
// }
//
// impl<T> Default for ExpressionOption<T> {
//     fn default() -> Self {
//         ExpressionOption::None
//     }
// }
// //
// impl From<SettingsLiteralBlockDefinition> for RootPatch {
//     fn from(cd: SettingsLiteralBlockDefinition) -> Self {
//         let mut ret = RootPatch::default();
//         cd.settings_key_value_pairs.iter().for_each(|kvp| {
//             match kvp.0.as_str() {
//                 "num_clicks" => {
//                     ret.num_clicks = match &kvp.1 {
//                         SettingsValueDefinition::Literal(val) => {
//                             ExpressionOption::Some(val)
//                         }
//                         SettingsValueDefinition::Expression(_) => {
//                             //TODO: parse expression, register in exptable,
//                             //and store a pointer here
//                             ExpressionOption::Expression("TODO-ADDRESS".to_string())
//                         }
//                         _ => {panic!("Type mismatch")}
//                     };
//                 },
//                 "current_rotation" => {
//
//                 },
//                 "deeper_struct" => {
//
//                 },
//                 _ => {
//                     panic!("Unexpected property applied to Root: {}", kvp.0);
//                 }
//             }
//         });
//         ret
//     }
// }

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

impl Root {
    pub fn new() -> Self {
        Self {
            //Default values.  Could shorthand this into a macro via PAXEL
            num_clicks: 0,
            current_rotation:0.0,
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
