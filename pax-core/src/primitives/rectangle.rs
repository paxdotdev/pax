

use std::cell::RefCell;
use std::rc::Rc;

use kurbo::BezPath;
use piet::RenderContext;


use crate::{Color, Property, RenderNode, RenderNodePtrList, RenderTreeContext, Size2D, Stroke, Transform, HostPlatformContext};
use std::str::FromStr;
use crate::metaruntime::{Manifestable, Patchable};


/// Properties refactor:
///     - expressions.rs -> properties.rs
///     - use a coproduct-style Property super-object
///         - TODO:  how to support `setting` values in Actions?
///                 set_timeline() // Probably future?  Will there be a Timeline Expression Language too?  Or a timeline module added to Exp.Lang?
///                 set_expression() // Probably future — interesting question of runtime vs. pre-compiled Expression Language
///                 set() // T


/// A basic 2D vector rectangle, drawn to fill the bounds specified
/// by `size`, transformed by `transform`
pub struct Rectangle {
    pub properties: Rc<RefCell<RectangleProperties>>
}

pub struct RectangleProperties {
    pub size: Size2D,
    pub transform: Rc<RefCell<Transform>>,
    pub stroke: Stroke,
    pub fill: Box<dyn Property<Color>>,
}

impl RenderNode for Rectangle {
    fn get_rendering_children(&self) -> RenderNodePtrList {
        Rc::new(RefCell::new(vec![]))
    }
    fn get_size(&self) -> Option<Size2D> { Some(Rc::clone(&self.properties.borrow().size)) }
    fn get_transform(&mut self) -> Rc<RefCell<Transform>> { Rc::clone(&self.properties.borrow_mut().transform) }
    fn compute_properties(&mut self, rtc: &mut RenderTreeContext) {
        self.properties.borrow_mut().size.borrow_mut().0.compute_in_place(rtc);
        self.properties.borrow_mut().size.borrow_mut().1.compute_in_place(rtc);
        self.properties.borrow_mut().fill.compute_in_place(rtc);
        self.properties.borrow_mut().transform.borrow_mut().compute_in_place(rtc);
    }
    fn render(&self, rtc: &mut RenderTreeContext, hpc: &mut HostPlatformContext) {
        let transform = rtc.transform;
        let bounding_dimens = rtc.bounds;
        let width: f64 =  bounding_dimens.0;
        let height: f64 =  bounding_dimens.1;

        let properties_borrowed = &self.properties.borrow();
        let fill: &Color = properties_borrowed.fill.read();

        let mut bez_path = BezPath::new();
        bez_path.move_to((0.0, 0.0));
        bez_path.line_to((width , 0.0));
        bez_path.line_to((width , height ));
        bez_path.line_to((0.0, height));
        bez_path.line_to((0.0,0.0));
        bez_path.close_path();

        let transformed_bez_path = transform * bez_path;
        let duplicate_transformed_bez_path = transformed_bez_path.clone();

        hpc.drawing_context.fill(transformed_bez_path, fill);
        hpc.drawing_context.stroke(duplicate_transformed_bez_path, &properties_borrowed.stroke.color, properties_borrowed.stroke.width);
    }
}

#[cfg(feature="metaruntime")]
lazy_static! {
    static ref RECTANGLE_PROPERTIES_MANIFEST: Vec<(&'static str, &'static str)> = {
        vec![
            ("transform", "Transform"),
            ("size", "Size2D"),
            ("stroke", "Stroke"),
            ("fill", "Color"),
        ]
    };
}

#[cfg(feature="metaruntime")]
impl Manifestable for RectangleProperties {
    fn get_type_identifier() -> &'static str {
        &"Rectangle"
    }
    fn get_manifest() -> &'static Vec<(&'static str, &'static str)> {
        RECTANGLE_PROPERTIES_MANIFEST.as_ref()
    }
}

#[cfg(feature="metaruntime")]
impl Patchable<RectanglePropertiesPatch> for RectangleProperties {
    fn patch(&mut self, patch: RectanglePropertiesPatch) {
        if let Some(p) = patch.transform {
            self.transform = Rc::clone(&p);
        }
        if let Some(p) = patch.size {
            self.size = Rc::clone(&p);
        }
        if let Some(p) = patch.stroke {
            self.stroke = p;
        }
        if let Some(p) = patch.fill {
            self.fill = p;
        }
    }
}

#[cfg(feature="metaruntime")]
pub struct RectanglePropertiesPatch {
    pub size: Option<Size2D>,
    pub transform: Option<Rc<RefCell<Transform>>>,
    pub stroke: Option<Stroke>,
    pub fill: Option<Box<dyn Property<Color>>>,
}

#[cfg(feature="metaruntime")]
impl Default for RectanglePropertiesPatch {
    fn default() -> Self {
        RectanglePropertiesPatch {
            transform: None,
            fill: None,
            size: None,
            stroke: None,
        }
    }
}

#[cfg(feature="metaruntime")]
impl FromStr for RectanglePropertiesPatch {
    type Err = ();

    fn from_str(_: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

