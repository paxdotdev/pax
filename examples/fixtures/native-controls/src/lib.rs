use pax_kit::*;

pax_kit::runnable! { root: crate::NativeControls }

#[pax]
#[file("lib.pax")]
pub struct NativeControls {
    pub options: Property<Vec<String>>,
    pub selected: Property<u32>,
    pub radio: Property<u32>,
    pub other_radio: Property<u32>,
    pub value: Property<f64>,
    pub minimum: Property<f64>,
    pub maximum: Property<f64>,
    pub step: Property<f64>,
    pub slider_events: Property<usize>,
    pub corner_clicks: Property<usize>,
    pub scroll_y: Property<f64>,
    pub radius: Property<f64>,
    pub viewport_height: Property<f64>,
    pub visible: Property<bool>,
    pub state: Property<String>,
}

impl NativeControls {
    pub fn on_mount(&mut self, _: &NodeContext) {
        self.reset_values();
    }

    fn reset_values(&mut self) {
        self.options
            .set(vec!["Alpha".into(), "Beta".into(), "Gamma".into()]);
        self.selected.set(1);
        self.radio.set(0);
        self.other_radio.set(2);
        self.minimum.set(-2.0);
        self.maximum.set(2.0);
        self.step.set(0.25);
        self.value.set(0.5);
        self.scroll_y.set(0.0);
        self.radius.set(48.0);
        self.viewport_height.set(250.0);
        self.visible.set(true);
        // Deliberately retain event counts: resetting properties must not
        // manufacture native input events.
        self.refresh();
    }

    pub fn reset(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.reset_values();
    }

    pub fn replace_options(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.options.set(vec!["Delta".into(), "Epsilon".into()]);
        self.selected.set(1);
        self.radio.set(1);
    }

    pub fn clear_options(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.options.set(Vec::new());
        self.selected.set(0);
        self.radio.set(0);
    }

    pub fn change_range(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        // Both endpoints move past the old range in one engine update.
        self.minimum.set(10.0);
        self.maximum.set(20.0);
        self.step.set(2.0);
        self.value.set(14.0);
    }

    pub fn off_grid(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.value
            .set(if self.minimum.get() < 0.0 { 0.6 } else { 14.6 });
    }

    pub fn toggle_controls(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.visible.set(!self.visible.get());
    }

    pub fn toggle_viewport(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.viewport_height
            .set(if self.viewport_height.get() > 0.0 {
                0.0
            } else {
                250.0
            });
    }

    pub fn cycle_radius(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.radius.set(match self.radius.get() {
            48.0 => 8.0,
            8.0 => 24.0,
            _ => 48.0,
        });
    }

    pub fn slider_changed(&mut self, _: &NodeContext, _: Event<SliderChange>) {
        self.slider_events.set(self.slider_events.get() + 1);
    }

    pub fn corner_clicked(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.corner_clicks.set(self.corner_clicks.get() + 1);
    }

    pub fn pre_render(&mut self, _: &NodeContext) {
        self.refresh();
    }

    fn refresh(&mut self) {
        self.state.set_if_neq(format!(
            "value={:.2} range={:.0}..{:.0} step={:.2} events={} dropdown={} radio={} other={} options={} corner={} scroll={:.0} radius={:.0} viewport={:.0}",
            self.value.get(), self.minimum.get(), self.maximum.get(), self.step.get(),
            self.slider_events.get(), self.selected.get(), self.radio.get(),
            self.other_radio.get(), self.options.get().len(), self.corner_clicks.get(),
            self.scroll_y.get(), self.radius.get(), self.viewport_height.get(),
        ));
    }
}
