#[allow(unused)]
use crate::*;
use pax_engine::api::{ClickOrTap, Event, MouseOut, MouseOver, Property};
use pax_engine::*;
use pax_runtime::api::NodeContext;
use pax_runtime::{
    bind_content_measurement_effect, resolve_axis_autosize, sync_content_autosize_with_axes,
};

/// A simple hover tooltip that renders slotted content plus a floating text tip.
#[pax]
#[engine_import_path("pax_engine")]
#[inlined(
    <Group>
        // Keep the trigger owned by Tooltip so hover/click reach the wrapper
        // even when the slotted content belongs to an outer component.
        <Rectangle width=100% height=100% fill=TRANSPARENT/>
        for i in 0..self._projected_children_count {
            slot(i)
        }
    </Group>
    if self._showing {
        <Frame x=0px y={100% + 8px} anchor_x=0% anchor_y=0% width=220px autosize_y=true border_radius=8.0 layout_role=LayoutRole::Breakout>
            <Text x=12px y=10px width={100% - 24px} id=text text={self.tip}/>
            <Rectangle
                width=100%
                height=100%
                fill=rgb(12.5%, 12.5%, 12.5%)
                corner_radii={RectangleCornerRadii::radii(8.00, 8.00, 8.00, 8.00)}/>
        </Frame>
    }
    @settings {
        @click_or_tap: self.toggle
        @mouse_over: self.mouse_over
        @mouse_out: self.mouse_out
        @mount: on_mount
        #text {
            selectable: false,
            style: {
                    font: {Font::Web(
                        "ff-real-headline-pro",
                        "https://use.typekit.net/ivu7epf.css",
                        FontStyle::Normal,
                        FontWeight::ExtraLight,
                    )},
                    font_size: 16px,
                    fill: WHITE,
                    align_vertical: TextAlignVertical::Top,
                    align_horizontal: TextAlignHorizontal::Left,
                    align_multiline: TextAlignHorizontal::Left
            }
        }
    }

)]
pub struct Tooltip {
    /// Tooltip text.
    pub tip: Property<String>,
    /// Automatically sizes the trigger wrapper to its slotted content when possible.
    pub autosize: Property<bool>,
    /// Optional override for whether autosize manages the `x` axis.
    pub autosize_x: Property<Option<bool>>,
    /// Optional override for whether autosize manages the `y` axis.
    pub autosize_y: Property<Option<bool>>,
    // Private visibility flag driven by hover.
    pub _showing: Property<bool>,
    // Private slot count mirrored from `NodeContext`.
    pub _projected_children_count: Property<usize>,
}

impl Tooltip {
    // Mirrors slot count and reactive autosize behavior for the inline template.
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let projected_children_count = ctx.projected_children_count.clone();
        let deps = [projected_children_count.untyped()];
        self._projected_children_count
            .replace_with(Property::computed(
                move || projected_children_count.get(),
                &deps,
            ));
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
            "tooltip autosize",
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

    // Toggles the tooltip on click/tap for touch-first environments.
    pub fn toggle(&mut self, _ctx: &NodeContext, _event: Event<ClickOrTap>) {
        self._showing.set(!self._showing.get());
    }

    // Shows the tooltip on hover.
    pub fn mouse_over(&mut self, _ctx: &NodeContext, _event: Event<MouseOver>) {
        self._showing.set(true);
    }

    // Hides the tooltip when hover exits.
    pub fn mouse_out(&mut self, _ctx: &NodeContext, _event: Event<MouseOut>) {
        self._showing.set(false);
    }
}
