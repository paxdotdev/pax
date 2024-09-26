use std::f64::consts::PI;

use pax_engine::api::*;
use pax_engine::math::{Generic, Transform2, TransformParts, Vector2};
use pax_engine::*;
use pax_std::*;

pub mod intent;
pub use intent::Intent;

use crate::math::coordinate_spaces::Glass;
use crate::model::{self, GlassNode};

#[pax]
#[engine_import_path("pax_engine")]
#[file("glass/mouse_over_intents.pax")]
pub struct MouseOverIntents {
    pub intents: Property<Vec<IntentDef>>,
}

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

#[allow(unused)]
impl MouseOverIntents {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let (glass_transform, intent_objects) = model::read_app_state_with_derived(|_, derived| {
            (
                derived.to_glass_transform.clone(),
                derived.intent_objects.clone(),
            )
        });
        let deps = [intent_objects.untyped(), glass_transform.untyped()];
        self.intents.replace_with(Property::computed(
            move || {
                intent_objects.read(|intent_objects| {
                    intent_objects
                        .iter()
                        .map(|node| {
                            let node = GlassNode::new(&node, &glass_transform.get());
                            let t_and_b = node.transform_and_bounds.get();
                            IntentDef::new(
                                t_and_b.as_transform(),
                                Color::rgba(255.into(), 0.into(), 0.into(), 100.into()),
                                Color::BLACK,
                            )
                        })
                        .collect()
                })
            },
            &deps,
        ));
    }
}
