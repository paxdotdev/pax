#![allow(unused_imports)]

use model::math::coordinate_spaces::{self, World};
use pax_engine::api::*;
use pax_engine::math::Point2;
use pax_engine::*;

pub mod controls;
pub mod designtime_component_viewer;
pub mod glass;
use crate::designtime_component_viewer::DesigntimeComponentViewer;
use crate::glass::Glass;
use crate::{controls::Controls, model::input::Dir};
use designer_project::Example;
use pax_std::primitives::{Group, Rectangle};

use pax_engine::layout::ComputableTransform;
pub mod model;

pub const USERLAND_PROJECT_ID: &'static str = "userland_project";
pub const DESIGNER_GLASS_ID: &'static str = "designer_glass";

#[pax]
#[main]
#[file("lib.pax")]
pub struct PaxDesigner {
    pub transform2d: Property<Transform2D>,
    pub debug_p_x: Property<f64>,
    pub debug_p_y: Property<f64>,
}

impl PaxDesigner {
    pub fn tick(&mut self, ctx: &NodeContext) {
        model::read_app_state(|app_state| {
            let world_to_glass = app_state.glass_to_world_transform.inverse();
            let t = world_to_glass.get_translation();
            let s = world_to_glass.get_scale();
            self.transform2d.set(
                Transform2D::scale(
                    Size::Percent((100.0 * s.x).into()),
                    Size::Percent((100.0 * s.y).into()),
                ) * Transform2D::translate(Size::Pixels((t.x).into()), Size::Pixels((t.y).into())),
            );
        });
    }
    pub fn handle_key_down(&mut self, ctx: &NodeContext, args: ArgsKeyDown) {
        let res = model::process_keyboard_input(ctx, Dir::Down, args.keyboard.key);
        if let Err(e) = res {
            pax_engine::log::warn!("{}", e);
        }
    }

    pub fn handle_key_up(&mut self, ctx: &NodeContext, args: ArgsKeyUp) {
        let res = model::process_keyboard_input(ctx, Dir::Up, args.keyboard.key);
        if let Err(e) = res {
            pax_engine::log::warn!("{}", e);
        }
    }

    pub fn handle_mount(&mut self, _ctx: &NodeContext) {}

    pub fn click_test(&mut self, _ctx: &NodeContext, args: ArgsClick) {}
}
