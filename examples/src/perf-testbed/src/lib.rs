#![allow(unused_imports)]

use pax_kit::*;

pub mod virtual_elements;
pub use virtual_elements::*;


#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
}

impl Example {
}
