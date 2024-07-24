#![allow(unused_imports)]
use pax_engine::api::*;
use pax_engine::*;
use pax_std::*;

#[pax]
#[main]
#[file("fireworks.pax")]
pub struct Fireworks {
    pub rotation: Property<f64>,
    pub ticks: Property<usize>,
}

const ROTATION_COEFFICIENT: f64 = 0.00010;

impl Fireworks {
    pub fn handle_wheel(&mut self, _ctx: &NodeContext, args: Event<Wheel>) {
        let old_t = self.rotation.get();
        let new_t = old_t - args.delta_y * ROTATION_COEFFICIENT;
        self.rotation.set(f64::max(0.0, new_t));
    }

    pub fn handle_tick(&mut self, _ctx: &NodeContext) {
        let old_ticks = self.ticks.get();
        self.ticks.set(old_ticks + 1);
    }
}
