#![allow(unused_imports)]

use pax_kit::*;

pub mod calculator;
pub use calculator::*;

pub mod fireworks;
pub use fireworks::*;

pub mod space_game;
pub use space_game::*;

pub mod color_picker;
pub use color_picker::*;

mod animation;


#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
}

impl Example {
}
