#![allow(unused_imports)]

use pax_kit::*;

const PANEL_FULL_HEIGHT: f64 = 158.0;
const PANEL_TARGET_SIZE: f64 = 1.0;
const PANEL_TOGGLE_FRAMES: u64 = 22;
const PANEL_HIDE_BUFFER_FRAMES: u64 = 2;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub content_width: Property<f64>,
    pub ticks: Property<u64>,
    pub call_sign: Property<String>,
    pub selected_route: Property<u32>,
    pub gain: Property<f64>,
    pub armed: Property<bool>,
    pub mode: Property<u32>,
    pub panel_visible: Property<bool>,
    pub panel_motion: Property<f64>,
    pub panel_cell_height: Property<f64>,
    pub panel_scale_x: Property<f64>,
    pub panel_scale_y: Property<f64>,
    pub panel_origin_y: Property<Size>,
    pub panel_opacity: Property<f64>,
    pub panel_hide_frame: Property<u64>,
    pub panel_last_checked: Property<bool>,
    pub tap_count: Property<usize>,
    pub tap_label: Property<String>,
    pub armed_label: Property<String>,
    pub route_label: Property<String>,
}

impl Example {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let bounds_parent = ctx.bounds_parent.clone();
        self.content_width.replace_with(Property::computed(
            move || {
                let (width, _) = bounds_parent.get();
                (width - 32.0).clamp(320.0, 980.0)
            },
            &[ctx.bounds_parent.untyped()],
        ));
        self.call_sign.set("ios-lab-26".to_string());
        self.selected_route.set(1);
        self.gain.set(0.62);
        self.armed.set(true);
        self.mode.set(0);
        self.panel_visible.set(true);
        self.panel_motion.set(0.0);
        self.panel_opacity.set(1.0);
        self.panel_hide_frame.set(0);
        self.panel_last_checked.set(true);
        self.bind_panel_motion_properties();
        self.refresh_labels();
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        self.ticks.set(self.ticks.get() + 1);
        let checked = self.armed.get();
        if checked != self.panel_last_checked.get() {
            self.animate_panel(checked, ctx.elapsed_frames.get());
            self.panel_last_checked.set(checked);
        }

        let hide_frame = self.panel_hide_frame.get();
        if hide_frame != 0 && ctx.elapsed_frames.get() >= hide_frame {
            self.panel_opacity.set(0.0);
            self.panel_visible.set(false);
            self.panel_hide_frame.set(0);
        }
        self.refresh_labels();
    }

    pub fn record_tap(&mut self, _ctx: &NodeContext, _args: Event<ButtonClick>) {
        self.tap_count.set(self.tap_count.get() + 1);
        self.refresh_labels();
    }

    pub fn update_call_sign(&mut self, _ctx: &NodeContext, args: Event<TextboxChange>) {
        self.call_sign.set(args.text.clone());
    }

    pub fn update_armed(&mut self, ctx: &NodeContext, args: Event<CheckboxChange>) {
        self.armed.set(args.checked);
        self.animate_panel(args.checked, ctx.elapsed_frames.get());
        self.panel_last_checked.set(args.checked);
        self.refresh_labels();
    }

    fn animate_panel(&mut self, show: bool, current_frame: u64) {
        if show {
            self.panel_visible.set(true);
            self.panel_hide_frame.set(0);
            self.panel_opacity.set(1.0);
            self.panel_motion
                .ease_to(0.0, PANEL_TOGGLE_FRAMES, EasingCurve::InQuad);
        } else {
            self.panel_motion
                .ease_to(1.0, PANEL_TOGGLE_FRAMES, EasingCurve::InQuad);
            self.panel_hide_frame
                .set(current_frame + PANEL_TOGGLE_FRAMES + PANEL_HIDE_BUFFER_FRAMES);
        }
    }

    fn bind_panel_motion_properties(&mut self) {
        let motion = self.panel_motion.clone();
        let motion_calc = motion.clone();
        self.panel_cell_height.replace_with(Property::computed(
            move || panel_height_for_progress(motion_calc.get()),
            &[motion.untyped()],
        ));

        let motion = self.panel_motion.clone();
        let width = self.content_width.clone();
        let motion_calc = motion.clone();
        let width_calc = width.clone();
        self.panel_scale_x.replace_with(Property::computed(
            move || {
                let progress = normalized_motion(motion_calc.get());
                let min_scale =
                    (PANEL_TARGET_SIZE / width_calc.get().max(PANEL_TARGET_SIZE)).clamp(0.0, 1.0);
                lerp(1.0, min_scale, progress)
            },
            &[motion.untyped(), width.untyped()],
        ));

        let motion = self.panel_motion.clone();
        let motion_calc = motion.clone();
        self.panel_scale_y.replace_with(Property::computed(
            move || panel_height_for_progress(motion_calc.get()) / PANEL_FULL_HEIGHT,
            &[motion.untyped()],
        ));

        let motion = self.panel_motion.clone();
        let motion_calc = motion.clone();
        self.panel_origin_y.replace_with(Property::computed(
            move || {
                let progress = normalized_motion(motion_calc.get());
                let height = panel_height_for_progress(progress);
                let absolute_origin_y = lerp(PANEL_FULL_HEIGHT * 0.5, 0.0, progress);

                // `y` is authored as a percent, but the parent cell height is animated too.
                // Convert the desired absolute drain path back to percent to avoid ease*ease motion.
                Size::Percent(((absolute_origin_y / height.max(PANEL_TARGET_SIZE)) * 100.0).into())
            },
            &[motion.untyped()],
        ));
    }

    fn refresh_labels(&mut self) {
        let route = match self.selected_route.get() {
            0 => "REGULAR",
            1 => "CLEAR",
            2 => "NESTED",
            _ => "FALLBACK",
        };
        self.route_label.set(route.to_string());
        self.armed_label
            .set(if self.armed.get() { "ARMED" } else { "SAFE" }.to_string());
        self.tap_label
            .set(format!("Pulse {}", self.tap_count.get()));
    }
}

fn normalized_motion(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

fn panel_height_for_progress(progress: f64) -> f64 {
    lerp(
        PANEL_FULL_HEIGHT,
        PANEL_TARGET_SIZE,
        normalized_motion(progress),
    )
}

fn lerp(start: f64, end: f64, progress: f64) -> f64 {
    start + ((end - start) * normalized_motion(progress))
}
