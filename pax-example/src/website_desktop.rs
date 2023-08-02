use pax_lang::*;
use pax_lang::api::*;
use pax_std::primitives::{Frame, Group, Rectangle, Text, Image};
use pax_std::types::{Color, Fill, LinearGradient, StackerDirection};
use pax_std::components::{Stacker, Sidebar};

#[derive(Pax)]
#[file("website_desktop.pax")]
pub struct WebsiteDesktop {
    pub scroll_position: Property<f64>,
}

impl WebsiteDesktop {
    pub fn handle_container_scroll(&mut self, ctx: RuntimeContext, args: ArgsScroll) {
        let mut scroll_position = *self.scroll_position.get();
        scroll_position = scroll_position - args.delta_y;
        scroll_position = scroll_position.min(0.0);
        scroll_position = scroll_position.max(-4000.0);
        self.scroll_position.set(scroll_position);
    }

    pub fn handle_container_key_down(&mut self, ctx: RuntimeContext, args: ArgsKeyDown) {
        let mut scroll_position = *self.scroll_position.get();
        if args.keyboard.key == "ArrowDown".to_string() || args.keyboard.key == "Down".to_string() {
            scroll_position = scroll_position - 20.0;
            scroll_position = scroll_position.min(0.0);
            scroll_position = scroll_position.max(-4000.0);
        }
        if args.keyboard.key == "ArrowUp".to_string() || args.keyboard.key == "Up".to_string() {
            scroll_position = scroll_position + 20.0;
            scroll_position = scroll_position.min(0.0);
            scroll_position = scroll_position.max(-4000.0);
        }
        if args.keyboard.key == "ArrowLeft".to_string() || args.keyboard.key == "Left".to_string() {
            scroll_position = scroll_position + 1000.0;
            scroll_position = scroll_position.min(0.0);
            scroll_position = scroll_position.max(-4000.0);
        }
        if args.keyboard.key == "ArrowRight".to_string() || args.keyboard.key == "Right".to_string() {
            scroll_position = scroll_position - 1000.0;
            scroll_position = scroll_position.min(0.0);
            scroll_position = scroll_position.max(-4000.0);
        }
        self.scroll_position.set(scroll_position);
    }
}