pub mod action;
use crate::model::action::ActionContext;
use action::Action;
use anyhow::Result;
use pax_designtime::DesigntimeManager;
use pax_lang::api::NodeContext;
use std::cell::RefCell;
use std::ops::DerefMut;

// Needs to be changed if we use a multithreaded async runtime
thread_local!(
    pub static GLOBAL_STATE: RefCell<GlobalDesignerState> =
        RefCell::new(GlobalDesignerState::default());
);

#[derive(Default)]
pub struct GlobalDesignerState {
    pub actions: action::ActionManager,
    pub app_state: AppState,
}

pub fn perform_action(action: impl Into<Action>, ctx: &NodeContext) -> Result<()> {
    GLOBAL_STATE.with(|model| {
        let mut binding = model.borrow_mut();
        let GlobalDesignerState {
            ref mut actions,
            ref mut app_state,
        } = binding.deref_mut();
        ActionContext {
            action_manager: actions,
            designtime: &mut ctx.designtime.borrow_mut(),
            app_state,
        }
        .perform(action)
    })
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

// #[derive(Default, Clone)]
// pub enum FSMState {
//     #[default]
//     Idle, //no sceduled action (next input event will likely not do nothing - maybe trigger mouseovers?)
//     PanningCamera,     // ready to move view
//     ArmedForTranslate, // ready to move a object
//     ToolArmed(),
// }

#[derive(Clone, Copy)]
pub enum Tool {
    Rectangle,
}

pub enum ToolVisual {
    Box { x1: f64, y1: f64, x2: f64, y2: f64 },
}
