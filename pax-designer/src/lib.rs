#![allow(unused_imports)]
use anyhow::anyhow;
use granular_change_store::GranularManifestChangeStore;
use std::{collections::HashSet, rc::Rc, sync::Mutex};

use model::{
    action::{init::InitWorldTransform, meta::Schedule, pointer::Pointer, Action, ActionContext},
    app_state::{ProjectMode, StageInfo},
    input::Dir,
};
use pax_engine::{
    api::*,
    math::{Point2, Transform2, Vector2},
    *,
};
use pax_manifest::TypeId;
use pax_std::inline_frame::InlineFrame;
use pax_std::*;

pub mod console;
pub mod context_menu;
pub mod controls;
pub mod designer_node_type;
pub mod glass;
pub mod llm_interface;
pub mod message_log_display;
pub mod project_mode_toggle;
pub mod project_publish_button;

pub mod granular_change_store;
pub mod math;
pub mod model;
pub mod utils;

use console::Console;
use context_menu::{ContextMenuMsg, DesignerContextMenu};
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

use crate::math::coordinate_spaces::{self, World};

pub const DESIGNER_GLASS_ID: &str = "designer_glass";
//We only want to show the publish button on www.pax.dev for now; as a hack to parameterize this,
//we are reading this env var at compiletime and exposing it via a const to runtime.
const PAX_PUBLISH_BUTTON_ENABLED: bool = option_env!("PAX_PUBLISH_BUTTON").is_some();

#[pax]
#[engine_import_path("pax_engine")]
#[main]
#[file("lib.pax")]
pub struct PaxDesigner {
    // transform of the userland project relative to the glass bounds
    pub glass_to_world_transform: Property<Transform2D>,
    // outline width for the lines around the stage area,
    // needs to be a property since it exists in world space,
    // but has width depending on the glass_to_world transform
    pub stage_outline_width: Property<f64>,
    pub stage: Property<StageInfo>,
    pub is_in_play_mode: Property<bool>,
    pub manifest_loaded_from_server: Property<bool>,
    pub show_publish_button: Property<bool>,
    pub console_height: Property<f64>,
    pub console_status: Property<bool>,
    pub cull_console: Property<bool>,
}


const OPEN_CONSOLE_HEIGHT : f64 = 425.0;
const CLOSED_CONSOLE_HEIGHT : f64 = 55.0;
const CONSOLE_TRANSITION_DURATION : u64 = 40;

impl PaxDesigner {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        self.console_status.set(false);
        self.console_height.set(CLOSED_CONSOLE_HEIGHT);
        self.show_publish_button.set(PAX_PUBLISH_BUTTON_ENABLED);
        // needs to happen before model init
        self.setup_granular_notification_store(ctx);
        model::Model::init(ctx);
        model::read_app_state(|app_state| {
            self.bind_stage_property(&app_state);
            self.bind_glass_to_world_transform_property(&app_state);
            self.bind_is_in_play_mode(&app_state);
            self.bind_stage_outline_width_property(&app_state);
        });
        self.bind_is_manifest_loaded_from_server(ctx);
    }


    pub fn toggle_console(&mut self, ctx: &NodeContext, args: Event<Click>) {
        let current_console_status = self.console_status.get();
        let new_console_status = !current_console_status;
        self.console_status.set(new_console_status);

        if new_console_status {
            self.console_height.ease_to(OPEN_CONSOLE_HEIGHT, CONSOLE_TRANSITION_DURATION, EasingCurve::OutQuad);
        } else {
            self.console_height.ease_to(CLOSED_CONSOLE_HEIGHT, CONSOLE_TRANSITION_DURATION, EasingCurve::OutQuad);
        }
    }

    pub fn tick(&mut self, ctx: &NodeContext) {
        // Some actions can't be run immediately because of manifest state
        // hasn't been written to engine yet - this is the place they get
        // flushed
        model::action::meta::flush_sheduled_actions(ctx);


        let should_cull_console =self.console_height.get() >= CLOSED_CONSOLE_HEIGHT - f64::EPSILON && self.console_height.get() <= CLOSED_CONSOLE_HEIGHT + f64::EPSILON;
        self.cull_console.set(should_cull_console);

    }

    fn bind_stage_property(&mut self, app_state: &model::AppState) {
        let stage = app_state.stage.clone();
        let deps = [stage.untyped()];
        self.stage
            .replace_with(Property::computed(move || stage.get(), &deps));
    }

    fn bind_is_manifest_loaded_from_server(&mut self, ctx: &NodeContext) {
        // used to show "loading screen"
        let manifest_load_state = borrow!(ctx.designtime).get_manifest_loaded_from_server_prop();
        let deps = [manifest_load_state.untyped()];
        self.manifest_loaded_from_server
            .replace_with(Property::computed(move || manifest_load_state.get(), &deps));
        model::perform_action(
            &Schedule {
                action: Rc::new(InitWorldTransform),
            },
            ctx,
        );
    }

    fn bind_glass_to_world_transform_property(&mut self, app_state: &model::AppState) {
        let glass_to_world = app_state.glass_to_world_transform.clone();
        let deps = [glass_to_world.untyped()];
        self.glass_to_world_transform
            .replace_with(Property::computed(
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

    fn bind_stage_outline_width_property(&mut self, app_state: &model::AppState) {
        let glass_to_world = app_state.glass_to_world_transform.clone();
        let deps = [glass_to_world.untyped()];
        self.stage_outline_width.replace_with(Property::computed(
            move || {
                let world_to_glass = glass_to_world.get().inverse();
                let s = world_to_glass.get_scale();
                // scaling x/y should always be equal
                3.0 / s.x
            },
            &deps,
        ));
    }

    fn bind_is_in_play_mode(&mut self, app_state: &model::AppState) {
        let proj_mode = app_state.project_mode.clone();
        let deps = [proj_mode.untyped()];
        self.is_in_play_mode.replace_with(Property::computed(
            move || matches!(proj_mode.get(), ProjectMode::Playing),
            &deps,
        ));
    }

    fn setup_granular_notification_store(&self, ctx: &NodeContext) {
        let granular_update_store = GranularManifestChangeStore::default();
        ctx.push_local_store(granular_update_store.clone());
        let manifest_ver = borrow!(ctx.designtime).get_last_rendered_manifest_version();
        let ctxp = ctx.clone();
        ctx.subscribe(&[manifest_ver.untyped()], move || {
            let manifest_modification_data = borrow_mut!(ctxp.designtime)
                .get_orm_mut()
                .take_manifest_modification_data();
            granular_update_store
                .notify_from_manifest_modification_data(manifest_modification_data);
        });
    }

    pub fn focused(&mut self, _ctx: &NodeContext, _args: Event<Focus>) {
        // Reset modifier keys
        model::read_app_state(|app_state| {
            app_state.modifiers.set(HashSet::new());
        });
    }

    pub fn handle_mouse_move(&mut self, ctx: &NodeContext, args: Event<MouseMove>) {
        model::perform_action(
            &crate::model::action::pointer::MouseEntryPointAction {
                event: Pointer::Move,
                button: args.mouse.button.clone(),
                point: Point2::new(args.mouse.x, args.mouse.y),
            },
            ctx,
        );
    }

    pub fn handle_mouse_up(&mut self, ctx: &NodeContext, event: Event<MouseUp>) {
        // TODO: long term some form of pointer capture would be useful to
        // allow the elements to keep listening to mouse ups outside of the
        // objects. see:
        // https://developer.mozilla.org/en-US/docs/Web/API/Pointer_events#capturing_the_pointer

        // Context menu mouse up
        if event.mouse.button != MouseButton::Right {
            model::perform_action(
                // if triggered directly, doesn't have time to register click on context menu
                &Schedule {
                    action: Rc::new(ContextMenuMsg::Close),
                },
                ctx,
            );
        }

        // Glass mouse up
        model::perform_action(
            &crate::model::action::pointer::MouseEntryPointAction {
                event: Pointer::Up,
                button: event.mouse.button.clone(),
                point: Point2::new(event.mouse.x, event.mouse.y),
            },
            ctx,
        );

        // Color picker mouse up
        color_picker::trigger_mouseup();

        // Tree mouse up
        tree::trigger_global_mouseup();

        // Toolbar mouse up
        if toolbar::dropdown_is_in_open_state() {
            model::perform_action(
                // if done directly, would not have time to intercept mouse event further down to select a new tool
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

pub struct ProjectMsg(pub ProjectMode);

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
