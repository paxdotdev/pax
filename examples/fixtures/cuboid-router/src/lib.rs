use pax_kit::*;

mod scene_page;
pub use scene_page::*;

pax_kit::runnable! { root: crate::Example }

#[pax]
#[file("lib.pax")]
pub struct Example {
    pub direction: Property<RouteCuboidDirection>,
    pub direction_label: Property<String>,
    pub duration_ms: Property<u64>,
    pub narrow: Property<bool>,
    pub background_depth: Property<f64>,
}

impl Example {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.direction_label.set("LEFT".into());
        self.duration_ms.set(1200);
        let bounds = ctx.bounds_self.clone();
        let deps = [bounds.untyped()];
        self.narrow.replace_with(LocalProperty::computed(
            move || bounds.get().0 < 740.0,
            &deps,
        ));
        let bounds = ctx.bounds_self.clone();
        self.background_depth.replace_with(LocalProperty::computed(
            move || {
                let (width, height) = bounds.get();
                -(width + height)
            },
            &deps,
        ));
    }

    fn choose(&mut self, direction: RouteCuboidDirection, label: &str) {
        self.direction.set(direction);
        self.direction_label.set(label.into());
    }

    pub fn left(&mut self, _ctx: &NodeContext, _: Event<ButtonClick>) {
        self.choose(RouteCuboidDirection::Left, "LEFT");
    }
    pub fn right(&mut self, _ctx: &NodeContext, _: Event<ButtonClick>) {
        self.choose(RouteCuboidDirection::Right, "RIGHT");
    }
    pub fn up(&mut self, _ctx: &NodeContext, _: Event<ButtonClick>) {
        self.choose(RouteCuboidDirection::Up, "UP");
    }
    pub fn down(&mut self, _ctx: &NodeContext, _: Event<ButtonClick>) {
        self.choose(RouteCuboidDirection::Down, "DOWN");
    }
    pub fn speed(&mut self, _ctx: &NodeContext, _: Event<ButtonClick>) {
        self.duration_ms.set(match self.duration_ms.get() {
            600 => 1200,
            1200 => 2400,
            _ => 600,
        });
    }
    fn navigate(&mut self, ctx: &NodeContext, destination: &str) {
        ctx.navigate_to(destination, NavigationTarget::Current);
    }

    pub fn notes(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        self.navigate(ctx, "/notes");
    }
    pub fn studio(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        self.navigate(ctx, "/studio");
    }
    pub fn home(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        self.navigate(ctx, "/");
    }
}
