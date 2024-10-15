use crate::model::{
    action::tool::SetToolBehaviour,
    action::world::{SelectMode, SelectNodes},
    ProjectMode,
};
use pax_engine::api::*;
use pax_engine::*;

use pax_std::*;

use crate::{model, ProjectMsg};

#[pax]
#[engine_import_path("pax_engine")]
#[file("project_mode_toggle/mod.pax")]
pub struct ProjectModeToggle {
    pub edit_mode: Property<bool>,
    pub running_mode: Property<bool>,
    pub text: Property<String>,
}

#[allow(unused)]
impl ProjectModeToggle {
    pub fn mount(&mut self, ctx: &NodeContext) {
        // Ideally we don't do any of this, but bounds returned on first change aren't correct atm,
        // and tool state needs to be handled for for example text better. (shows old value on return)
        let project_mode = model::read_app_state(|app_state| app_state.project_mode.clone());
        let running = match ProjectMode::default() {
            ProjectMode::Edit => false,
            ProjectMode::Playing => true,
        };
        let deps = [project_mode.untyped()];

        let ctxp = ctx.clone();
        let project_mode_cp = project_mode.clone();
        self.running_mode.replace_with(Property::computed(
            move || {
                // WARNING: this won't trigger if running mode isn't used in template - hacky
                let mut dt = borrow_mut!(ctxp.designtime);
                dt.reload();
                matches!(project_mode_cp.get(), ProjectMode::Playing)
            },
            &deps,
        ));
        let ctxp = ctx.clone();
        self.edit_mode.replace_with(Property::computed(
            move || {
                // WARNING: this won't trigger if running mode isn't used in template - hacky
                let mut dt = borrow_mut!(ctxp.designtime);
                dt.reload();
                matches!(project_mode.get(), ProjectMode::Edit)
            },
            &deps,
        ));
    }

    pub fn click(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        model::perform_action(
            &SelectNodes {
                ids: &[],
                mode: SelectMode::DiscardOthers,
            },
            ctx,
        );
        let project_mode = model::read_app_state(|app_state| app_state.project_mode.clone());
        model::perform_action(&SetToolBehaviour(None), ctx);
        model::perform_action(
            &ProjectMsg(match project_mode.get() {
                ProjectMode::Edit => ProjectMode::Playing,
                ProjectMode::Playing => ProjectMode::Edit,
            }),
            ctx,
        );
    }
}
