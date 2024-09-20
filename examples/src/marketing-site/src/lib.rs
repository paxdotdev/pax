#![allow(unused_imports)]

use pax_kit::*;

pub mod calculator;
pub use calculator::*;

pub mod fireworks;
pub use fireworks::*;

pub mod space_game;
pub use space_game::*;

pub mod color_picker;
pub use color_picker::*;

mod animation;


#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub ticks: Property<usize>,
    pub num_clicks: Property<usize>,
}

impl Example {
    pub fn handle_pre_render(&mut self, _ctx: &NodeContext) {
    }

    pub fn increment(&mut self, _ctx: &NodeContext, _args: Event<Click>) {
        let old_num_clicks = self.num_clicks.get();
        self.num_clicks.set(old_num_clicks + 1);
    }
}
