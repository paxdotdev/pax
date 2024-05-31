use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

pub struct ContextStore {
    data: Property<Vec<String>>,
}

impl Store for ContextStore {}

#[pax]
#[inlined(
    @settings {
        @mount: on_mount
        @pre_render: tick
    }
)]
pub struct ContextualParent {
    pub on_data_change: Property<bool>,
}

impl ContextualParent {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let data: Vec<String> = Property::default();
        ctx.push_local_store(ContextStore { data: data.clone() });
        let deps = [data.untyped()];
        self.on_data_change.replace_with(Property::computed(
            move || {
                let data = data.get();
                log::debug!("data changed: {}", data);
            },
            &deps,
        ));
    }

    pub fn tick(&mut self, ctx: &NodeContext) {
        // dirty trigger
        self.on_data_change.get();
    }
}

#[pax]
#[inlined(
    @settings {}
)]
pub struct ContextualChild {
    pub text: Property<String>,
    pub id: Property<usize>,

    // private
    pub on_change: Property<bool>,
}

impl ContextualChild {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let parent_data = ctx.peek_data_store(|store: &mut ContextStore| store.data.clone());
        let text = self.text.clone();
        let id = self.id.clone();
        let deps = [text.untyped(), id.untyped()];
        self.on_change.replace(Property::computed(
            // move || {
                parent_data.update(|list| {
                    list[id.get()] = text.get();
                });
            },
            deps,
        ));
    }

    pub fn pre_render(&mut self, ctx: &NodeContext) {
        // dirty patching
        self.on_change.get();
    }
}
