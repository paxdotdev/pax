pub mod action;
mod input;
pub mod math;

use crate::model::action::ActionContext;
use action::Action;
use anyhow::Result;
use math::coordinate_spaces::World;
use pax_designtime::DesigntimeManager;
use pax_engine::math::{Transform2, Vector2};
use pax_engine::{api::NodeContext, math::Point2, rendering::TransformAndBounds};
use pax_std::types::Color;
use std::cell::RefCell;

use math::coordinate_spaces::Glass;

use self::math::coordinate_spaces;

// Needs to be changed if we use a multithreaded async runtime
thread_local!(
    static GLOBAL_STATE: RefCell<GlobalDesignerState> = RefCell::new(GlobalDesignerState::new());
);

#[derive(Default)]
pub struct GlobalDesignerState {
    pub undo_stack: action::UndoStack,
    pub app_state: AppState,
}

impl GlobalDesignerState {
    fn new() -> Self {
        Self {
            app_state: AppState {
                selected_component_id: "pax_designer::pax_reexports::designer_project::Example"
                    .to_owned(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

pub fn perform_action(action: impl Action, ctx: &NodeContext) -> Result<()> {
    GLOBAL_STATE.with(|model| {
        let mut binding = model.borrow_mut();
        let GlobalDesignerState {
            ref mut undo_stack,
            ref mut app_state,
            ..
        } = *binding;
        ActionContext {
            undo_stack,
            engine_context: ctx,
            app_state,
        }
        .execute(action)
    })
}

pub fn selected_bounds(ctx: &NodeContext) -> Option<[Point2<Glass>; 4]> {
    GLOBAL_STATE.with(|model| {
        let mut binding = model.borrow_mut();
        let GlobalDesignerState {
            ref mut undo_stack,
            ref mut app_state,
            ..
        } = *binding;
        ActionContext {
            undo_stack,
            engine_context: ctx,
            app_state,
        }
        .selected_bounds()
    })
}

pub fn read_app_state(closure: impl FnOnce(&AppState)) {
    GLOBAL_STATE.with(|model| {
        closure(&model.borrow_mut().app_state);
    });
}

//--------------------------------------------------------------------------------------------------
// App state singleton (undo/redo stack that modifies this state handled separately by ActionManager)
//--------------------------------------------------------------------------------------------------

#[derive(Default)]
pub struct AppState {
    //globals
    pub selected_component_id: String,
    pub selected_template_node_id: Option<usize>,
    pub glass_to_world_transform: Transform2<Glass, World>,

    //toolbar
    pub selected_tool: Tool,

    //glass
    pub tool_state: ToolState,

    //keyboard
    main_mod_key: bool,
}

#[derive(Clone, Copy, Default)]
pub enum Tool {
    #[default]
    Pointer,
    Rectangle,
}

#[derive(Clone, Default)]
pub enum ToolState {
    #[default]
    Idle,
    Pan {
        point: Point2<Glass>,
        offset: Vector2<World>,
    },
    Movement {
        offset: Vector2<World>,
    },
    Box {
        p1: Point2<Glass>,
        p2: Point2<Glass>,
        fill: Color,
        stroke: Color,
    },
}
