use kurbo::{BezPath};
use piet::{RenderContext};

use pax_std::primitives::{Rectangle};
use pax_std::types::ColorVariant;
use pax_core::{Color, RenderNode, RenderNodePtrList, RenderTreeContext, ExpressionContext, InstanceRegistry, HandlerRegistry, InstantiationArgs, RenderNodePtr};
use pax_core::pax_properties_coproduct::{PropertiesCoproduct, TypesCoproduct};
use pax_runtime_api::{PropertyInstance, PropertyLiteral, Size, Transform2D, Size2D, ArgsCoproduct};

use std::str::FromStr;
use std::cell::RefCell;
use std::rc::Rc;


/// A basic 2D vector rectangle, drawn to fill the bounds specified
/// by `size`, transformed by `transform`
///
/// maybe #[pax primitive]
pub struct RectangleInstance {
    pub handler_registry: Option<Rc<RefCell<HandlerRegistry>>>,
    pub instance_id: u64,
    pub properties: Rc<RefCell<Rectangle>>,

    pub size: Rc<RefCell<[Box<dyn PropertyInstance<Size>>; 2]>>,
    pub transform: Rc<RefCell<dyn PropertyInstance<Transform2D>>>,
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



impl<R: 'static + RenderContext>  RenderNode<R> for RectangleInstance {
    fn get_instance_id(&self) -> u64 {
        self.instance_id
    }
    
    fn get_rendering_children(&self) -> RenderNodePtrList<R> {
        Rc::new(RefCell::new(vec![]))
    }

    fn instantiate(args: InstantiationArgs<R>) -> Rc<RefCell<Self>> where Self: Sized {
        let properties = if let PropertiesCoproduct::Rectangle(p) = args.properties { p } else {unreachable!("Wrong properties type")};

        let mut instance_registry = (*args.instance_registry).borrow_mut();
        let instance_id = instance_registry.mint_id();
        let ret = Rc::new(RefCell::new(RectangleInstance {
            instance_id,
            transform: args.transform,
            properties: Rc::new(RefCell::new(properties)),
            size: Rc::new(RefCell::new(args.size.expect("Rectangle requires a size"))),
            handler_registry: args.handler_registry,
        }));

        instance_registry.register(instance_id, Rc::clone(&ret) as RenderNodePtr<R>);
        ret
    }

    fn get_handler_registry(&self) -> Option<Rc<RefCell<HandlerRegistry>>> {
        match &self.handler_registry {
            Some(registry) => {
                Some(Rc::clone(registry))
            },
            _ => {None}
        }
    }
    fn get_size(&self) -> Option<Size2D> { Some(Rc::clone(&self.size)) }
    fn get_transform(&mut self) -> Rc<RefCell<dyn PropertyInstance<Transform2D>>> { Rc::clone(&self.transform) }
    fn compute_properties(&mut self, rtc: &mut RenderTreeContext<R>) {
        let mut properties = &mut *self.properties.as_ref().borrow_mut();

        if let Some(stroke) = rtc.compute_vtable_value(properties.stroke._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Stroke(v) = stroke { v } else { unreachable!() };
            properties.stroke.set(new_value);
        }

        if let Some(fill) = rtc.compute_vtable_value(properties.fill._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Color(v) = fill { v } else { unreachable!() };
            properties.fill.set(new_value);
        }

        let mut size = &mut *self.size.as_ref().borrow_mut();

        if let Some(new_size) = rtc.compute_vtable_value(size[0]._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Size(v) = new_size { v } else { unreachable!() };
            size[0].set(new_value);
        }

        if let Some(new_size) = rtc.compute_vtable_value(size[1]._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Size(v) = new_size { v } else { unreachable!() };
            size[1].set(new_value);
        }

        let mut transform = &mut *self.transform.as_ref().borrow_mut();
        if let Some(new_transform) = rtc.compute_vtable_value(transform._get_vtable_id()) {
            let new_value = if let TypesCoproduct::Transform2D(v) = new_transform { v } else { unreachable!() };
            transform.set(new_value);
        }

    }
    fn handle_render(&self, rtc: &mut RenderTreeContext<R>, rc: &mut R) {
        let transform = rtc.transform;
        let bounding_dimens = rtc.bounds;
        let width: f64 =  bounding_dimens.0;
        let height: f64 =  bounding_dimens.1;

        let properties = (*self.properties).borrow();

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
        rc.fill(transformed_bez_path, &color);
        rc.stroke(duplicate_transformed_bez_path, &properties.stroke.get().color.get().to_piet_color(), **&properties.stroke.get().width.get());


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
