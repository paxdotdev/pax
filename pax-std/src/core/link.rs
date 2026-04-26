#![allow(unused)]
use crate::Rectangle;
use pax_engine::api::cursor::CursorStyle;
use pax_engine::api::{ClickOrTap, MouseOut, MouseOver, Property};
use pax_engine::api::{Event, NavigationTarget};
use pax_engine::*;
use pax_runtime::api::NodeContext;
use pax_runtime::{
    bind_content_measurement_effect, resolve_axis_autosize, sync_content_autosize_with_axes,
};

#[pax]
#[engine_import_path("pax_engine")]
#[inlined(
    <Rectangle id=_designer_raycast_ignore fill=TRANSPARENT/>
    for i in 0..self._projected_children {
        slot(i)
    }

    @settings {
        @mouse_over: self.mouse_over
        @mouse_out: self.mouse_out
        @mount: on_mount
        @click_or_tap: on_click_or_tap
    }

)]
/// Navigates to a URL when its slotted content is clicked or tapped.
pub struct Link {
    /// Destination URL.
    pub url: Property<String>,
    /// Whether to open the URL in the current or a new browsing context.
    pub target: Property<Target>,
    /// Automatically sizes the link wrapper to its slotted content when possible.
    pub autosize: Property<bool>,
    /// Optional override for whether autosize manages the `x` axis.
    pub autosize_x: Property<Option<bool>>,
    /// Optional override for whether autosize manages the `y` axis.
    pub autosize_y: Property<Option<bool>>,
    // Number of slotted children to render.
    pub _projected_children: Property<usize>,
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
    // Binds slot count and reactive autosize behavior for the generated inline template.
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let s = ctx.projected_children_count.clone();
        let deps = [s.untyped()];
        self._projected_children
            .replace_with(Property::computed(move || s.get(), &deps));
        let Some(expanded_node) = ctx.expanded_node.upgrade() else {
            return;
        };
        let autosize = self.autosize.clone();
        let autosize_x = self.autosize_x.clone();
        let autosize_y = self.autosize_y.clone();
        let deps = [
            autosize.untyped(),
            autosize_x.untyped(),
            autosize_y.untyped(),
        ];
        bind_content_measurement_effect(
            &expanded_node,
            ctx,
            "link autosize",
            &deps,
            move |node, node_ctx| {
                sync_content_autosize_with_axes(
                    node,
                    node_ctx,
                    resolve_axis_autosize(autosize.get(), autosize_x.get(), true),
                    resolve_axis_autosize(autosize.get(), autosize_y.get(), true),
                );
            },
        );
    }

    // Dispatches navigation through the active runtime context.
    pub fn on_click_or_tap(&mut self, ctx: &NodeContext, _event: Event<ClickOrTap>) {
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
