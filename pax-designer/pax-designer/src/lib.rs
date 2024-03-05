#![allow(unused_imports)]

use model::math::coordinate_spaces::{self, World};
use pax_engine::api::*;
use pax_engine::math::Point2;
use pax_engine::*;

pub mod context_menu;
pub mod controls;
pub mod designtime_component_viewer;
pub mod glass;
use crate::context_menu::ContextMenu;
use crate::controls::Controls;
use crate::designtime_component_viewer::DesigntimeComponentViewer;
use crate::glass::Glass;
use designer_project::Example;
use pax_std::primitives::{Group, Rectangle};

pub mod llm_interface;
pub mod math;
pub mod model;

use llm_interface::LLMInterface;

pub const USERLAND_PROJECT_ID: &'static str = "userland_project";
pub const DESIGNER_GLASS_ID: &'static str = "designer_glass";

#[pax]
#[main]
#[file("lib.pax")]
pub struct PaxDesigner {
    pub transform2d: Property<Transform2D>,
}

impl PaxDesigner {
    pub fn pre_render(&mut self, _ctx: &NodeContext) {
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
}
