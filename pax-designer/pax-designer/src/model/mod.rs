pub mod action;
use crate::model::action::ActionContext;
use action::Action;
use anyhow::Result;
use pax_designtime::DesigntimeManager;
use pax_lang::api::NodeContext;
use pax_std::types::Color;
use std::cell::RefCell;

// Needs to be changed if we use a multithreaded async runtime
thread_local!(
    static GLOBAL_STATE: RefCell<GlobalDesignerState> =
        RefCell::new(GlobalDesignerState::default());
);

#[derive(Default)]
pub struct GlobalDesignerState {
    pub actions: action::ActionManager,
    pub app_state: AppState,
}

pub fn perform_action(action: impl Action, ctx: &NodeContext) -> Result<()> {
    GLOBAL_STATE.with(|model| {
        let mut binding = model.borrow_mut();
        let GlobalDesignerState {
            ref mut actions,
            ref mut app_state,
        } = *binding;
        ActionContext {
            action_manager: actions,
            designtime: &mut ctx.designtime.borrow_mut(),
            app_state,
        }
        .perform(action)
    })
}

pub fn with_app_state(closure: impl FnOnce(&mut AppState)) {
    GLOBAL_STATE.with(|model| {
        closure(&mut model.borrow_mut().app_state);
    });
}

//--------------------------------------------------------------------------------------------------
// App state singleton (undo/redo stack that modifies this state handled separately by ActionManager)
//--------------------------------------------------------------------------------------------------

#[derive(Default)]
pub struct AppState {
    //globals
    // pub fsm_state: action::FSMState,
    pub selection_state: Vec<usize>,

    //toolbar
    pub selected_tool: Option<Tool>,

    //glass
    pub tool_visual: Option<ToolVisual>,
}

#[derive(Clone, Copy)]
pub enum Tool {
    Rectangle,
    Pointer,
}

pub enum ToolVisual {
    Box {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        fill: Color,
        stroke: Color,
    },
}
