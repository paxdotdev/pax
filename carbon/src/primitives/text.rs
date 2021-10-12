use std::cell::RefCell;
use std::rc::Rc;

use kurbo::BezPath;
use piet::RenderContext;
use piet_web::WebRenderContext;

use serde::{Serialize};

use crate::{Color, PropertyValue, RenderNode, RenderNodePtrList, RenderTreeContext, Size2D, Stroke, Transform, Affine, HostPlatformContext};
use wasm_bindgen::prelude::*;


pub struct Text {
    pub content: Box<dyn PropertyValue<String>>,
    pub transform: Rc<RefCell<Transform>>,
    pub size: Size2D,
    pub id: usize,
}

/// Simplified data structure used to serialize data for mixed-mode rendering
#[derive(Serialize)]
pub struct TextMessage {
    kind: MessageKind,
    content: String,
    transform: [f64; 6],
    size: (f64, f64),
    id: usize,
}

#[derive(Serialize)]
pub enum MessageKind {
    TextMessage,
}

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
            content: self.content.read().clone(), //TODO: can we do better than cloning here?
            transform: (rtc.transform * *self.transform.borrow().get_cached_computed_value()).as_coeffs(),
            size: self.get_size_calc(rtc.bounds),
            id: self.id,
            kind: MessageKind::TextMessage,
        };

        hpc.render_message_queue.push(JsValue::from_serde(&message).unwrap())
    }
}
