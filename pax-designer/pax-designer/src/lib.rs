#![allow(unused_imports)]

use crate::math::coordinate_spaces::{self, World};
use model::{
    action::{Action, ActionContext, CanUndo},
    ProjectMode,
};
use pax_engine::api::*;
use pax_engine::math::Point2;
use pax_engine::*;

pub mod context_menu;
pub mod controls;
pub mod glass;
use crate::context_menu::DesignerContextMenu;
use crate::controls::Controls;
use crate::glass::Glass;
use designer_project::Example;
use pax_std::primitives::*;
use project_mode_toggle::ProjectModeToggle;

pub mod llm_interface;
pub mod math;
pub mod model;
pub mod project_mode_toggle;

use llm_interface::LLMInterface;

pub const USERLAND_PROJECT_ID: &'static str = "userland_project";
pub const DESIGNER_GLASS_ID: &'static str = "designer_glass";

#[pax]
#[main]
#[file("lib.pax")]
pub struct PaxDesigner {
    pub transform2d: Property<Transform2D>,
    pub stage: Property<StageInfo>,
    pub glass_active: Property<bool>,
    pub interaction_mode: Property<bool>,
}

pub enum ProjectMsg {
    SetMode(ProjectMode),
}

impl Action for ProjectMsg {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> anyhow::Result<CanUndo> {
        match *self {
            ProjectMsg::SetMode(mode) => ctx.app_state.project_mode = mode,
        }
        Ok(CanUndo::No)
    }
}

impl PaxDesigner {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.glass_active.set(true);
        self.stage.set(StageInfo {
            width: 2561 / 2,
            height: 1440 / 2,
        });
    }

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        model::read_app_state(|app_state| {
            // set transform to world transform
            let world_to_glass = app_state.glass_to_world_transform.inverse();
            let t = world_to_glass.get_translation();
            let s = world_to_glass.get_scale();
            self.transform2d.set(
                Transform2D::scale(
                    Size::Percent((100.0 * s.x).into()),
                    Size::Percent((100.0 * s.y).into()),
                ) * Transform2D::translate(Size::Pixels((t.x).into()), Size::Pixels((t.y).into())),
            );

            // set app mode
            let editing = match app_state.project_mode {
                ProjectMode::Edit => true,
                ProjectMode::Playing => false,
            };
            self.glass_active.set(editing);
            self.interaction_mode.set(!editing);
        });
    }
}

#[pax]
pub struct StageInfo {
    pub width: u32,
    pub height: u32,
}
