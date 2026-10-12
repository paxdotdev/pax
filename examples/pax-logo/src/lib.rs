#![allow(unused_imports)]

use pax_kit::*;

pax_kit::runnable! { root: crate::Example }

#[pax]
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
        let progress = self.logo_progress.local();
        progress.cancel_transitions();
        progress.set(0.0);
        self.play_logo();
    }

    pub fn scrub_logo(&mut self, _ctx: &NodeContext, event: Event<SliderChange>) {
        let progress = self.logo_progress.local();
        progress.cancel_transitions();
        progress.set(event.value);
    }

    fn play_logo(&self) {
        self.logo_progress.local().ease_to(
            1.0,
            Duration::Milliseconds(1440.into()),
            EasingCurve::Linear,
        );
    }
}
