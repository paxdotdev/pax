#![allow(unused_imports)]

use ::core::f64;

use pax_engine::{api::*, *};
use pax_std::*;

pub mod card;
pub use card::*;

#[pax]
#[engine_import_path("pax_engine")]
#[file("console/mod.pax")]
pub struct Console {
    pub messages: Property<Vec<Message>>,
    pub textbox: Property<String>,
    pub scroll_y: Property<f64>,
    pub enqueue_scroll_set: Property<Option<EnqueuedScrollSet>>,
    pub request_id: Property<u64>,
    pub llm_response_listener: Property<bool>,
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct Message {
    pub is_ai: bool,
    pub text: String,
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct EnqueuedScrollSet {
    pub frame: u64,
    pub scroll_y: f64,
}

impl Console {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let new_message_listener = ctx.designtime.borrow().get_llm_new_message_listener();
        let deps = &[new_message_listener.untyped()];
        let messages_cloned = self.messages.clone();
        let dt = ctx.designtime.clone();
        let current_id_cloned = self.request_id.clone();
        self.llm_response_listener.replace_with(Property::computed(
            move || {
                let mut messages = messages_cloned.get();
                let current_id = current_id_cloned.get();
                let mut design_time = dt.borrow_mut();
                let mut llm_message = design_time.get_llm_messages(current_id);
                llm_message.reverse();
                for message in llm_message {
                    messages.push(Message {
                        is_ai: true,
                        text: message.clone(),
                    });
                }
                messages_cloned.set(messages);
                false
            },
            deps,
        ));

        let frames_elapsed = ctx.frames_elapsed.clone();
        let messages_cloned = self.messages.clone();
        self.enqueue_scroll_set.replace_with(Property::computed(
            move || {
                let enqueue_scroll_set = EnqueuedScrollSet {
                    frame: frames_elapsed.get() + 1,
                    scroll_y: f64::MAX,
                };
                Some(enqueue_scroll_set)
            },
            &[messages_cloned.untyped()],
        ));
    }

    pub fn text_input(&mut self, ctx: &NodeContext, args: Event<TextboxChange>) {
        let mut messages = self.messages.get();
        let request = &args.text;
        messages.push(Message {
            is_ai: false,
            text: request.clone(),
        });
        let new_request_id = self.request_id.get() + 1;
        self.request_id.set(new_request_id);
        let mut dt = borrow_mut!(ctx.designtime);
        if let Err(e) = dt.llm_request(request, new_request_id) {
            pax_engine::log::warn!("llm request failed: {:?}", e);
        };
        self.messages.set(messages);
        self.textbox.set("".to_string());
    }

    pub fn update(&mut self, _ctx: &NodeContext) {
        if let Some(e) = self.enqueue_scroll_set.get() {
            if e.frame == _ctx.frames_elapsed.get() {
                self.scroll_y.set(e.scroll_y);
                self.enqueue_scroll_set.set(None);
            }
        }
        self.llm_response_listener.get();
    }
}
