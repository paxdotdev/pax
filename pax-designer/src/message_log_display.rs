use std::collections::VecDeque;

use crate::model::{tools::SelectNodes, ProjectMode};
use pax_engine::api::*;
use pax_engine::*;

use pax_std::*;

pub struct DesignerLogMsg {
    message: String,
    // add other metadata? (size of banner, stay duration, etc)
}

impl DesignerLogMsg {
    pub fn message(message: String) -> Self {
        Self { message }
    }
}

thread_local! {
    static DESIGNER_LOG: std::cell::RefCell<VecDeque<DesignerLogMsg>> = std::cell::RefCell::default();
}

pub fn log(message: DesignerLogMsg) {
    DESIGNER_LOG.with_borrow_mut(|log| {
        if log.back().is_some_and(|m| m.message == message.message) {
            return;
        }
        log.push_back(message)
    })
}

#[pax]
#[file("message_log_display.pax")]
pub struct MessageLogDisplay {
    pub message: Property<String>,
    pub message_visible: Property<bool>,
    pub timer: Property<usize>,
    pub opacity: Property<u8>,
}

impl MessageLogDisplay {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        let timer = self.timer.clone();
        let deps = [timer.untyped()];
        self.opacity.replace_with(Property::computed(
            move || timer.get().min(255) as u8,
            &deps,
        ));
    }
    pub fn pre_render(&mut self, _ctx: &NodeContext) {
        let time = self.timer.get();
        self.timer.set(time.saturating_sub(1));

        if time == 0 {
            if let Some(message) = DESIGNER_LOG.with_borrow_mut(|log| log.pop_front()) {
                self.timer.set(600); // TODO make this real time instead of frames, long on 60fps, short on 120fps
                self.message.set(message.message);
                self.message_visible.set(true);
            } else {
                if self.message_visible.get() {
                    self.message_visible.set(false);
                }
            };
        }
    }

    pub fn mouse_move(&self, _ctx: &NodeContext, _event: Event<MouseMove>) {
        if self.timer.get() != 0 {
            self.timer.set(600);
        }
    }
}
