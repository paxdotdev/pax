use pax_engine::api::*;
use pax_engine::*;
use pax_std::*;

use crate::model;
use crate::model::action::orm::SerializeRequested;

#[pax]
#[file("controls/logobar.pax")]
pub struct Logobar {}

impl Logobar {
    pub fn handle_logo_click(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        model::perform_action(&SerializeRequested {}, ctx);
    }
}
