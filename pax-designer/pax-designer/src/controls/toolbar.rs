use pax_designtime::input::{FSMEvent, Interface};
use pax_lang::api::{ArgsButtonClick, ArgsClick, NodeContext};
use pax_lang::*;
use pax_std::components::Stacker;
use pax_std::primitives::{Group, Image, Rectangle};
use std::collections::HashMap;

use pax_std::primitives::Button;
use std::rc::Rc;

#[pax]
#[file("controls/toolbar.pax")]
pub struct Toolbar {}

impl Toolbar {
    pub fn save_component(&mut self, ctx: &NodeContext, _args: ArgsButtonClick) {
        let mut dt = ctx.designtime.borrow_mut();
        dt.send_component_update("pax_designer::pax_reexports::designer_project::Example");
    }

    pub fn handle_click_pointer(&mut self, _ctx: &NodeContext, _args: ArgsClick) {
        unimplemented!("handle click for pointer")
    }

    pub fn handle_click_brush(&mut self, _ctx: &NodeContext, _args: ArgsClick) {
        unimplemented!("handle click for brush")
    }

    pub fn handle_click_pen(&mut self, _ctx: &NodeContext, _args: ArgsClick) {
        unimplemented!("handle click for pen")
    }

    pub fn handle_click_rect(&mut self, ctx: &NodeContext, _args: ArgsClick) {
        let res = ctx
            .designtime
            .borrow_mut()
            .input_transition(FSMEvent::InterfaceEvent(Interface::ActivateRectangleTool));
        log(&format!("input result: {:?}", res));
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
