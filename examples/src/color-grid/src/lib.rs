#![allow(unused_imports)]

use pax_lang::*;
use pax_lang::api::*;
use pax_std::primitives::*;
use pax_std::types::*;
use pax_std::types::text::*;
use pax_std::components::*;
use pax_std::components::Stacker;

#[derive(Pax)]
#[main]
#[file("color-grid.pax")]
pub struct ColorGrid {
}