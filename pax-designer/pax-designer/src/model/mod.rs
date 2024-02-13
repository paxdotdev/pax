pub mod action;
mod input;

use crate::model::action::ActionContext;
use action::Action;
use anyhow::Result;
use pax_designtime::DesigntimeManager;
use pax_engine::api::NodeContext;
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

//--------------------------------------------------------------------------------------------------
// App state singleton (undo/redo stack that modifies this state handled separately by ActionManager)
//--------------------------------------------------------------------------------------------------

#[derive(Default)]
pub struct AppState {
    //globals
    pub selected_component_id: String,
    pub selected_template_node_id: Option<usize>,

    //toolbar
    pub selected_tool: Option<Tool>,

    //glass
    pub tool_visual: Option<ToolVisual>,

    //keyboard
    main_mod_key: bool,
}

#[derive(Clone, Copy)]
pub enum Tool {
    Rectangle,
    Pointer,
}

#[derive(Clone)]
pub enum ToolVisual {
    MovingNode {
        // local transform of object in relation to parent,
        // and parent transform in relation to world
        grab_offset_x: f64,
        grab_offset_y: f64,
    },
    Box {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        fill: Color,
        stroke: Color,
    },
}
