#![allow(unused_imports)]

use pax_lang::api::*;
use pax_lang::*;
use pax_std::components::Stacker;
use pax_std::primitives::{Frame, Group, Image, Rectangle, Scroller, Text};
use pax_std::types::{Color, Fill, LinearGradient, StackerDirection};

#[derive(Pax)]
#[file("website_mobile.pax")]
pub struct WebsiteMobile {
    pub scroll_position: Property<f64>,
}

impl WebsiteMobile {
    pub fn handle_container_scroll(&mut self, _ctx: RuntimeContext, _args: ArgsScroll) {}
}
