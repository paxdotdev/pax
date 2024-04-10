use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::primitives::*;
use pax_std::types::StackerDirection;

use std::sync::atomic::AtomicI32;

#[pax]
#[main]
#[file("grids.pax")]
pub struct Grids {}
