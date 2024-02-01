#![allow(unused_imports)]

use pax_lang::api::*;
use pax_lang::*;

pub mod controls;
pub mod designtime_component_viewer;
pub mod glass;

use crate::controls::Controls;
use crate::designtime_component_viewer::DesigntimeComponentViewer;
use crate::glass::Glass;

#[pax]
#[main]
#[file("lib.pax")]
pub struct PaxDesigner {
    // pub state: DesignerState,
}

impl PaxDesigner {
    pub fn handle_mount(&mut self, _ctx: &NodeContext) {
        let _orm = _ctx.designtime.borrow().get_orm();
        //self.state = load_previous_state_if_relevant();
    }
}

//TODO: derive Serialize and Deserialize
#[derive(Default, Clone)]
pub struct DesignerState {
    // designtime_api: pax_designtime::api::DesigntimeApi,
    // undo_stack
    // redo_stack
}

// pub trait DesignerCommand {
//     fn execute_command(&mut DesignerState, &DesigntimeApi);
//     fn undo(&mut DesignerState, &DesigntimeApi);
//     fn redo(&mut DesignerState, &DesigntimeApi);
// }
