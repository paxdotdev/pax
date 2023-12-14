#![allow(unused_imports)]

use pax_lang::*;
use pax_lang::api::*;

pub mod hud;
pub mod controls;

use crate::hud::Hud;
use crate::controls::Controls;

//TODO: import userland project as `Renderer` for `<Renderer />`

#[derive(Pax)]
#[main]
#[file("lib.pax")]
pub struct PaxDesigner {}

