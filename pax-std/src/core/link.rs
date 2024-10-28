#![allow(unused)]
use crate::Rectangle;
use pax_engine::api::cursor::CursorStyle;
use pax_engine::api::{Clap, MouseOut, MouseOver, Property};
use pax_engine::api::{Event, NavigationTarget};
use pax_engine::*;
use pax_runtime::api::NodeContext;

#[pax]
#[engine_import_path("pax_engine")]
#[inlined(
    <Rectangle id=_designer_raycast_ignore fill=TRANSPARENT/>
    for i in 0..self._slot_children {
        slot(i)
    }

    @settings {
        @mouse_over: self.mouse_over
        @mouse_out: self.mouse_out
        @mount: on_mount
        @clap: on_clap
    }

)]
pub struct Link {
    pub url: Property<String>,
    pub target: Property<Target>,
    pub _slot_children: Property<usize>,
}

#[pax]
#[engine_import_path("pax_engine")]
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
    pub fn on_clap(&mut self, ctx: &NodeContext, _event: Event<Clap>) {
        ctx.navigate_to(&self.url.get(), self.target.get().into());
    }

    pub fn mouse_over(&mut self, ctx: &NodeContext, _event: Event<MouseOver>) {
        ctx.set_cursor(CursorStyle::Pointer);
    }

    pub fn mouse_out(&mut self, ctx: &NodeContext, _event: Event<MouseOut>) {
        ctx.set_cursor(CursorStyle::Auto);
    }
}
