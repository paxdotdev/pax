#![allow(unused_imports)]

use pax_kit::*;

mod font_comparison_row;
pub mod svg_fixtures;
pub use font_comparison_row::*;
pub use svg_fixtures::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {}
