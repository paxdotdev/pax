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
    pub id: String,
}

/// Simplified data structure used to serialize data for mixed-mode rendering
#[derive(Serialize)]
pub struct TextMessage<'a> {
    kind: MessageKind,
    id: String,
    content: Option<&'a String>,
    transform: Option<[f64; 6]>,
    bounds: Option<(f64, f64)>,
}

impl<'a> TextMessage<'a> {
    pub fn empty(kind: MessageKind, id: String) -> Self {
        TextMessage {
            kind,
            id,
            content: None,
            transform: None,
            bounds: None
        }
    }
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
        let update_message = TextMessage::empty(MessageKind::TextMessage, self.id.clone());
        self.size.borrow_mut().0.compute_in_place(rtc);
        self.size.borrow_mut().1.compute_in_place(rtc);
        self.transform.borrow_mut().compute_in_place(rtc);
    }
    fn render(&self, rtc: &mut RenderTreeContext, hpc: &mut HostPlatformContext) {

        let message = TextMessage {
            kind: MessageKind::TextMessage,
            id: self.id.clone(), //TODO: can we do better than cloning here?
            content: Some(self.content.read()), //TODO: can we do better than cloning here?
            transform: Some((rtc.transform * *self.transform.borrow().get_cached_computed_value()).as_coeffs()),
            bounds: Some(self.get_size_calc(rtc.bounds)),
        };

        hpc.render_message_queue.push(JsValue::from_serde(&message).unwrap())
    }
}
