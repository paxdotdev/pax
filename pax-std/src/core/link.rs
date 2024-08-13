use pax_engine::api::Property;
use pax_engine::api::{Click, Event, NavigationTarget};
use pax_engine::*;
use pax_runtime::api::NodeContext;

#[pax]
#[inlined(
    for i in 0..self._slot_children {
        slot(i)
    }

    @settings {
        @mount: on_mount
        @click: on_click
    }

)]
pub struct Link {
    pub url: Property<String>,
    pub target: Property<Target>,
    pub _slot_children: Property<usize>,
}

#[pax]
pub enum Target {
    #[default]
    Current,
    New,
}

impl From<Target> for NavigationTarget {
    fn from(value: Target) -> Self {
        match value {
            Target::Current => NavigationTarget::Current,
            Target::New => NavigationTarget::New,
        }
    }
}

impl Link {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let s = ctx.slot_children_count.clone();
        let deps = [s.untyped()];
        self._slot_children
            .replace_with(Property::computed(move || s.get(), &deps));
    }
    pub fn on_click(&mut self, ctx: &NodeContext, _event: Event<Click>) {
        ctx.navigate_to(&self.url.get(), self.target.get().into());
    }
}
