#![allow(unused_imports)]
use pax_engine::{api::*, *};
use pax_std::*;

#[pax]
#[engine_import_path("pax_engine")]
#[file("console/card.pax")]
pub struct Card {
    pub is_ai: Property<bool>,
    pub text: Property<String>,
}
