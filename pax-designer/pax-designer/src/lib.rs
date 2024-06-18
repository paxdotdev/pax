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
pub const USERLAND_EDIT_ID: &'static str = "userland_project_edit_root";
pub const DESIGNER_GLASS_ID: &'static str = "designer_glass";
pub const RUNNING_MODE_STAGE_GROUP: &'static str = "running_mode_stage_group";
pub const EDIT_MODE_STAGE_GROUP: &'static str = "edit_mode_stage_group";
pub const USER_PROJ_ROOT_IMPORT_PATH: &str =
    "pax_designer::pax_reexports::designer_project::userland";
pub const USER_PROJ_ROOT_COMPONENT: &str = "Userland";

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

        model::read_app_state_with_derived(|app_state, derived| {
            let stage = derived.stage.clone();
            // init stage to some reasonable size
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
        // TODO can't find expanded node for this since none exist?
        // // set running mode size
        // let uid = ctx
        //     .engine_context
        //     .get_nodes_by_id(RUNNING_MODE_STAGE_GROUP)
        //     .first()
        //     .cloned()
        //     .unwrap()
        //     .global_id()
        //     .unwrap();
        let mut dt = borrow_mut!(ctx.engine_context.designtime);
        // let mut builder = dt
        //     .get_orm_mut()
        //     .get_node(uid)
        //     .ok_or_else(|| anyhow!("cound't find node"))?;
        // builder.set_property("scroll_height", &format!("{}px", self.0.height))?;
        // builder.save().map_err(|_| anyhow!("cound't find node"))?;

        // set edit mode size
        let uid = ctx
            .engine_context
            .get_nodes_by_id(EDIT_MODE_STAGE_GROUP)
            .first()
            .cloned()
            .unwrap()
            .global_id()
            .unwrap();
        let mut builder = (&mut *dt)
            .get_orm_mut()
            .get_node(uid)
            .ok_or_else(|| anyhow!("cound't find node"))?;
        builder.set_property("height", &format!("{}px", self.0.height))?;
        builder.save().map_err(|_| anyhow!("cound't find node"))?;
        Ok(CanUndo::No)
    }
}
