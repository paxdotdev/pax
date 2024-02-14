use pax_engine::api::{ArgsButtonClick, ArgsClick, NodeContext};
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::primitives::{Group, Image, Rectangle};
use std::collections::HashMap;

use pax_std::primitives::Button;
use std::rc::Rc;

use crate::model::action::Action;
use crate::model::{self, Tool};
use anyhow::Result;
use model::action::CanUndo;

#[pax]
#[file("controls/toolbar.pax")]
pub struct Toolbar {}

pub struct SelectTool {
    pub tool: Tool,
}

impl Action for SelectTool {
    fn perform(self, ctx: &mut model::action::ActionContext) -> Result<CanUndo> {
        ctx.app_state.selected_tool = self.tool;
        Ok(CanUndo::No)
    }
}

impl Toolbar {
    pub fn save_component(&mut self, ctx: &NodeContext, _args: ArgsButtonClick) {
        let mut dt = ctx.designtime.borrow_mut();
        dt.send_component_update("pax_designer::pax_reexports::designer_project::Example");
    }

    pub fn handle_click_pointer(&mut self, ctx: &NodeContext, _args: ArgsClick) {
        model::perform_action(
            SelectTool {
                tool: Tool::Pointer,
            },
            ctx,
        );
    }

    pub fn handle_click_brush(&mut self, ctx: &NodeContext, _args: ArgsClick) {}

    pub fn handle_click_pen(&mut self, ctx: &NodeContext, _args: ArgsClick) {}

    pub fn handle_click_rect(&mut self, ctx: &NodeContext, _args: ArgsClick) {
        model::perform_action(
            SelectTool {
                tool: Tool::Rectangle,
            },
            ctx,
        );
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
