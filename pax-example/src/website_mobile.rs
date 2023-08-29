use pax_lang::*;
use pax_lang::api::*;
use pax_std::primitives::{Frame, Group, Rectangle, Text, Image, Scroller};
use pax_std::types::{Color, Fill, LinearGradient, StackerDirection};
use pax_std::components::{Stacker, Sidebar};

#[derive(Pax)]
#[file("website_mobile.pax")]
pub struct WebsiteMobile {
    pub scroll_position: Property<f64>,
}


impl WebsiteMobile {
    pub fn handle_container_scroll(&mut self, ctx: RuntimeContext, args: ArgsScroll) {
        let mut scroll_position = *self.scroll_position.get();
        scroll_position = scroll_position + args.delta_y;
        scroll_position = scroll_position.min(0.0);
        scroll_position = scroll_position.max(-3400.0);
        self.scroll_position.set(scroll_position);
    }
}