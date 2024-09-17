#![allow(unused_imports)]

use anyhow::anyhow;
use pax_engine::{
    api::*,
    math::{Point2, Transform2, Vector2},
    *,
};
use pax_manifest::TypeId;
use pax_std::*;
use std::{collections::HashSet, rc::Rc, sync::Mutex};

use pax_std::inline_frame::InlineFrame;

use crate::math::coordinate_spaces::{self, World};
use model::{
    action::{meta::Schedule, pointer::Pointer, Action, ActionContext},
    input::Dir,
    ProjectMode, StageInfo,
};

pub mod context_menu;
pub mod designer_node_type;
pub mod glass;
pub mod utils;

pub mod controls;
pub mod llm_interface;
pub mod math;
pub mod message_log_display;
pub mod model;
pub mod project_mode_toggle;
pub mod project_publish_button;

use context_menu::DesignerContextMenu;
use controls::{
    settings::color_picker,
    toolbar::{self, CloseDropdown},
    tree, Controls,
};
use glass::Glass;
use llm_interface::LLMInterface;
use message_log_display::MessageLogDisplay;
use project_mode_toggle::ProjectModeToggle;
use project_publish_button::ProjectPublishButton;

use pax_std::*;

// TODO:
// clean up glass::on_double_click
// remove with_action_context and make everything actions?

// Things to decide:
// - Who/what should be allowed to modify model state? (harder to encode when everything is Properties)

pub const DESIGNER_GLASS_ID: &str = "designer_glass";

//We only want to show the publish button on www.pax.dev for now; as a hack to parameterize this,
//we are reading this env var at compiletime and exposing it via a const to runtime.
const PAX_PUBLISH_BUTTON_ENABLED: bool = option_env!("PAX_PUBLISH_BUTTON").is_some();

#[pax]
#[engine_import_path("pax_engine")]
#[main]
#[file("lib.pax")]
pub struct PaxDesigner {
    pub transform2d: Property<Transform2D>,
    pub stage: Property<StageInfo>,
    pub play_active: Property<bool>,
    pub glass_active: Property<bool>,
    pub manifest_loaded_from_server: Property<bool>,
    pub show_publish_button: Property<bool>,
}

impl PaxDesigner {
    pub fn on_mount(&mut self, ctx: &NodeContext) {

        self.show_publish_button.set(PAX_PUBLISH_BUTTON_ENABLED);

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
            .replace_with(Property::computed(move || manifest_load_state.get(), &deps));
    }

    pub fn tick(&mut self, ctx: &NodeContext) {
        if ctx.frames_elapsed.get() == 1 {
            // when manifests load, set transform of glass to fit scene
            model::read_app_state(|app_state| {
                let stage = app_state.stage.get();
                let Some(glass_node) = ctx.get_nodes_by_id(DESIGNER_GLASS_ID).into_iter().next()
                else {
                    log::warn!("cound't hook up glass to world transform: couldn't find designer glass node");
                    return;
                };
                let (w, h) = glass_node.transform_and_bounds().get().bounds;
                app_state.glass_to_world_transform.set(
                    Transform2::<World>::translate(Vector2::new(
                        stage.width as f64 / 2.0,
                        stage.height as f64 / 2.0,
                    )) * Transform2::scale(1.4)
                        * Transform2::<math::coordinate_spaces::Glass>::translate(-Vector2::new(
                            w / 2.0,
                            h / 2.0,
                        )),
                );
            });
        }

        model::action::meta::flush_sheduled_actions(ctx);
    }

    fn bind_stage_property(&mut self, app_state: &model::AppState) {
        let stage = app_state.stage.clone();
        let deps = [stage.untyped()];
        self.stage
            .replace_with(Property::computed(move || stage.get(), &deps));
    }

    pub fn focused(&mut self, _ctx: &NodeContext, _args: Event<Focus>) {
        // Reset modifier keys
        model::read_app_state(|app_state| {
            app_state.modifiers.set(HashSet::new());
        });
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

    pub fn handle_mouse_up(&mut self, ctx: &NodeContext, event: Event<MouseUp>) {
        // TODO: the below todos refer to the original locations of these event handlers.
        // long term some form of pointer capture would be useful to allow
        // the elements to keep listening to mouse ups outside of the objects.
        // see: https://developer.mozilla.org/en-US/docs/Web/API/Pointer_events#capturing_the_pointer

        let prevent_default = || event.prevent_default();
        // NOTE: this was originally on glass
        model::perform_action(
            &crate::model::action::pointer::MouseEntryPointAction {
                prevent_default: &prevent_default,
                event: Pointer::Up,
                button: event.mouse.button.clone(),
                point: Point2::new(event.mouse.x, event.mouse.y),
            },
            ctx,
        );
        color_picker::trigger_mouseup();
        tree::trigger_global_mouseup();
        if toolbar::dropdown_is_in_open_state() {
            model::perform_action(
                // if done directly, would not have time to intercept mouse event futher down to select a new tool
                &Schedule {
                    action: Rc::new(CloseDropdown),
                },
                ctx,
            );
        }
    }

    pub fn handle_key_down(&mut self, ctx: &NodeContext, event: Event<KeyDown>) {
        event.prevent_default();
        model::process_keyboard_input(ctx, Dir::Down, event.keyboard.key.clone());
    }

    pub fn handle_key_up(&mut self, ctx: &NodeContext, event: Event<KeyUp>) {
        event.prevent_default();
        model::process_keyboard_input(ctx, Dir::Up, event.keyboard.key.clone());
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
