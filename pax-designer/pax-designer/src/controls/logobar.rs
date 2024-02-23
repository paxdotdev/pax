use pax_engine::api::*;
use pax_engine::*;

use pax_std::primitives::{Image, Text};

use crate::model;

#[pax]
#[file("controls/logobar.pax")]
pub struct Logobar {}

impl Logobar {
    pub fn handle_logo_click(&mut self, ctx: &NodeContext, _args: ArgsClick) {
        model::read_app_state(|app_state| {
            let mut dt = ctx.designtime.borrow_mut();
            if let Err(e) = dt.send_component_update(&app_state.selected_component_id) {
                pax_engine::log::error!("failed to save component to file: {:?}", e);
            }
        });
    }
}
