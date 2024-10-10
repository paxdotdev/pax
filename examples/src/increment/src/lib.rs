#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub ticks: Property<usize>,
    pub num_clicks: Property<usize>,
    pub current_rotation: Property<f64>,
}

const ROTATION_INCREMENT_DEGREES: f64 = 90.0;
const ROTATION_EASING_DURATION_FRAMES: u64 = 120;

impl Example {
    pub fn handle_pre_render(&mut self, _ctx: &NodeContext) {
        let old_ticks = self.ticks.get();
        self.ticks.set((old_ticks + 1) % 255);
    }

    pub fn increment(&mut self, _ctx: &NodeContext, _args: Event<Click>) {
        let old_num_clicks = self.num_clicks.get();
        let new_val = old_num_clicks + 1;
        self.num_clicks.set(new_val);
        self.current_rotation.ease_to(new_val as f64 * ROTATION_INCREMENT_DEGREES, ROTATION_EASING_DURATION_FRAMES, EasingCurve::OutQuad);
    }
}
