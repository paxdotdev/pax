#![allow(unused_imports)]

use pax_kit::*;
use fireworks::*;
use color_picker::*;
use breakout::*;
use space_game::*;

pub mod calculator;
pub use calculator::Calculator;


#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub ticks: Property<usize>,
    pub num_clicks: Property<usize>,
}

impl Example {
    pub fn handle_pre_render(&mut self, _ctx: &NodeContext) {
        let old_ticks = self.ticks.get();
        self.ticks.set((old_ticks + 1) % 255);
    }

    pub fn increment(&mut self, _ctx: &NodeContext, _args: Event<Click>) {
        let old_num_clicks = self.num_clicks.get();
        self.num_clicks.set(old_num_clicks + 1);
    }
}
