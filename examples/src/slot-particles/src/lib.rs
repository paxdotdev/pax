#![allow(unused_imports)]
pub mod slot_particles;
use crate::slot_particles::SlotParticles;
use pax_kit::*;
use fireworks::*;

pax_kit::runnable! { root: crate::Example }

#[pax]
#[file("lib.pax")]
pub struct Example {}

impl Example {}
