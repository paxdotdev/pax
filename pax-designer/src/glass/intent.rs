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
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
    pub width: f64,
    pub height: f64,
    pub stroke: Color,
    pub fill: Color,
    pub stroke_width_pixels: f64,
}

impl IntentDef {
    pub fn new(transform: Transform2<NodeLocal, Glass>, fill: Color, stroke: Color) -> Self {
        let parts: TransformParts = transform.into();
        Self {
            x: parts.origin.x,
            y: parts.origin.y,
            rotation: parts.rotation,
            width: parts.scale.x,
            height: parts.scale.y,
            fill,
            stroke,
            stroke_width_pixels: 1.0,
        }
    }
}
