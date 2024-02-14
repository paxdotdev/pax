use pax_engine::api::*;
use pax_engine::*;

use pax_std::primitives::{Text, Image};

#[pax]
#[file("controls/logobar.pax")]
pub struct Logobar {}


impl Logobar {
    pub fn handle_logo_click(&mut self, ctx: &NodeContext, _args: ArgsClick) {
        let mut dt = ctx.designtime.borrow_mut();
        dt.send_component_update("pax_designer::pax_reexports::designer_project::Example");
    }

}
