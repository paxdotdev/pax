#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[file("card.pax")]
pub struct BasicCard {
    pub is_ai: Property<bool>,
    pub text: Property<String>,
}