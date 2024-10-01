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
      self.messages.set(vec![
        Message {
          is_ai: false,
          text: "Hello, world!".to_string(),
        },
        Message {
          is_ai: true,
          text: "Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium, totam rem aperiam, eaque ipsa quae ab illo inventore veritatis et quasi architecto beatae vitae dicta sunt explicabo. Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit aut fugit, sed quia consequuntur magni dolores eos qui ratione voluptatem sequi nesciunt. Neque porro quisquam est, qui dolorem ipsum quia dolor sit amet, consectetur, adipisci velit, sed quia non numquam eius modi tempora incidunt ut labore et dolore magnam aliquam quaerat voluptatem. Ut enim ad minima veniam, quis nostrum exercitationem ullam corporis suscipit laboriosam, nisi ut aliquid ex ea commodi consequatur? Quis autem vel eum iure reprehenderit qui in ea voluptate velit esse quam nihil molestiae consequatur, vel illum qui dolorem eum fugiat quo voluptas nulla pariatur? ".to_string(),
        },
      ]);
    }

    fn scroll_to_end(&mut self, ctx: &NodeContext) {
      let enqueue_scroll_set = EnqueuedScrollSet {
        frame: ctx.frames_elapsed.get() + 1,
        scroll_y: f64::MAX,
      };
      self.enqueue_scroll_set.set(Some(enqueue_scroll_set));
    }

    pub fn text_input(&mut self, ctx: &NodeContext, args: Event<TextboxChange>) {
      let mut messages = self.messages.get();
      messages.push(Message {
          is_ai: false,
          text: args.text.clone(),
      });
      self.messages.set(messages);
      self.scroll_to_end(ctx);
      self.textbox.set("".to_string());
    }

    pub fn update(&mut self, _ctx: &NodeContext) {
      if let Some(e) = self.enqueue_scroll_set.get() {
        if e.frame == _ctx.frames_elapsed.get() {
          self.scroll_y.set(e.scroll_y);
          self.enqueue_scroll_set.set(None);
        }
      }
    }
}
