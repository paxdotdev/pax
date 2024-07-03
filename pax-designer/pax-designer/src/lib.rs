#![allow(unused_imports)]

use anyhow::anyhow;
use pax_engine::{api::*, math::Point2, *};
use pax_manifest::TypeId;
use pax_std::primitives::*;
use std::sync::Mutex;

use crate::math::coordinate_spaces::{self, World};
use model::{
    action::{Action, ActionContext, CanUndo},
    ProjectMode, StageInfo,
};

pub mod context_menu;
pub mod glass;

pub mod controls;
pub mod llm_interface;
pub mod math;
pub mod model;
pub mod project_mode_toggle;

use context_menu::DesignerContextMenu;
use controls::Controls;
use glass::Glass;
use llm_interface::LLMInterface;
use project_mode_toggle::ProjectModeToggle;

use designer_project::Example;

pub const ROOT_PROJECT_ID: &str = "userland_project";
pub const DESIGNER_GLASS_ID: &str = "designer_glass";
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

impl PaxDesigner {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        model::init_model(ctx);
        model::read_app_state(|app_state| {
            self.bind_stage_property(&app_state);
            self.bind_transform2d_property(&app_state);
            self.bind_glass_active_property(&app_state);
            self.bind_interaction_mode_property();
        });
    }

    fn bind_stage_property(&mut self, app_state: &model::AppState) {
        let stage = app_state.stage.clone();
        let deps = [stage.untyped()];
        self.stage
            .replace_with(Property::computed(move || stage.get(), &deps));
    }

    fn bind_transform2d_property(&mut self, app_state: &model::AppState) {
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
                ) * Transform2D::translate(Size::Pixels(t.x.into()), Size::Pixels(t.y.into()))
            },
            &deps,
        ));
    }

    fn bind_glass_active_property(&mut self, app_state: &model::AppState) {
        let proj_mode = app_state.project_mode.clone();
        let deps = [proj_mode.untyped()];
        self.glass_active.replace_with(Property::computed(
            move || matches!(proj_mode.get(), ProjectMode::Edit),
            &deps,
        ));
    }

    fn bind_interaction_mode_property(&mut self) {
        let glass_active = self.glass_active.clone();
        let deps = [glass_active.untyped()];
        self.interaction_mode
            .replace_with(Property::computed(move || !glass_active.get(), &deps));
    }
}

pub struct ProjectMsg(ProjectMode);

impl Action for ProjectMsg {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> anyhow::Result<CanUndo> {
        ctx.app_state.project_mode.set(self.0);
        Ok(CanUndo::No)
    }
}

pub struct SetStage(pub StageInfo);

impl Action for SetStage {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> anyhow::Result<CanUndo> {
        ctx.app_state.stage.set(self.0);
        Ok(CanUndo::No)
    }
}
