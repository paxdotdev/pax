#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub ticks: Property<u64>,
    pub button_clicks: Property<usize>,
    pub call_sign: Property<String>,
    pub ellipse_mask_x: Property<f64>,
    pub ellipse_mask_y: Property<f64>,
    pub ellipse_mask_size: Property<f64>,
    pub path_mask_x: Property<f64>,
    pub path_mask_rotation: Property<f64>,
}

impl Example {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.call_sign.set("PAX-42".to_string());
    }

    pub fn handle_pre_render(&mut self, _ctx: &NodeContext) {
        let ticks = self.ticks.get() + 1;
        self.ticks.set(ticks);

        let t = ticks as f64 / 60.0;
        self.ellipse_mask_x.set(132.0 + 54.0 * (t * 1.2).sin());
        self.ellipse_mask_y.set(154.0 + 42.0 * (t * 0.9).cos());
        self.ellipse_mask_size.set(220.0 + 52.0 * (t * 1.6).sin());
        self.path_mask_x.set(106.0 + 48.0 * (t * 1.05).cos());
        self.path_mask_rotation.set(18.0 * (t * 0.55).sin());
    }

    pub fn increment_deal(&mut self, _ctx: &NodeContext, _args: Event<ButtonClick>) {
        self.button_clicks.set(self.button_clicks.get() + 1);
    }

    pub fn update_call_sign(&mut self, _ctx: &NodeContext, args: Event<TextboxChange>) {
        self.call_sign.set(args.text.clone());
    }
}
