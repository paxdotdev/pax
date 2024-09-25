use pax_engine::api::*;
use pax_engine::*;
use pax_std::*;

#[pax]
#[engine_import_path("pax_engine")]
#[file("glass/mouse_over_intents.pax")]
pub struct MouseOverIntents {
    pub intents: Property<Vec<IntentDef>>,
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct IntentDef {
    pub transform: Transform2D,
    pub stroke: Color,
    pub fill: Color,
    pub stroke_width_pixels: f64,
    pub width: f64,
    pub height: f64,
}

#[allow(unused)]
impl MouseOverIntents {
    pub fn on_mount(&mut self, ctx: &NodeContext) {}
}
