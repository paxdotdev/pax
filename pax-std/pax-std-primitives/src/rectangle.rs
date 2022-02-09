
use kurbo::{BezPath, Rect};
use piet::{RenderContext, StrokeStyle};

use pax_core::{Color, RenderNode, ComputableTransform, RenderNodePtrList, RenderTreeContext, HostPlatformContext, StrokeInstance, ComputableProperty};
use std::str::FromStr;
use std::cell::RefCell;
use std::rc::Rc;
use pax_core::pax_properties_coproduct::PropertiesCoproduct;
use pax_runtime_api::{Property, PropertyLiteral, Size, Transform, Size2D};

/// A basic 2D vector rectangle, drawn to fill the bounds specified
/// by `size`, transformed by `transform`
///
/// maybe #[pax primitive]
pub struct RectangleInstance {
    pub transform: Rc<RefCell<Box<dyn Property<Transform>>>>,
    pub properties: Rc<RefCell<PropertiesCoproduct>>,
    pub size: Rc<RefCell<[Box<dyn Property<Size>>; 2]>>,
}


pub struct RectangleProperties {
    pub stroke: Box<dyn Property<pax_std::types::Stroke>>,
    pub fill: Box<dyn Property<pax_std::types::Color>>,
}


impl RectangleInstance {
    pub fn instantiate(properties: PropertiesCoproduct, transform: Box<dyn Property<Transform>>, size: [Box<dyn Property<Size>>;2]) -> Rc<RefCell<dyn RenderNode>> {
        match &properties {
            PropertiesCoproduct::Rectangle(cast_properties) => {
                Rc::new(RefCell::new(RectangleInstance {
                    transform: Rc::new(RefCell::new(transform)),
                    properties: Rc::new(RefCell::new(properties)),
                    size: Rc::new(RefCell::new(size))
                }))
            },
            _ => {
                panic!("Wrong properties type received while instantiating Rectangle");
            }
        }
    }
}






//Generate via #[pax]



#[cfg(feature="parser")]
use pax_message::ComponentDefinition;
#[cfg(feature="parser")]
use parser;
#[cfg(feature="parser")]
use std::collections::HashSet;
#[cfg(feature="parser")]
use std::{env, fs};
use std::borrow::BorrowMut;
use std::ops::Deref;
#[cfg(feature="parser")]
use std::path::{Path, PathBuf};
use pax_std::types::ColorVariant;
#[cfg(feature="parser")]
use parser::ManifestContext;

#[cfg(feature="parser")]
lazy_static! {
    static ref source_id : String = parser::get_uuid();
}
#[cfg(feature="parser")]
//GENERATE pascal_identifier
impl RectangleInstance {
    pub fn parse_to_manifest(mut ctx: ManifestContext) -> (ManifestContext, String) {

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
                let PASCAL_IDENTIFIER = "Rectangle";
                let component_definition_for_this_file = parser::handle_primitive(PASCAL_IDENTIFIER, module_path!(), &source_id as &str);
                ctx.component_definitions.push(component_definition_for_this_file);
                //GENERATE:
                //Leaf node; no template, no pax file, no children to generate

                (ctx, source_id.to_string())
            },
            _ => {(ctx, source_id.to_string())} //early return; this file has already been parsed
        }

    }
}


impl RenderNode for RectangleInstance {
    fn get_rendering_children(&self) -> RenderNodePtrList {
        Rc::new(RefCell::new(vec![]))
    }
    fn get_size(&self) -> Option<Size2D> { Some(Rc::clone(&self.size)) }
    fn get_transform(&mut self) -> Rc<RefCell<Transform>> { Rc::clone(&self.transform) }
    fn compute_properties(&mut self, rtc: &mut RenderTreeContext) {
        let mut properties = &mut *self.properties.as_ref().borrow_mut();
        match properties {
            PropertiesCoproduct::Rectangle(properties_cast) => {
                properties_cast.stroke.compute_in_place(rtc);
                properties_cast.fill.compute_in_place(rtc);
            },
            _=>{},
        }
        let mut transform_borrowed = (**self.transform.borrow_mut()).borrow_mut();
        transform_borrowed.compute_in_place(rtc);

        let mut size_borrowed = (**self.size.borrow_mut()).borrow_mut();

        size_borrowed[0].compute_in_place(rtc);
        size_borrowed[1].compute_in_place(rtc);

    }
    fn render(&self, rtc: &mut RenderTreeContext, hpc: &mut HostPlatformContext) {
        let transform = rtc.transform;
        let bounding_dimens = rtc.bounds;
        let width: f64 =  bounding_dimens.0;
        let height: f64 =  bounding_dimens.1;

        match self.properties.borrow().deref() {
            PropertiesCoproduct::Rectangle(properties) => {
                let properties_color = properties.fill.get();
                let color = match properties_color.color_variant {
                    ColorVariant::Hlca(slice) => {
                        Color::hlca(slice[0], slice[1], slice[2], slice[3])
                    }
                    ColorVariant::Rgba(slice) => {
                        Color::rgba(slice[0], slice[1], slice[2], slice[3])
                    }
                };


                let mut bez_path = BezPath::new();
                bez_path.move_to((0.0, 0.0));
                bez_path.line_to((width , 0.0));
                bez_path.line_to((width , height ));
                bez_path.line_to((0.0, height));
                bez_path.line_to((0.0,0.0));
                bez_path.close_path();

                let transformed_bez_path = transform * bez_path;
                let duplicate_transformed_bez_path = transformed_bez_path.clone();

                let color = properties.fill.get().to_piet_color();
                hpc.drawing_context.fill(transformed_bez_path, &color);
                hpc.drawing_context.stroke(duplicate_transformed_bez_path, &properties.stroke.get().color.to_piet_color(), *&properties.stroke.get().width);
            },
            _=>{unreachable!()}
        };

    }
}
//
// #[cfg(feature="designtime")]
// lazy_static! {
//     static ref RECTANGLE_PROPERTIES_MANIFEST: Vec<(&'static str, &'static str)> = {
//         vec![
//             ("transform", "Transform"),
//             ("size", "Size2D"),
//             ("stroke", "Stroke"),
//             ("fill", "Color"),
//         ]
//     };
// }
//
// #[cfg(feature="designtime")]
// impl Manifestable for RectangleProperties {
//     fn get_type_identifier() -> &'static str {
//         &"Rectangle"
//     }
//     fn get_manifest() -> &'static Vec<(&'static str, &'static str)> {
//         RECTANGLE_PROPERTIES_MANIFEST.as_ref()
//     }
// }
//
// #[cfg(feature="designtime")]
// impl Patchable<RectanglePropertiesPatch> for RectangleProperties {
//     fn patch(&mut self, patch: RectanglePropertiesPatch) {
//         if let Some(p) = patch.transform {
//             self.transform = Rc::clone(&p);
//         }
//         if let Some(p) = patch.size {
//             self.size = Rc::clone(&p);
//         }
//         if let Some(p) = patch.stroke {
//             self.stroke = p;
//         }
//         if let Some(p) = patch.fill {
//             self.fill = p;
//         }
//     }
// }
//
// #[cfg(feature="designtime")]
// pub struct RectanglePropertiesPatch {
//     pub size: Option<Size2D>,
//     pub transform: Option<Rc<RefCell<Transform>>>,
//     pub stroke: Option<Stroke>,
//     pub fill: Option<Box<dyn Property<Color>>>,
// }
//
// #[cfg(feature="designtime")]
// impl Default for RectanglePropertiesPatch {
//     fn default() -> Self {
//         RectanglePropertiesPatch {
//             transform: None,
//             fill: None,
//             size: None,
//             stroke: None,
//         }
//     }
// }
//
// #[cfg(feature="designtime")]
// impl FromStr for RectanglePropertiesPatch {
//     type Err = ();
//
//     fn from_str(_: &str) -> Result<Self, Self::Err> {
//         todo!()
//     }
// }
//
