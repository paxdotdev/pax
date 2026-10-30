use pax_kit::*;

#[pax]
#[file("scene_page.pax")]
pub struct ScenePage {
    pub title: Property<String>,
    pub number: Property<String>,
    pub caption: Property<String>,
    pub tone: Property<Color>,
    pub note: Property<String>,
    pub clicks: Property<u64>,
    pub tilt: Property<f64>,
    pub narrow: Property<bool>,
    pub show_solid: Property<bool>,
}

impl ScenePage {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.note.set("Type here, then turn the page.".into());
        self.tilt.set(-28.0);
        let bounds = ctx.bounds_self.clone();
        let deps = [bounds.untyped()];
        self.narrow.replace_with(LocalProperty::computed(
            move || bounds.get().0 < 740.0,
            &deps,
        ));
        let bounds = ctx.bounds_self.clone();
        self.show_solid.replace_with(LocalProperty::computed(
            move || {
                let (width, height) = bounds.get();
                width >= 740.0 || height >= 680.0
            },
            &deps,
        ));
    }
    pub fn press(&mut self, _ctx: &NodeContext, _: Event<ButtonClick>) {
        self.clicks.set(self.clicks.get() + 1);
    }
}
