use pax_kit::*;

mod scene_page;
pub use scene_page::*;

pax_kit::runnable! { root: crate::Example }

#[pax]
#[file("lib.pax")]
pub struct Example {
    pub direction: Property<RouteCuboidDirection>,
    pub direction_label: Property<String>,
    pub busy_until: Property<u64>,
    pub narrow: Property<bool>,
    pub background_depth: Property<f64>,
}

impl Example {
    pub fn mount(&mut self, ctx: &NodeContext) {
        self.direction_label.set("LEFT".into());
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

    fn choose(&mut self, ctx: &NodeContext, direction: RouteCuboidDirection, label: &str) {
        if ctx.elapsed_millis.get() < self.busy_until.get() {
            return;
        }
        self.direction.set(direction);
        self.direction_label.set(label.into());
    }

    pub fn left(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        self.choose(ctx, RouteCuboidDirection::Left, "LEFT");
    }
    pub fn right(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        self.choose(ctx, RouteCuboidDirection::Right, "RIGHT");
    }
    pub fn up(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        self.choose(ctx, RouteCuboidDirection::Up, "UP");
    }
    pub fn down(&mut self, ctx: &NodeContext, _: Event<ButtonClick>) {
        self.choose(ctx, RouteCuboidDirection::Down, "DOWN");
    }
    fn navigate(&mut self, ctx: &NodeContext, destination: &str) {
        let now = ctx.elapsed_millis.get();
        if now < self.busy_until.get() {
            return;
        }
        self.busy_until.set(now + 1200);
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
