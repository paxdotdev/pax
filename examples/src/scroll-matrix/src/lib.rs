#![allow(unused_imports)]

use pax_kit::*;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub outer_scroll_y: Property<f64>,
    pub vertical_probe_y: Property<f64>,
    pub horizontal_probe_x: Property<f64>,
    pub notes: Property<String>,
    pub selected_route: Property<u32>,
    pub gain: Property<f64>,
    pub armed: Property<bool>,
    pub band: Property<u32>,
    pub tap_count: Property<usize>,
    pub tap_label: Property<String>,
    pub route_label: Property<String>,
    pub armed_label: Property<String>,
}

impl Example {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.outer_scroll_y.set(0.0);
        self.vertical_probe_y.set(0.0);
        self.horizontal_probe_x.set(0.0);
        self.notes.set("deckhand-17".to_string());
        self.selected_route.set(1);
        self.gain.set(0.68);
        self.armed.set(true);
        self.band.set(0);
        self.refresh_labels();
    }

    pub fn handle_pre_render(&mut self, _ctx: &NodeContext) {
        self.refresh_labels();
    }

    pub fn record_tap(&mut self, _ctx: &NodeContext, _args: Event<ButtonClick>) {
        self.tap_count.set(self.tap_count.get() + 1);
        self.refresh_labels();
    }

    pub fn update_notes(&mut self, _ctx: &NodeContext, args: Event<TextboxChange>) {
        self.notes.set(args.text.clone());
    }

    pub fn update_armed(&mut self, _ctx: &NodeContext, args: Event<CheckboxChange>) {
        self.armed.set(args.checked);
        self.refresh_labels();
    }

    fn refresh_labels(&mut self) {
        let route = match self.selected_route.get() {
            0 => "LOOP",
            1 => "NESTED",
            2 => "MASKED",
            _ => "TILE",
        };
        self.route_label.set_if_neq(route.to_string());
        self.armed_label
            .set_if_neq(if self.armed.get() { "ARMED" } else { "SAFE" }.to_string());
        self.tap_label
            .set_if_neq(format!("STAMP {}", self.tap_count.get()));
    }
}
