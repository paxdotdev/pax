use pax_engine::api::*;
use pax_engine::math::{Generic, Transform2, TransformParts, Vector2};
use pax_engine::*;
use pax_std::*;

use crate::math::coordinate_spaces::Glass;

#[pax]
#[engine_import_path("pax_engine")]
#[file("glass/intent.pax")]
pub struct Intent {
    pub data: Property<IntentDef>,
}

impl Intent {}

#[pax]
#[engine_import_path("pax_engine")]
pub struct IntentDef {
    pub intent_x: f64,
    pub intent_y: f64,
    pub intent_rotation: f64,
    pub intent_width: f64,
    pub intent_height: f64,
    pub intent_stroke: Color,
    pub intent_fill: Color,
    pub intent_stroke_width_pixels: f64,
}

impl IntentDef {
    pub fn new(
        transform: Transform2<NodeLocal, Glass>,
        fill: Color,
        stroke: Option<(f64, Color)>,
    ) -> Self {
        let parts: TransformParts = transform.into();
        let (stroke_width_pixels, stroke) = stroke.unwrap_or((0.0, Color::BLACK));
        Self {
            intent_x: parts.origin.x,
            intent_y: parts.origin.y,
            intent_rotation: parts.rotation,
            intent_width: parts.scale.x,
            intent_height: parts.scale.y,
            intent_fill: fill,
            intent_stroke: stroke,
            intent_stroke_width_pixels: stroke_width_pixels,
        }
    }
}
