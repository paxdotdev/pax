use pax_lang::*;
use pax_lang::api::*;
use pax_std::primitives::{Group, Text};
use pax_std::types::{Color};
use pax_std::components::{Stacker, Sidebar};

#[derive(Pax)]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub container_width: Property<f64>,
}

impl Example {
    pub fn handle_did_mount(&mut self, ctx: RuntimeContext) {
        pax_lang::log("Mounted!");
    }

    pub fn handle_will_render(&mut self, ctx: RuntimeContext) {
        self.container_width.set(ctx.bounds_parent.0);
    }
}