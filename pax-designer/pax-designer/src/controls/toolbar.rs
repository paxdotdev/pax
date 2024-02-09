use pax_lang::api::{ArgsButtonClick, ArgsClick, NodeContext};
use pax_lang::*;
use pax_std::components::Stacker;
use pax_std::primitives::{Group, Image, Rectangle};
use std::collections::HashMap;

use pax_std::primitives::Button;
use std::rc::Rc;

use crate::model::{self, Tool};

#[pax]
#[file("controls/toolbar.pax")]
pub struct Toolbar {}

impl Toolbar {
    pub fn save_component(&mut self, ctx: &NodeContext, _args: ArgsButtonClick) {
        let mut dt = ctx.designtime.borrow_mut();
        dt.send_component_update("pax_designer::pax_reexports::designer_project::Example");
    }

    pub fn handle_click_pointer(&mut self, ctx: &NodeContext, _args: ArgsClick) {
        model::with_app_state(|app_state| {
            app_state.selected_tool = Some(Tool::Pointer);
        });
    }

    pub fn handle_click_brush(&mut self, ctx: &NodeContext, _args: ArgsClick) {}

    pub fn handle_click_pen(&mut self, ctx: &NodeContext, _args: ArgsClick) {}

    pub fn handle_click_rect(&mut self, ctx: &NodeContext, _args: ArgsClick) {
        model::with_app_state(|app_state| {
            app_state.selected_tool = Some(Tool::Rectangle);
        });
    }

    pub fn handle_click_stacker(&mut self, _ctx: &NodeContext, _args: ArgsClick) {
        unimplemented!("handle click for stacker")
    }

    pub fn handle_click_text(&mut self, _ctx: &NodeContext, _args: ArgsClick) {
        unimplemented!("handle click for text")
    }

    pub fn handle_click_checkbox(&mut self, _ctx: &NodeContext, _args: ArgsClick) {
        unimplemented!("handle click for checkbox")
    }

    pub fn handle_click_speech(&mut self, _ctx: &NodeContext, _args: ArgsClick) {
        unimplemented!("handle click for speech")
    }
}
