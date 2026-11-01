use pax_kit::*;

#[pax]
#[file("scroll_study.pax")]
pub struct ScrollStudy {
    pub scroll: Property<f64>,
    pub progress: Property<f64>,
    pub lower_progress: Property<f64>,
    pub note: Property<String>,
    pub presses: Property<u64>,
}
impl ScrollStudy {
    pub fn mount(&mut self, _: &NodeContext) {
        self.note.set("A little room for your imagination.".into());
        let scroll = self.scroll.local();
        let deps = [scroll.untyped()];
        self.progress.replace_with(LocalProperty::computed(
            move || (scroll.get() / 900.).clamp(0., 1.),
            &deps,
        ));
        let scroll = self.scroll.local();
        let deps = [scroll.untyped()];
        self.lower_progress.replace_with(LocalProperty::computed(
            move || ((scroll.get() - 600.) / 900.).clamp(0., 1.),
            &deps,
        ));
    }
    pub fn press(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.presses.set(self.presses.get() + 1);
    }
}
