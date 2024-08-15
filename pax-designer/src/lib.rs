#![allow(unused_imports)]

use anyhow::anyhow;
use pax_engine::{api::*, math::Point2, *};
use pax_manifest::TypeId;
use pax_std::*;
use std::sync::Mutex;

use pax_std::inline_frame::InlineFrame;

use crate::math::coordinate_spaces::{self, World};
use model::{
    action::{pointer::Pointer, Action, ActionContext},
    ProjectMode, StageInfo,
};

pub mod context_menu;
pub mod glass;
pub mod utils;

pub mod controls;
pub mod llm_interface;
pub mod math;
pub mod message_log_display;
pub mod model;
pub mod project_mode_toggle;

use context_menu::DesignerContextMenu;
use controls::Controls;
use glass::Glass;
use llm_interface::LLMInterface;
use message_log_display::MessageLogDisplay;
use project_mode_toggle::ProjectModeToggle;

use pax_std::*;

// TODO:
// clean up glass::on_double_click
// remove with_action_context and make everything actions?

// Things to decide:
// - Who/what should be allowed to modify model state? (harder to encode when everything is Properties)

pub const ROOT_PROJECT_ID: &str = "userland_project";
pub const DESIGNER_GLASS_ID: &str = "designer_glass";


#[pax]
#[main]
#[file("lib.pax")]
pub struct PaxDesigner {
    pub transform2d: Property<Transform2D>,
    pub stage: Property<StageInfo>,
    pub play_active: Property<bool>,

    pub glass_active: Property<bool>,
    pub manifest_loaded_from_server: Property<bool>,
}

impl PaxDesigner {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        model::Model::init(ctx);
        model::read_app_state(|app_state| {
            self.bind_stage_property(&app_state);
            self.bind_transform2d_property(&app_state);
            self.bind_glass_active_property(&app_state);
            self.bind_interaction_mode_property();
        });

        // used to show "loading screen"
        let manifest_load_state = borrow!(ctx.designtime).get_manifest_loaded_from_server_prop();
        let deps = [manifest_load_state.untyped()];
        self.manifest_loaded_from_server
            .replace_with(Property::computed(
                move || {
                    log::debug!("manifest load state: {}", manifest_load_state.get());
                    manifest_load_state.get()
                },
                &deps,
            ));
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
        self.play_active
            .replace_with(Property::computed(move || !glass_active.get(), &deps));
    }

    pub fn handle_mouse_move(&mut self, ctx: &NodeContext, args: Event<MouseMove>) {
        let prevent_default = || args.prevent_default();
        model::perform_action(
            &crate::model::action::pointer::MouseEntryPointAction {
                prevent_default: &prevent_default,
                event: Pointer::Move,
                button: args.mouse.button.clone(),
                point: Point2::new(args.mouse.x, args.mouse.y),
            },
            ctx,
        );
    }

    pub fn handle_mouse_up(&mut self, ctx: &NodeContext, args: Event<MouseUp>) {
        let prevent_default = || args.prevent_default();
        model::perform_action(
            &crate::model::action::pointer::MouseEntryPointAction {
                prevent_default: &prevent_default,
                event: Pointer::Up,
                button: args.mouse.button.clone(),
                point: Point2::new(args.mouse.x, args.mouse.y),
            },
            ctx,
        );
    }
}

pub struct ProjectMsg(ProjectMode);

impl Action for ProjectMsg {
    fn perform(&self, ctx: &mut ActionContext) -> anyhow::Result<()> {
        ctx.app_state.project_mode.set(self.0.clone());
        Ok(())
    }
}

pub struct SetStage(pub StageInfo);

impl Action for SetStage {
    fn perform(&self, ctx: &mut ActionContext) -> anyhow::Result<()> {
        ctx.app_state.stage.set(self.0.clone());
        Ok(())
    }
}
