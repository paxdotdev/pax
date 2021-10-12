use std::cell::RefCell;
use std::rc::Rc;

use kurbo::BezPath;
use piet::RenderContext;
use piet_web::WebRenderContext;

use serde::{Serialize};

use crate::{Color, PropertyValue, RenderNode, RenderNodePtrList, RenderTreeContext, Size2D, Stroke, Transform, Affine, HostPlatformContext};
use wasm_bindgen::JsValue;


pub struct Text {
    pub content: Box<dyn PropertyValue<String>>,
    pub transform: Rc<RefCell<Transform>>,
    pub size: Size2D,
}

/// Simplified data structure used to serialize data for mixed-mode rendering
#[derive(Serialize)]
pub struct TextMessage {
    content: String, //TODO: skip wasm-bindgen
    transform: [f64; 12],
    size: (f64, f64),
}

impl TextMessage {

}

/// Types that implement RenderMessage can be serialized and added to the
/// RenderMessageQueue to communicate to the host platform how to render the native
/// component of a RenderNode
pub trait RenderMessage {}

impl RenderMessage for TextMessage {}

///TODO:  does this need to be a disc. union instead of a trait?
///       if disc. union we also need to communicate which of the
///       subtypes each message is, to facilitate unboxing on the other side

impl RenderNode for Text {
    fn get_rendering_children(&self) -> RenderNodePtrList {
        Rc::new(RefCell::new(vec![]))
    }
    fn get_size(&self) -> Option<Size2D> { Some(Rc::clone(&self.size)) }
    fn get_transform(&mut self) -> Rc<RefCell<Transform>> { Rc::clone(&self.transform) }
    fn compute_properties(&mut self, rtc: &mut RenderTreeContext) {
        self.size.borrow_mut().0.compute_in_place(rtc);
        self.size.borrow_mut().1.compute_in_place(rtc);
        self.transform.borrow_mut().compute_in_place(rtc);
    }
    fn render(&self, rtc: &mut RenderTreeContext, hpc: &mut HostPlatformContext) {
        //TODO:
        // attach a TextMessage to the HostPlatformContext

        let message = TextMessage {
            content: "A thing of beauty is a joy forever".to_string(),
            transform: [0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0],
            size: (550.0, 400.0)
        };
        // message.serialize(&hpc.serializer);
        hpc.render_message_queue.push(JsValue::from_serde(&message).unwrap())
    }
}
