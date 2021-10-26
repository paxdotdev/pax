use std::cell::RefCell;
use std::rc::Rc;

use kurbo::BezPath;
use piet::RenderContext;
use piet_web::WebRenderContext;

use crate::{Color, PropertyValue, RenderNode, RenderNodePtrList, RenderTreeContext, Size2D, Stroke, Transform, HostPlatformContext};

/// A basic 2D vector rectangle, drawn to fill the bounds specified
/// by `size`, transformed by `transform`
pub struct Rectangle {
    pub properties: Rc<RefCell<RectangleProperties>>
}

pub struct RectangleProperties {
    pub size: Size2D,
    pub transform: Rc<RefCell<Transform>>,
    pub stroke: Stroke,
    pub fill: Box<dyn PropertyValue<Color>>,
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



