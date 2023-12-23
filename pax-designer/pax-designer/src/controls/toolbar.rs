use std::collections::HashMap;
use std::rc::Rc;
use pax_lang::*;
use pax_lang::api::{ArgsClick, NodeContext};
use pax_std::primitives::{Rectangle, Group, Text};


#[derive(Pax)]
#[file("controls/toolbar.pax")]
pub struct Toolbar {
}

impl Toolbar {

    pub fn handle_click_pointer(&mut self, ctx: &NodeContext, args: ArgsClick) {
        unimplemented!("handle click for pointer")
    }

    pub fn handle_click_brush(&mut self, ctx: &NodeContext, args: ArgsClick) {
        unimplemented!("handle click for brush")
    }

    pub fn handle_click_pen(&mut self, ctx: &NodeContext, args: ArgsClick) {
        unimplemented!("handle click for pen")
    }
}




