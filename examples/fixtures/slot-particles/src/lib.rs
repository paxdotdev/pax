#![allow(unused_imports)]
pub mod slot_particles;
use crate::slot_particles::SlotParticles;
use fireworks::*;
use pax_kit::*;

pax_kit::runnable! { root: crate::Example }

#[pax]
#[file("lib.pax")]
pub struct Example {}

impl Example {}
