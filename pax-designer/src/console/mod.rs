#![allow(unused_imports)]

use ::core::f64;

use anyhow::anyhow;
use pax_engine::{api::*, *};
use pax_std::*;

pub mod card;
pub use card::*;

use crate::model;

#[pax]
#[engine_import_path("pax_engine")]
#[file("console/mod.pax")]
#[has_helpers]
pub struct Console {
    pub messages: Property<Vec<Message>>,
    pub textbox: Property<String>,
    pub scroll_y: Property<f64>,
    pub enqueue_scroll_set: Property<Option<EnqueuedScrollSet>>,
    pub request_id: Property<u64>,
    pub external_message_listener: Property<bool>,
}

#[pax]
#[engine_import_path("pax_engine")]
pub enum MessageType {
    Diff,
    LLM,
    #[default]
    Human,
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct Message {
    pub message_type: MessageType,
    pub text: String,
}

#[pax]
#[engine_import_path("pax_engine")]
pub struct EnqueuedScrollSet {
    pub frame: u64,
    pub scroll_y: f64,
}

#[helpers]
impl Console {
    fn len(vec: Vec<Message>) -> usize {
        vec.len()
    }
}

impl Console {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let new_message_listener = ctx.designtime.borrow().get_new_message_listener();
        let deps = &[new_message_listener.untyped()];
        let messages_cloned = self.messages.clone();
        let dt = ctx.designtime.clone();
        let current_id_cloned = self.request_id.clone();
        let ctx_p = ctx.clone();
        self.external_message_listener
            .replace_with(Property::computed(
                move || {
                    let mut messages = messages_cloned.get();
                    let current_id = current_id_cloned.get();
                    let mut new_messages = vec![];
                    new_message_listener.update(|messages| {
                        new_messages = std::mem::take(messages);
                    });
                    while let Some(message_type) = new_messages.pop() {
                        match message_type {
                            pax_designtime::orm::MessageType::LLMSuccess(components) => {
                                model::with_action_context(&ctx_p, |ctx| {
                                    let t = ctx.transaction("llm update");
                                    if let Err(e) = t.run(|| {
                                        let mut dt = borrow_mut!(ctx.engine_context.designtime);
                                        let orm = dt.get_orm_mut();
                                        for component in components {
                                            log::warn!(
                                                "replacing templates: {:?}",
                                                component.type_id
                                            );
                                            orm.replace_template(
                                                component.type_id,
                                                component.template.unwrap_or_default(),
                                                component.settings.unwrap_or_default(),
                                            )
                                            .map_err(|e| anyhow!(e))?;
                                        }
                                        Ok(())
                                    }) {
                                        log::warn!("failed llm message component update {:?}", e);
                                    };
                                });
                            }
                            pax_designtime::orm::MessageType::LLMPartial => {
                                let mut design_time = dt.borrow_mut();
                                let mut llm_message = design_time.get_llm_messages(current_id);
                                //llm_message.reverse();
                                for message in llm_message {
                                    messages.push(Message {
                                        message_type: MessageType::LLM,
                                        text: message.clone(),
                                    });
                                }
                                messages_cloned.set(messages.clone());
                            }
                            pax_designtime::orm::MessageType::Serialization(msg) => {
                                messages.push(Message {
                                    message_type: MessageType::Diff,
                                    text: msg.clone(),
                                });
                                messages_cloned.set(messages.clone());
                            }
                        }
                    }
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
            message_type: MessageType::Human,
            text: request.clone(),
        });
        self.messages.set(messages);
        self.textbox.set("".to_string());
        let new_request_id = self.request_id.get() + 1;
        self.request_id.set(new_request_id);
        let mut dt = borrow_mut!(ctx.designtime);
        ctx.screenshot(new_request_id as u32);
        if let Err(e) = dt.llm_request(request, new_request_id) {
            pax_engine::log::warn!("llm request failed: {:?}", e);
        };
    }

    pub fn update(&mut self, _ctx: &NodeContext) {
        if let Some(e) = self.enqueue_scroll_set.get() {
            if e.frame == _ctx.frames_elapsed.get() {
                self.scroll_y.set(e.scroll_y);
                self.enqueue_scroll_set.set(None);
            }
        }
        self.external_message_listener.get();
    }
}
