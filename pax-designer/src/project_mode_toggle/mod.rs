use crate::model::{tools::SelectNodes, ProjectMode};
use pax_engine::api::*;
use pax_engine::*;

use pax_std::*;

use crate::{model, ProjectMsg};

#[pax]
#[file("project_mode_toggle/mod.pax")]
pub struct ProjectModeToggle {
    pub edit_mode: Property<bool>,
    pub running_mode: Property<bool>,
    pub text: Property<String>,
}

#[allow(unused)]
impl ProjectModeToggle {
    pub fn mount(&mut self, _ctx: &NodeContext) {
        let running = match ProjectMode::default() {
            ProjectMode::Edit => false,
            ProjectMode::Playing => true,
        };
        self.running_mode.set(running);
        self.edit_mode.set(!running);
    }

    pub fn click(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        let curr = self.edit_mode.get();
        self.edit_mode.set(!curr);
        self.running_mode.set(curr);
        let mode = match self.edit_mode.get() {
            true => {
                let mut dt = borrow_mut!(ctx.designtime);
                dt.reload_edit();
                ProjectMode::Edit
            }
            false => {
                let mut dt = borrow_mut!(ctx.designtime);
                dt.reload_play();
                ProjectMode::Playing
            }
        };
        // Ideally we don't do this, but bounds returned on first change aren't correct atm
        model::perform_action(
            &SelectNodes {
                ids: &[],
                force_deselection_of_others: true,
            },
            ctx,
        );
        model::perform_action(&ProjectMsg(mode), ctx);
    }
}
