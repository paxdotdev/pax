#![allow(unused_imports)]

use pax_kit::*;

pub mod animated_pax_logo;
pub mod animated_pax_logo_banner;
pub mod animated_pax_logo_post;
pub mod pax_logo;
pub mod pax_logo_board;
pub mod pax_logo_post;
pub use animated_pax_logo::*;
pub use animated_pax_logo_banner::*;
pub use animated_pax_logo_post::*;
pub use pax_logo::*;
pub use pax_logo_board::*;
pub use pax_logo_post::*;

#[pax]
#[main]
#[custom(Default)]
#[file("lib.pax")]
pub struct Example {
    pub logo_progress: Property<f64>,
    pub background_mode: Property<LogoBackgroundMode>,
}

impl Default for Example {
    fn default() -> Self {
        Self {
            logo_progress: Property::new(0.0),
            background_mode: Property::new(LogoBackgroundMode::Light),
        }
    }
}

impl Example {
    pub fn toggle_background(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.background_mode.set(match self.background_mode.get() {
            LogoBackgroundMode::Light => LogoBackgroundMode::Dark,
            LogoBackgroundMode::Dark => LogoBackgroundMode::Light,
        });
    }

    pub fn handle_mount(&mut self, _ctx: &NodeContext) {
        self.play_logo();
    }

    pub fn replay(&mut self, _ctx: &NodeContext, _args: Event<Click>) {
        self.logo_progress.cancel_transitions();
        self.logo_progress.set(0.0);
        self.play_logo();
    }

    pub fn scrub_logo(&mut self, _ctx: &NodeContext, event: Event<SliderChange>) {
        self.logo_progress.cancel_transitions();
        self.logo_progress.set(event.value);
    }

    fn play_logo(&self) {
        self.logo_progress.ease_to(
            1.0,
            Duration::Milliseconds(1440.into()),
            EasingCurve::Linear,
        );
    }
}
