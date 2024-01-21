use pax_lang::api::{ArgsClick, NodeContext};
use pax_lang::*;
use pax_std::components::Stacker;
use pax_std::primitives::{Group, Image, Rectangle};
use std::collections::HashMap;

use std::rc::Rc;

#[derive(Pax)]
#[file("controls/toolbar.pax")]
pub struct Toolbar {}

impl Toolbar {
    pub fn handle_click_pointer(&mut self, _ctx: &NodeContext, _args: ArgsClick) {
        unimplemented!("handle click for pointer")
    }

    pub fn handle_click_brush(&mut self, _ctx: &NodeContext, _args: ArgsClick) {
        unimplemented!("handle click for brush")
    }

    pub fn handle_click_pen(&mut self, _ctx: &NodeContext, _args: ArgsClick) {
        unimplemented!("handle click for pen")
    }

    pub fn handle_click_rect(&mut self, _ctx: &NodeContext, _args: ArgsClick) {
        unimplemented!("handle click for rect")
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
