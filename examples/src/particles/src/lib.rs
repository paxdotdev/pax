#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;
use rand::random;

mod ball;
use crate::ball::Ball;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub particles: Property<Vec<Particle>>,
}

pub const PARTICLE_COUNT: usize = 200;
pub const LOOP_FRAMES: f64 = 200.0;
impl Example {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        let bounds_parent = ctx.bounds_parent.get();
        self.particles.set(
            (0..PARTICLE_COUNT)
                .map(|i| Particle {
                    x: random::<f64>() * bounds_parent.0,
                    y: random::<f64>() * bounds_parent.1,
                    magnitude: random::<f64>(),
                })
                .collect(),
        );
    }
    pub fn handle_tick(&mut self, ctx: &NodeContext) {}
}

#[pax]
#[custom(Defaults)]
#[derive(Debug)]
pub struct Particle {
    pub x: f64,
    pub y: f64,
    pub magnitude: f64,
}
