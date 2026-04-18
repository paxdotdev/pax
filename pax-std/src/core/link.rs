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
/// Navigates to a URL when its slotted content is clapped/clicked.
pub struct Link {
    /// Destination URL.
    pub url: Property<String>,
    /// Whether to open the URL in the current or a new browsing context.
    pub target: Property<Target>,
    // Number of slotted children to render.
    pub _slot_children: Property<usize>,
}

/// Navigation target for `Link`.
#[pax]
#[engine_import_path("pax_engine")]
pub enum Target {
    /// Navigate in the current window or tab.
    #[default]
    Current,
    /// Navigate in a new window or tab.
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
    // Binds slot count for the generated inline template.
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let s = ctx.slot_children_count.clone();
        let deps = [s.untyped()];
        self._slot_children
            .replace_with(Property::computed(move || s.get(), &deps));
    }
    // Dispatches navigation through the active runtime context.
    pub fn on_clap(&mut self, ctx: &NodeContext, _event: Event<Clap>) {
        ctx.navigate_to(&self.url.get(), self.target.get().into());
    }

    // Uses a pointer cursor while hovering the link.
    pub fn mouse_over(&mut self, ctx: &NodeContext, _event: Event<MouseOver>) {
        ctx.set_cursor(CursorStyle::Pointer);
    }

    // Restores the default cursor after hover.
    pub fn mouse_out(&mut self, ctx: &NodeContext, _event: Event<MouseOut>) {
        ctx.set_cursor(CursorStyle::Auto);
    }
}
