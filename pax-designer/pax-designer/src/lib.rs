#![allow(unused_imports)]

use std::sync::Mutex;

use crate::math::coordinate_spaces::{self, World};
use anyhow::anyhow;
use model::{
    action::{Action, ActionContext, CanUndo},
    ProjectMode, StageInfo,
};
use pax_engine::api::*;
use pax_engine::math::Point2;
use pax_engine::*;
use pax_manifest::TypeId;

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

pub const ROOT_PROJECT_ID: &'static str = "userland_project";
pub const DESIGNER_GLASS_ID: &'static str = "designer_glass";
pub const USER_PROJ_ROOT_IMPORT_PATH: &str = "pax_designer::pax_reexports::designer_project";
pub const USER_PROJ_ROOT_COMPONENT: &str = "Example";

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
            ProjectMsg::SetMode(mode) => ctx.app_state.project_mode.set(mode),
        }
        Ok(CanUndo::No)
    }
}

impl PaxDesigner {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        model::init_model(ctx);

        model::read_app_state(|app_state| {
            let stage = app_state.stage.clone();
            let deps = [stage.untyped()];
            self.stage
                .replace_with(Property::computed(move || stage.get(), &deps));
            let glass_to_world = app_state.glass_to_world_transform.clone();
            let deps = [glass_to_world.untyped()];
            self.transform2d.replace_with(Property::computed(
                move || {
                    let world_to_glass = glass_to_world.get().inverse();
                    let t = world_to_glass.get_translation();
                    let s = world_to_glass.get_scale();
                    Transform2D::scale(
                        Size::Percent((100.0 * s.x).into()),
                        Size::Percent((100.0 * s.y).into()),
                    ) * Transform2D::translate(
                        Size::Pixels((t.x).into()),
                        Size::Pixels((t.y).into()),
                    )
                },
                &deps,
            ));

            let proj_mode = app_state.project_mode.clone();
            let deps = [proj_mode.untyped()];
            self.glass_active.replace_with(Property::computed(
                move || match proj_mode.get() {
                    ProjectMode::Edit => true,
                    ProjectMode::Playing => false,
                },
                &deps,
            ));
            let glass_active = self.glass_active.clone();
            let deps = [glass_active.untyped()];
            self.interaction_mode
                .replace_with(Property::computed(move || !glass_active.get(), &deps));
        });
    }
}
pub struct SetStage(pub StageInfo);

impl Action for SetStage {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> anyhow::Result<CanUndo> {
        ctx.app_state.stage.set(self.0);
        Ok(CanUndo::No)
    }
}
