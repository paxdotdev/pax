use pax_engine::api::*;
use pax_engine::*;
use pax_std::components::Stacker;
use pax_std::components::*;
use pax_std::primitives::*;
use pax_std::types::text::*;
use pax_std::types::*;

#[pax]
#[inlined(
    @settings {
        @mount: on_mount
        @pre_render: tick
    }
)]
pub struct ContextualParent {
    pub on_children_change: Property<bool>,
    pub on_prop_change: Property<bool>,
}

impl ContextualParent {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let slot_children = ctx.get_slot_children();
        let deps = [slot_children.untyped()];
        let on_prop_change = self.on_prop_change.clone();
        // To know exactly what has changed, we need property vec? atm check diff between last value?
        slot_children.subscribe(
            move || {
                let slot_children = slot_children.get();
                let mut text_props = vec![];
                for child in slot_children {
                    child.with_properties(|child: &mut ContextualChild| {
                        log::debug!("child: {}", child.text.get());
                        text_props.push(child.text.untyped());
                    });
                }
                text_props.subscribe(move || {
                        log::debug!("a text prop changed!!");
                })
            },
            &deps,
        ));
    }

    pub fn tick(&mut self, ctx: &NodeContext) {
        // dirty trigger
        self.on_children_change.get();
        self.on_prop_change.get();
    }
}

#[pax]
#[inlined(
    @settings {}
)]
pub struct ContextualChild {
    pub text: Property<String>,
    pub on_change: Property<bool>,
}

impl ContextualChild {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let text = self.text.clone();
        text.subscribe(|| {
            ctx.get_env().push(......)
        });
    }

    pub fn pre_render(&mut self, ctx: &NodeContext) {
        // dirty patching
        self.on_change.get();
    }
}
