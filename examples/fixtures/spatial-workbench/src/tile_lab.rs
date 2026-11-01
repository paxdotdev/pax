use pax_kit::*;

#[pax]
#[file("tile_lab.pax")]
pub struct TileLab {
    pub camera_zoom: Property<f64>,
    pub scroll_x: Property<f64>,
    pub scroll_y: Property<f64>,
    pub inner_y: Property<f64>,
    pub distant: Property<bool>,
    pub near: Property<bool>,
    pub note: Property<String>,
}

impl TileLab {
    pub fn mount(&mut self, _ctx: &NodeContext) {
        self.note.set("Edit me, then jump away and back.".into());
    }

    pub fn jump(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        let distant = !self.distant.get();
        self.distant.set(distant);
        self.scroll_x.set(if distant { 3072.0 } else { 0.0 });
        self.scroll_y.set(if distant { 3072.0 } else { 0.0 });
    }

    pub fn zoom(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        let target = if self.camera_zoom.get() < 1.0 {
            1.0
        } else {
            0.65
        };
        self.camera_zoom.local().ease_to(
            target,
            Duration::Milliseconds(450.into()),
            EasingCurve::InOutQuad,
        );
    }

    pub fn cross_near(&mut self, _ctx: &NodeContext, _event: Event<ButtonClick>) {
        self.near.set(!self.near.get());
    }
}
