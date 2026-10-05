#![allow(unused_imports)]

use pax_kit::*;

pub mod dot_row;
pub use dot_row::*;

pax_kit::runnable! { root: crate::Example }

#[pax]
#[file("lib.pax")]
pub struct Example {}
