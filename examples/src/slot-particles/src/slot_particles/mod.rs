#![allow(unused_imports)]

use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;
use rand::Rng;
use std::cell::RefCell;
use std::rc::Rc;
#[pax]
#[main]
#[file("slot_particles/mod.pax")]
pub struct SlotParticles {
    pub particles: Property<Vec<ParticleData>>,
    pub persistent_rng_data: Property<Vec<RngData>>,

    //private
    pub particle_len: Property<usize>,
}

impl SlotParticles {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let tick = ctx.frames_elapsed.clone();
        let num = self.particle_len.clone();
        let rng = Rc::new(RefCell::new(rand::thread_rng()));
        let bounds = ctx.bounds_self.clone();
        let store = Rc::new(RefCell::new(Vec::new()));
        self.persistent_rng_data.replace_with(Property::computed(
            move || {
                let mut store = store.borrow_mut();
                let mut rng = rng.borrow_mut();
                let (w, h) = bounds.get();
                while store.len() < num.get() {
                    store.push(RngData {
                        x: rng.gen_range(0.0..w),
                        y: rng.gen_range(0.0..h),
                        dx: rng.gen_range(-1.0..1.0),
                        dy: rng.gen_range(-1.0..1.0),
                        r: rng.gen_range(-1.0..1.0),
                    });
                }
                store.clone()
            },
            &[self.particle_len.untyped()],
        ));
        let bounds = ctx.bounds_self.clone();
        let base_data = self.persistent_rng_data.clone();
        const SIZE: f64 = 30.0;
        self.particles.replace_with(Property::computed(
            move || {
                let t = tick.get() as f64;
                let base = base_data.get();
                let (w, h) = bounds.get();
                base.iter()
                    .map(|b| ParticleData {
                        x: (b.x + t * b.dx).rem_euclid(w + 4.0 * SIZE) - 2.0 * SIZE,
                        y: (b.y + t * b.dy).rem_euclid(h + 4.0 * SIZE) - 2.0 * SIZE,
                        width: SIZE,
                        height: SIZE,
                        rotate: t * b.r,
                    })
                    .collect()
            },
            &[
                self.persistent_rng_data.untyped(),
                ctx.frames_elapsed.untyped(),
                ctx.bounds_self.untyped(),
            ],
        ));
    }

    pub fn tick(&mut self, ctx: &NodeContext) {
        if ctx.slot_children != self.particle_len.get() {
            self.particle_len.set(ctx.slot_children);
        }
    }
}

#[pax]
pub struct RngData {
    pub x: f64,
    pub y: f64,
    pub dx: f64,
    pub dy: f64,
    pub r: f64,
}

#[pax]
pub struct ParticleData {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rotate: f64,
}
