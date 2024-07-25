use std::sync::atomic::{AtomicBool, Ordering};

use pax_engine::api::*;
use pax_engine::*;

use pax_std::primitives::*;

use crate::model::{
    action::{Action, ActionContext, orm::SerializeRequested },
    input::InputEvent,
};
#[pax]
#[file("llm_interface/mod.pax")]
pub struct LLMInterface {
    pub visible: Property<bool>,
    pub request: Property<String>,
}

pub struct OpenLLMPrompt {
    pub require_meta: bool,
}

impl Action for OpenLLMPrompt {
    fn perform(&self, ctx: &mut ActionContext) -> anyhow::Result<()> {
        SerializeRequested {}.perform(ctx)?;
        if !self.require_meta || ctx.app_state.keys_pressed.get().contains(&InputEvent::Meta) {
            OPEN_LLM_PROMPT_PROP.with(|p| p.set(true));
        }
        Ok(())
    }
}

thread_local! {
    static OPEN_LLM_PROMPT_PROP: Property<bool> = Property::new(false);
}

impl LLMInterface {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        let state = OPEN_LLM_PROMPT_PROP.with(|p| p.clone());
        let deps = [state.untyped()];
        self.visible
            .replace_with(Property::computed(move || state.get(), &deps));
    }

    pub fn textbox_change(&mut self, ctx: &NodeContext, args: Event<TextboxChange>) {
        let request = &args.text;
        let mut dt = borrow_mut!(ctx.designtime);
        if let Err(e) = dt.llm_request(request) {
            pax_engine::log::warn!("llm request failed: {:?}", e);
        };
        self.visible.set(false);
        self.request.set(String::new());
    }
}
