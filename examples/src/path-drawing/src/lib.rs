#![allow(unused_imports)]

use pax_kit::*;

pub mod svg_fixtures;
pub use svg_fixtures::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {}

