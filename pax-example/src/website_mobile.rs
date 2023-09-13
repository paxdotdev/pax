#![allow(unused_imports)]

use pax_lang::*;
use pax_lang::api::*;
use pax_std::primitives::{Frame, Group, Rectangle, Text, Image, Scroller};
use pax_std::types::{Color, Fill, LinearGradient, StackerDirection};
use pax_std::components::{Stacker};

#[derive(Pax)]
#[file("website_mobile.pax")]
pub struct WebsiteMobile {
    pub scroll_position: Property<f64>,
}


impl WebsiteMobile {
    pub fn handle_container_scroll(&mut self, _ctx: RuntimeContext, _args: ArgsScroll) {
    }
}