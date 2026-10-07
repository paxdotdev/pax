use pax_kit::*;

#[pax]
#[file("catalog.pax")]
pub struct Catalog {
    pub units: Property<f64>,
    pub branch: Property<usize>,
}

#[pax]
#[main]
#[file("lib.pax")]
pub struct SurfaceScheduling {
    pub mode: Property<usize>,
    pub pane_width: Property<f64>,
    pub pane_height: Property<f64>,
    pub branches: Property<usize>,
    pub units: Property<f64>,
}

impl SurfaceScheduling {
    pub fn layout(&mut self, ctx: &NodeContext) {
        let (width, height) = ctx.bounds_self.get();
        let branches = if self.mode.get() >= 2 { 2 } else { 1 };
        self.branches.set_if_neq(branches);
        self.units
            .set_if_neq(if self.mode.get() % 2 == 1 { 2.0 } else { 1.0 });
        self.pane_width
            .set_if_neq(((width - 36.0) / branches as f64 - 12.0).max(1.0));
        self.pane_height.set_if_neq((height - 120.0).max(1.0));
    }

    pub fn single(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.mode.set(0);
    }
    pub fn scaled(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.mode.set(1);
    }
    pub fn pair(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.mode.set(2);
    }
    pub fn pair_scaled(&mut self, _: &NodeContext, _: Event<ButtonClick>) {
        self.mode.set(3);
    }
}
