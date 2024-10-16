#![allow(unused_imports)]

use breakout::*;
use color_picker::*;
use fireworks::*;
use pax_kit::*;
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

    pub fn increment(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        pax_designer::model::perform_action(
            &pax_designer::ProjectMsg(pax_designer::model::app_state::ProjectMode::Edit),
            ctx,
        );
        let old_num_clicks = self.num_clicks.get();
        self.num_clicks.set(old_num_clicks + 1);
    }
}
