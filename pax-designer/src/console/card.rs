#![allow(unused_imports)]
use pax_engine::{api::*, *};
use pax_std::*;

use super::MessageType;

#[pax]
#[engine_import_path("pax_engine")]
#[file("console/card.pax")]
pub struct Card {
    pub message_type: Property<MessageType>,
    pub text: Property<String>,
}
