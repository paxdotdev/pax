pub mod action;
mod input;

use crate::model::action::ActionContext;
use action::Action;
use anyhow::Result;
use pax_designtime::DesigntimeManager;
use pax_engine::rendering::kurbo;
use pax_engine::{
    api::NodeContext,
    rendering::{Point2D, TransformAndBounds},
};
use pax_std::types::Color;
use std::cell::RefCell;

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
            node_context: ctx,
            app_state,
        }
        .execute(action)
    })
}

pub fn read_app_state(closure: impl FnOnce(&AppState)) {
    GLOBAL_STATE.with(|model| {
        closure(&model.borrow_mut().app_state);
    });
}

pub fn register_glass_transform(transform: kurbo::Affine) {
    GLOBAL_STATE.with(|model| {
        let mut model = model.borrow_mut();
        model.app_state.screen_to_glass_transform = transform;
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
    pub screen_to_glass_transform: kurbo::Affine,
    pub glass_to_world_transform: kurbo::Affine,

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
    Movement {
        delta: Point2D,
    },
    Box {
        p1: Point2D,
        p2: Point2D,
        fill: Color,
        stroke: Color,
    },
}
