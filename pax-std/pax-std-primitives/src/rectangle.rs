
use kurbo::{BezPath};
use piet::{RenderContext};

use pax_core::{Color, RenderNode, RenderNodePtrList, RenderTreeContext, HostPlatformContext, ExpressionContext, InstanceMap};
use std::str::FromStr;
use std::cell::RefCell;
use std::rc::Rc;
use pax_core::pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use pax_runtime_api::{Property, PropertyLiteral, Size, Transform, Size2D, ArgsCoproduct};

/// A basic 2D vector rectangle, drawn to fill the bounds specified
/// by `size`, transformed by `transform`
///
/// maybe #[pax primitive]
pub struct RectangleInstance {
    pub transform: Rc<RefCell<dyn Property<Transform>>>,
    pub properties: Rc<RefCell<PropertiesCoproduct>>,
    pub size: Rc<RefCell<[Box<dyn Property<Size>>; 2]>>,
}


pub struct RectangleProperties {
    pub stroke: Box<dyn Property<pax_std::types::StrokeProperties>>,
    pub fill: Box<dyn Property<pax_std::types::Color>>,
}


impl RectangleInstance {
    pub fn instantiate(instance_map: Rc<RefCell<InstanceMap>>, properties: PropertiesCoproduct, transform: Rc<RefCell<dyn Property<Transform>>>, size: [Box<dyn Property<Size>>;2]) -> Rc<RefCell<dyn RenderNode>> {

        match &properties {
            PropertiesCoproduct::Rectangle(cast_properties) => {
                let new_id = pax_runtime_api::generate_unique_id();
                let ret = Rc::new(RefCell::new(RectangleInstance {
                    transform,
                    properties: Rc::new(RefCell::new(properties)),
                    size: Rc::new(RefCell::new(size))
                }));

                (*instance_map).borrow_mut().insert(new_id, Rc::clone(&ret) as Rc<RefCell<dyn RenderNode>>);
                ret
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
use std::borrow::{Borrow, BorrowMut};
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


//what if PropertyLiteral/PropertyExpression were stored as an enum Property::Literal(literal value) or
//Property::Expression() instead of as traits — would this resolve the issue of
//can't call compute_in_place on property
//because it's only implemented for ComputedProperty
//even though we _know_ we will have a ComputedProperty,
//the compiler doesn't know this.
//`unsafe` may be one way out!
//




pub fn handle_properties_computation(id: &str, rtc: &mut RenderTreeContext) {
    //for each of Timelines and Expressions:
    //look up ID to see if present in hash
}

impl RenderNode for RectangleInstance {
    fn get_rendering_children(&self) -> RenderNodePtrList {
        Rc::new(RefCell::new(vec![]))
    }
    fn get_size(&self) -> Option<Size2D> { Some(Rc::clone(&self.size)) }
    fn get_transform(&mut self) -> Rc<RefCell<dyn Property<Transform>>> { Rc::clone(&self.transform) }
    fn compute_properties(&mut self, rtc: &mut RenderTreeContext) {
        let mut properties = &mut *self.properties.as_ref().borrow_mut();
        match properties {
            PropertiesCoproduct::Rectangle(properties_cast) => {
                let id = properties_cast.stroke.get_id();
                // if let Some(id) = id {
                //     if let Some(evaluator) = rtc.engine.expression_table.borrow().get(id) {
                //         let ec = ExpressionContext {
                //             engine: rtc.engine,
                //             stack_frame: Rc::clone(&(*rtc.runtime).borrow_mut().peek_stack_frame().unwrap())
                //         };
                //         let new_value = (**evaluator)(ec);
                //         if let TypesCoproduct::Transform(cast_new_value) = new_value {
                //             properties_cast.stroke.cache_value(cast_new_value)
                //         }
                //     }
                // }
                //
                // if let Some(id) = id { handle_properties_computation(id, rtc); }

                let id = properties_cast.fill.get_id();
                // if let Some(id) = id { handle_properties_computation(id, rtc); }
                //now that IDs are registered, need to dispatch
                //appropriate evaluators, passing value
                //back to property for storage
            },
            _=>{},
        }

        let mut transform_borrowed = (*self.transform).borrow_mut();
        let id = transform_borrowed.get_id();
        if let Some(id) = id {
            if let Some(evaluator) = rtc.engine.expression_table.borrow().get(id) {
                let ec = ExpressionContext {
                    engine: rtc.engine,
                    stack_frame: Rc::clone(&(*rtc.runtime).borrow_mut().peek_stack_frame().unwrap())
                };
                let new_value = (**evaluator)(ec);
                if let TypesCoproduct::Transform(cast_new_value) = new_value {
                    transform_borrowed.set(cast_new_value)
                }
            }
        }

        let mut size_borrowed = (*self.size).borrow_mut();

        size_borrowed[0].get_id();;
        size_borrowed[1].get_id();;

    }
    fn render(&self, rtc: &mut RenderTreeContext, hpc: &mut HostPlatformContext) {
        let transform = rtc.transform;
        let bounding_dimens = rtc.bounds;
        let width: f64 =  bounding_dimens.0;
        let height: f64 =  bounding_dimens.1;

        match (*self.properties).borrow().deref() {
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
                hpc.drawing_context.stroke(duplicate_transformed_bez_path, &properties.stroke.get().color.get().to_piet_color(), **&properties.stroke.get().width.get());
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
