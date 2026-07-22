#![allow(unused_imports)]

use pax_kit::*;

pub mod animated_pax_logo;
pub mod animated_pax_logo_post;
pub mod pax_logo;
pub mod pax_logo_board;
pub mod pax_logo_post;
pub use animated_pax_logo::*;
pub use animated_pax_logo_post::*;
pub use pax_logo::*;
pub use pax_logo_board::*;
pub use pax_logo_post::*;

const LOGO_DURATION_MS: u64 = 1100;

#[pax]
#[main]
#[custom(Default)]
#[file("lib.pax")]
pub struct Example {
    pub logo_playhead: Property<f64>,
    pub animation_origin_ms: Property<u64>,
    pub animation_running: Property<bool>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            logo_playhead: Property::new(0.0),
            animation_origin_ms: Property::new(0),
            animation_running: Property::new(false),
        }
    }
}

impl Example {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.restart_animation(ctx);
    }

    pub fn replay(&mut self, ctx: &NodeContext, _args: Event<Click>) {
        self.restart_animation(ctx);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        if !self.animation_running.get() {
            return;
        }

        let now = ctx.elapsed_time_millis().min(u64::MAX as u128) as u64;
        let elapsed = now
            .saturating_sub(self.animation_origin_ms.get())
            .min(LOGO_DURATION_MS);
        self.logo_playhead.set(elapsed as f64);
        if elapsed == LOGO_DURATION_MS {
            self.animation_running.set(false);
        }
    }

    fn restart_animation(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis().min(u64::MAX as u128) as u64;
        self.animation_origin_ms.set(now);
        self.logo_playhead.set(0.0);
        self.animation_running.set(true);
    }
}
