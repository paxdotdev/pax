#![allow(unused_imports)]

use pax_lang::api::*;
use pax_lang::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

#[derive(Pax)]
#[file("inner.pax")]
pub struct Inner {
    pub text: Property<StringBox>,
}

impl Inner {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        //self.text.set("Click me Yayyy".to_string());
    }
}
