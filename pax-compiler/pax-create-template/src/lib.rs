use pax_lang::*;
use pax_lang::api::*;
use pax_std::primitives::{Frame, Group, Rectangle, Text, Image};
use pax_std::types::{Color, Fill, LinearGradient, StackerDirection};
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