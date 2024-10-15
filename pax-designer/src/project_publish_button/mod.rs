use crate::model::{
    action::tool::SetToolBehaviour,
    action::world::{SelectMode, SelectNodes},
    app_state::ProjectMode,
};
use pax_engine::api::*;
use pax_engine::node_layout::calculate_transform_and_bounds;
use pax_engine::pax_manifest::server::PublishResponse;
use pax_engine::*;

use pax_std::*;

use crate::{model, ProjectMsg};

#[pax]
#[engine_import_path("pax_engine")]
#[file("project_publish_button/mod.pax")]
pub struct ProjectPublishButton {
    pub is_publishing: Property<bool>,
    pub publish_success: Property<bool>,
    pub publish_error: Property<bool>,
    pub github_pr_url: Property<String>,
    pub publish_error_message: Property<String>,
    pub publish_state: Property<Option<PublishResponse>>,
}

#[allow(unused)]
impl ProjectPublishButton {
    pub fn mount(&mut self, ctx: &NodeContext) {
        let publish_state_cloned = borrow!(ctx.designtime).publish_state.clone();
        let deps = [publish_state_cloned.untyped()];

        let publish_success_cloned = self.publish_success.clone();
        let github_pr_url_cloned = self.github_pr_url.clone();
        let publish_error_cloned = self.publish_error.clone();
        let publish_error_message_cloned = self.publish_error_message.clone();
        let is_publishing_cloned = self.is_publishing.clone();

        self.publish_state.replace_with(Property::computed(
            move || match publish_state_cloned.get() {
                Some(PublishResponse::Success(success)) => {
                    is_publishing_cloned.set(false);
                    publish_success_cloned.set(true);
                    github_pr_url_cloned.set(success.pull_request_url.clone());
                    Some(PublishResponse::Success(success))
                }
                Some(PublishResponse::Error(error)) => {
                    is_publishing_cloned.set(false);
                    publish_error_cloned.set(true);
                    publish_error_message_cloned.set(error.message.clone());
                    Some(PublishResponse::Error(error))
                }
                _ => None,
            },
            &deps,
        ));
    }

    pub fn tick(&mut self, ctx: &NodeContext) {
        self.publish_state.get(); //force lazy eval
    }

    pub fn click(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        let mut dt = borrow_mut!(ctx.designtime);
        dt.publish_state.set(None);
        dt.publish_project();
        self.is_publishing.set(true);
    }

    pub fn reset(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        self.is_publishing.set(false);
        self.publish_success.set(false);
        self.publish_error.set(false);
    }
}
