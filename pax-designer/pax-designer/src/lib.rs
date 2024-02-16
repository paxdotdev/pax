#![allow(unused_imports)]

use model::math::coordinate_spaces::{self, Window};
use pax_engine::api::*;
use pax_engine::*;

pub mod controls;
pub mod designtime_component_viewer;
pub mod glass;
use crate::controls::Controls;
use crate::designtime_component_viewer::DesigntimeComponentViewer;
use crate::glass::Glass;
use designer_project::Example;
use pax_std::primitives::{Group, Rectangle};

pub mod model;

pub const USERLAND_PROJECT_ID: &'static str = "userland_project";
pub const DESIGNER_GLASS_ID: &'static str = "designer_glass";

#[pax]
#[main]
#[file("lib.pax")]
pub struct PaxDesigner {
    pub world_transform_x: Property<f64>,
    pub world_transform_y: Property<f64>,
}

impl PaxDesigner {
    pub fn tick(&mut self, ctx: &NodeContext) {
        model::read_app_state(|app_state| {
            let world = app_state.glass_to_world_transform.get_translation();
            self.world_transform_x.set(-world.x);
            self.world_transform_y.set(-world.y);
        });
    }
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {}

    pub fn click_test(&mut self, _ctx: &NodeContext, args: ArgsClick) {}
}
