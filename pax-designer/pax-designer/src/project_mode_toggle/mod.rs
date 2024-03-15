use crate::model::ProjectMode;
use pax_engine::api::*;
use pax_engine::*;

use pax_std::primitives::Rectangle;

use crate::{model, ProjectMsg};
use pax_std::primitives::Group;
use pax_std::primitives::Text;

use pax_std::primitives::Path;

#[pax]
#[file("project_mode_toggle/mod.pax")]
pub struct ProjectModeToggle {
    pub edit_mode: Property<bool>,
    pub running_mode: Property<bool>,
    pub text: Property<String>,
}

impl ProjectModeToggle {
    pub fn mount(&mut self, _ctx: &NodeContext) {
        self.running_mode.set(false);
        self.edit_mode.set(true);
    }

    pub fn click(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        let curr = *self.edit_mode.get();
        self.edit_mode.set(!curr);
        self.running_mode.set(curr);
        let mode = match self.edit_mode.get() {
            true => {
                let mut dt = ctx.designtime.borrow_mut();
                dt.reload_edit();
                ProjectMode::Edit
            }
                ,
            false => {
                let mut dt = ctx.designtime.borrow_mut();
                dt.reload_play();
                ProjectMode::Playing
            },
        };
        model::perform_action(ProjectMsg::SetMode(mode), ctx);
    }
}
