use pax_kit::*;

pax_kit::runnable! { root: crate::NativeScrollSnap }

#[pax]
#[file("lib.pax")]
pub struct NativeScrollSnap {
    pub points: Property<Vec<Size>>,
    pub mode: Property<String>,
    pub horizontal: Property<f64>,
    pub vertical: Property<f64>,
    pub viewport_width: Property<f64>,
    pub viewport_height: Property<f64>,
    pub clicks: Property<usize>,
    pub warm: Property<bool>,
    pub state: Property<String>,
}

impl NativeScrollSnap {
    pub fn on_mount(&mut self, _: &NodeContext) {
        self.reset_values();
    }

    fn page_points(&mut self) {
        self.points.set(
            [0.0, 100.0, 200.0]
                .map(|v| Size::Percent(v.into()))
                .to_vec(),
        );
        self.mode.set("pages".into());
    }

    fn reset_values(&mut self) {
        self.warm.set(false);
        self.viewport_width.set(450.0);
        self.viewport_height.set(240.0);
        self.page_points();
        self.horizontal.set(0.0);
        self.vertical.set(0.0);
    }

    pub fn reset(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.reset_values();
    }

    pub fn replace(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.points.set(
            [0.0, 125.0, 325.0, 625.0]
                .map(|v| Size::Pixels(v.into()))
                .to_vec(),
        );
        self.mode.set("pixels".into());
    }

    pub fn clear(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.points.set(Vec::new());
        self.mode.set("free".into());
    }

    pub fn restore(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.page_points();
    }

    pub fn resize(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        let compact = self.viewport_width.get() > 400.0;
        self.viewport_width.set(if compact { 360.0 } else { 450.0 });
        self.viewport_height
            .set(if compact { 180.0 } else { 240.0 });
    }

    pub fn jump(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        // Deliberately between anchors: qualify each native platform's policy.
        self.horizontal.set(173.0);
        self.vertical.set(173.0);
    }

    pub fn switch_colors(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.warm.set(!self.warm.get());
    }

    pub fn click_page(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.clicks.set(self.clicks.get() + 1);
    }

    pub fn pre_render(&mut self, _: &NodeContext) {
        self.state.set_if_neq(format!(
            "x={:.1} y={:.1} mode={} points={} viewport={:.0}x{:.0} clicks={} warm={}",
            self.horizontal.get(),
            self.vertical.get(),
            self.mode.get(),
            self.points.get().len(),
            self.viewport_width.get(),
            self.viewport_height.get(),
            self.clicks.get(),
            self.warm.get(),
        ));
    }
}
