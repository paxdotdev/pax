use crate::model::{
    action::tool::SetToolBehaviour,
    tools::{SelectMode, SelectNodes},
    ProjectMode,
};
use pax_engine::api::*;
use pax_engine::*;

use pax_std::*;

use crate::{model, ProjectMsg};

#[pax]
#[engine_import_path("pax_engine")]
#[file("project_publish_button/mod.pax")]
pub struct ProjectPublishButton {
    pub publish_info: Property<bool>,
}

#[allow(unused)]
impl ProjectPublishButton {
    pub fn click(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        let mut dt = borrow_mut!(ctx.designtime);
        dt.publish_project();
        self.publish_info.set(true);
    }

    pub fn reset(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        self.publish_info.set(false);
    }
}
