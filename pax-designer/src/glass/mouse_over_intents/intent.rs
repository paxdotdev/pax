use super::IntentDef;
use pax_engine::api::*;
use pax_engine::math::{Generic, Transform2, TransformParts, Vector2};
use pax_engine::*;
use pax_std::*;

#[pax]
#[engine_import_path("pax_engine")]
#[file("glass/mouse_over_intents/intent.pax")]
pub struct Intent {
    pub data: Property<IntentDef>,
}

impl Intent {
    pub fn on_mouse_up(&mut self, _ctx: &NodeContext, _event: Event<MouseUp>) {
        log::debug!("on mouse up on intent");
    }
}
