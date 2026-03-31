#![allow(unused_imports)]

use pax_kit::*;

fn wave(theta: f64) -> f64 {
    ((theta.sin() + 1.0) * 0.5).clamp(0.0, 1.0)
}

fn smooth_wave(theta: f64) -> f64 {
    let value = wave(theta);
    value * value * (3.0 - 2.0 * value)
}

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub ticks: Property<u64>,
    pub callsign: Property<String>,
    pub selected_mode: Property<u32>,
    pub spectrum_band: Property<u32>,
    pub beam_gain: Property<f64>,
    pub engage_count: Property<usize>,
    pub stabilizers: Property<bool>,
    pub monolith_opacity: Property<f64>,
    pub inner_glass_opacity: Property<f64>,
    pub mask_panel_opacity: Property<f64>,
    pub lattice_left_opacity: Property<f64>,
    pub lattice_right_opacity: Property<f64>,
    pub halo_opacity: Property<f64>,
    pub drift_a: Property<f64>,
    pub drift_b: Property<f64>,
    pub drift_c: Property<f64>,
    pub spotlight_x: Property<f64>,
    pub spotlight_y: Property<f64>,
    pub iris_x: Property<f64>,
    pub iris_y: Property<f64>,
    pub iris_size: Property<f64>,
    pub star_rotation: Property<f64>,
    pub mask_hole_opacity: Property<f64>,
    pub ribbon_rotation: Property<f64>,
    pub mode_label: Property<String>,
    pub beam_label: Property<String>,
    pub engage_label: Property<String>,
    pub spectrum_label: Property<String>,
    pub stabilizer_label: Property<String>,
}

impl Example {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.callsign.set("LUX-17".to_string());
        self.selected_mode.set(0);
        self.spectrum_band.set(1);
        self.beam_gain.set(0.68);
        self.stabilizers.set(true);
        self.refresh_labels();
    }

    pub fn handle_pre_render(&mut self, _ctx: &NodeContext) {
        let ticks = self.ticks.get() + 1;
        self.ticks.set(ticks);

        let t = ticks as f64 / 60.0;

        self.monolith_opacity
            .set(0.67 + 0.16 * wave(t * 0.58 + 0.1));
        self.inner_glass_opacity
            .set(0.56 + 0.24 * wave(t * 1.08 + 0.8));
        self.mask_panel_opacity
            .set(0.58 + 0.24 * wave(t * 0.92 + 1.1));
        self.lattice_left_opacity
            .set(0.38 + 0.26 * wave(t * 0.76 + 0.2));
        self.lattice_right_opacity
            .set(0.34 + 0.30 * wave(t * 0.87 + 1.6));
        self.halo_opacity
            .set(0.46 + 0.18 * wave(t * 0.43 + 2.0));

        self.drift_a.set(28.0 * (t * 0.63).sin());
        self.drift_b.set(22.0 * (t * 0.82).cos());
        self.drift_c.set(18.0 * (t * 1.14).sin());

        self.spotlight_x
            .set(26.0 + 170.0 * smooth_wave(t * 0.44 + 0.4));
        self.spotlight_y.set(52.0 + 18.0 * (t * 0.57).cos());
        self.iris_x.set(96.0 + 138.0 * wave(t * 0.78 + 0.5));
        self.iris_y.set(124.0 + 82.0 * wave(t * 0.61 + 1.6));
        self.iris_size.set(154.0 + 92.0 * wave(t * 1.19 + 0.2));
        self.star_rotation
            .set(ticks as f64 * 1.25 + 18.0 * (t * 0.37).sin());
        self.mask_hole_opacity
            .set(0.18 + 0.56 * wave(t * 1.17 + 0.9));
        self.ribbon_rotation.set(-4.5 + 1.8 * (t * 0.34).sin());

        self.refresh_labels();
    }

    pub fn increment_engage(&mut self, _ctx: &NodeContext, _args: Event<ButtonClick>) {
        self.engage_count.set(self.engage_count.get() + 1);
        self.refresh_labels();
    }

    pub fn update_callsign(&mut self, _ctx: &NodeContext, args: Event<TextboxChange>) {
        self.callsign.set(args.text.clone());
    }

    pub fn update_stabilizers(&mut self, _ctx: &NodeContext, args: Event<CheckboxChange>) {
        self.stabilizers.set(args.checked);
        self.refresh_labels();
    }

    fn refresh_labels(&mut self) {
        let mode_label = match self.selected_mode.get() {
            0 => "SPECTRAL",
            1 => "NATIVE",
            _ => "MASK",
        };
        let spectrum_label = match self.spectrum_band.get() {
            0 => "AURORA",
            1 => "NOCTIS",
            _ => "PULSE",
        };

        self.mode_label.set(mode_label.to_string());
        self.spectrum_label.set(spectrum_label.to_string());
        self.beam_label
            .set(format!("{:03.0}%", self.beam_gain.get() * 100.0));
        self.engage_label
            .set(format!("ENGAGE {}", self.engage_count.get()));
        self.stabilizer_label.set(
            if self.stabilizers.get() {
                "ONLINE"
            } else {
                "OFFLINE"
            }
            .to_string(),
        );
    }
}
