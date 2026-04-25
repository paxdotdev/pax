#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub board_gain: Property<f64>,
    pub palette_mix: Property<f64>,
    pub tap_count: Property<usize>,
    pub controls_armed: Property<bool>,
    pub primary_action_label: Property<String>,
    pub armed_label: Property<String>,
}

impl Example {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.board_gain.set(0.62);
        self.palette_mix.set(0.38);
        self.tap_count.set(0);
        self.controls_armed.set(true);
        self.refresh_labels();
    }

    pub fn record_control_tap(&mut self, _ctx: &NodeContext, _args: Event<ButtonClick>) {
        self.tap_count.set(self.tap_count.get() + 1);
        self.refresh_labels();
    }

    pub fn toggle_controls_armed(&mut self, _ctx: &NodeContext, _args: Event<ButtonClick>) {
        self.controls_armed.set(!self.controls_armed.get());
        self.refresh_labels();
    }

    fn refresh_labels(&mut self) {
        self.primary_action_label
            .set(format!("PING {}", self.tap_count.get()));
        self.armed_label.set(
            if self.controls_armed.get() {
                "ARMED".to_string()
            } else {
                "SAFE".to_string()
            },
        );
    }
}
