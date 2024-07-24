use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::*;
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
        @pre_render: pre_render
    }
)]
pub struct ContextualParent {
    pub on_data_change: Property<bool>,
}

impl ContextualParent {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let data: Property<Vec<String>> = Property::default();
        ctx.push_local_store(ContextStore { data: data.clone() });
        let deps = [data.untyped()];
        self.on_data_change.replace_with(Property::computed(
            move || {
                let data = data.get();
                log::info!("data changed: {:?}", data);
                false
            },
            &deps,
        ));
    }

    pub fn pre_render(&mut self, ctx: &NodeContext) {
        // dirty trigger
        self.on_data_change.get();
    }
}

#[pax]
#[inlined(
    @settings {
        @mount: on_mount
        @pre_render: pre_render
    }
)]
pub struct ContextualChild {
    pub text: Property<String>,
    pub id: Property<usize>,

    // private
    pub on_change: Property<bool>,
}

impl ContextualChild {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let parent_data = ctx
            .peek_local_store(|store: &mut ContextStore| store.data.clone())
            .expect("child contextual element should exist under a parent");
        let text = self.text.clone();
        let id = self.id.clone();
        let deps = [text.untyped(), id.untyped()];
        self.on_change.replace_with(Property::computed(
            move || {
                parent_data.update(|list| {
                    let id = id.get();
                    while list.len() < id + 1 {
                        list.push(String::new());
                    }
                    list[id] = text.get();
                });
                false
            },
            &deps,
        ));
    }

    pub fn pre_render(&mut self, ctx: &NodeContext) {
        // dirty patching
        self.on_change.get();
    }
}
