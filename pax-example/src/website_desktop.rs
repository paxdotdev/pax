#![allow(unused_imports)]

use pax_lang::api::*;
use pax_lang::*;
use pax_std::components::Stacker;
use pax_std::primitives::{Frame, Group, Image, Rectangle, Scroller, Text};
use pax_std::types::text::*;
use pax_std::types::{Color, Fill, LinearGradient, StackerDirection};

#[derive(Pax)]
#[file("website_desktop.pax")]
pub struct WebsiteDesktop {
    pub ticks: Property<usize>,
}

impl WebsiteDesktop {
    pub fn handle_did_mount(&mut self, _ctx: RuntimeContext) {}
    pub fn handle_will_render(&mut self, ctx: RuntimeContext) {
        self.ticks.set(ctx.frames_elapsed);
    }
}

