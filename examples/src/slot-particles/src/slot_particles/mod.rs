#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

#[pax]
#[main]
#[file("slot_particles/mod.pax")]
pub struct SlotParticles {
    pub particles: Property<Vec<ParticleData>>,

    //private
    pub particle_len: Property<usize>,
}

impl SlotParticles {
    pub fn tick(&mut self, ctx: &NodeContext) {
        if ctx.slot_children != self.particle_len.get() {
            self.particle_len.set(ctx.slot_children);
        }
    }

    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let tick = ctx.frames_elapsed.clone();
        let num = self.particle_len.clone();
        self.particles.replace_with(Property::computed(
            move || {
                let t = tick.get() as f64 / 100.0;
                let n = num.get();
                (0..n)
                    .map(|v| ParticleData {
                        x: t + 100.0 * v as f64,
                        y: t + 100.0 * v as f64,
                        width: 30.0,
                        height: 30.0,
                    })
                    .collect()
            },
            &[ctx.frames_elapsed.untyped(), self.particle_len.untyped()],
        ));
    }
}

#[pax]
pub struct ParticleData {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
