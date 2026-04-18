#![allow(unused_imports)]

use pax_kit::*;

pub mod dot_row;
pub use dot_row::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {}
