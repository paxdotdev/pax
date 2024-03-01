use std::sync::atomic::{AtomicBool, Ordering};

use pax_engine::api::*;
use pax_engine::*;

use pax_std::primitives::Textbox;

use crate::model::{
    action::{Action, ActionContext, CanUndo},
    input::InputEvent,
};
#[pax]
#[main]
#[file("llm_interface/mod.pax")]
pub struct LLMInterface {
    pub visible: Property<bool>,
    pub request: Property<String>,
}

pub struct OpenLLMPrompt {
    pub require_meta: bool,
}

impl Action for OpenLLMPrompt {
    fn perform(self: Box<Self>, ctx: &mut ActionContext) -> anyhow::Result<CanUndo> {
        if !self.require_meta || ctx.app_state.keys_pressed.contains(&InputEvent::Meta) {
            OPEN_LLM_PROMPT.store(true, Ordering::Relaxed);
        }
        Ok(CanUndo::No)
    }
}

static OPEN_LLM_PROMPT: AtomicBool = AtomicBool::new(false);

impl LLMInterface {
    pub fn textbox_input(&mut self, _ctx: &NodeContext, args: ArgsTextboxInput) {
        self.request.set(args.text);
    }

    pub fn textbox_change(&mut self, ctx: &NodeContext, _args: ArgsTextboxChange) {
        let request = self.request.get();
        let mut dt = ctx.designtime.borrow_mut();
        if let Err(e) = dt.llm_request(request) {
            pax_engine::log::warn!("llm request failed: {:?}", e);
        };
        self.visible.set(false);
        self.request.set(String::new());
    }

    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        if OPEN_LLM_PROMPT.fetch_and(false, Ordering::Relaxed) {
            self.visible.set(true);
        }
    }
}
