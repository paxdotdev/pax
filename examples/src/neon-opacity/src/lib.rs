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
    pub orbit_a: Property<f64>,
    pub orbit_b: Property<f64>,
    pub orbit_c: Property<f64>,
    pub orbit_d: Property<f64>,
    pub drift_a: Property<f64>,
    pub drift_b: Property<f64>,
    pub drift_c: Property<f64>,
    pub spotlight_x: Property<f64>,
    pub spotlight_y: Property<f64>,
    pub iris_x: Property<f64>,
    pub iris_y: Property<f64>,
    pub iris_size: Property<f64>,
    pub star_size: Property<f64>,
    pub star_rotation: Property<f64>,
    pub mask_hole_opacity: Property<f64>,
    pub prism_rotation: Property<f64>,
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

        self.orbit_a.set((t * 0.37).sin());
        self.orbit_b.set((t * 0.29 + 1.1).cos());
        self.orbit_c
            .set(0.62 * (t * 0.53).sin() + 0.38 * (t * 1.07).cos());
        self.orbit_d
            .set(0.58 * (t * 0.41).cos() - 0.42 * (t * 0.93).sin());

        self.drift_a
            .set(42.0 * (t * 0.63).sin() + 14.0 * (t * 1.34).cos());
        self.drift_b
            .set(34.0 * (t * 0.82).cos() + 10.0 * (t * 0.47).sin());
        self.drift_c
            .set(26.0 * (t * 1.14).sin() - 9.0 * (t * 0.58).cos());

        self.spotlight_x
            .set(46.0 + 240.0 * smooth_wave(t * 0.44 + 0.4) + 24.0 * (t * 1.21).sin());
        self.spotlight_y
            .set(56.0 + 24.0 * (t * 0.57).cos() + 12.0 * (t * 1.08).sin());
        self.iris_x
            .set(118.0 + 128.0 * wave(t * 0.78 + 0.5) + 18.0 * self.orbit_b.get());
        self.iris_y
            .set(138.0 + 96.0 * wave(t * 0.61 + 1.6) + 12.0 * self.orbit_d.get());
        self.iris_size
            .set(198.0 + 118.0 * wave(t * 1.19 + 0.2));
        self.star_size
            .set(292.0 + 48.0 * wave(t * 0.86 + 0.7));
        self.star_rotation
            .set(ticks as f64 * 1.25 + 24.0 * (t * 0.37).sin());
        self.mask_hole_opacity
            .set(0.18 + 0.56 * wave(t * 1.17 + 0.9));
        self.prism_rotation
            .set(-9.0 + 7.0 * (t * 0.22).sin() + 2.6 * (t * 0.71).cos());
        self.ribbon_rotation
            .set(-6.0 + 2.4 * (t * 0.34).sin() + 1.2 * (t * 0.82).cos());

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
