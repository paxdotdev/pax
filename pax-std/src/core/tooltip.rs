#[allow(unused)]
use crate::*;
use pax_engine::api::{Event, MouseOut};
use pax_engine::api::{MouseOver, Property};
use pax_engine::*;
use pax_runtime::api::NodeContext;

/// A scrolling container for arbitrary content.
#[pax]
#[engine_import_path("pax_engine")]
#[inlined(
    <Group >
        <Rectangle fill=TRANSPARENT/>
        for i in 0..self._slot_children_count {
            slot(0)
        }
    </Group>
    if self._showing {
        <Group x={100% + 5px} y={100% + 5px} anchor_x=0% anchor_y=0% width=200px height=30px>
            <Text x=5px height=100% width={100% -10px} id=text text={self.tip}/>
            <Rectangle corner_radii={RectangleCornerRadii::radii(5.00, 5.00, 5.00, 5.00)} fill=rgb(12.5%, 12.5%, 12.5%)/>
        </Group>
    }
    @settings {
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
                    align_vertical: TextAlignVertical::Center,
                    align_horizontal: TextAlignHorizontal::Left,
                    align_multiline: TextAlignHorizontal::Center
            }
    	}
    }

)]
pub struct Tooltip {
    pub tip: Property<String>,
    pub _showing: Property<bool>,
    pub _slot_children_count: Property<usize>,
}

impl Tooltip {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let slot_children_count = ctx.slot_children_count.clone();
        let deps = [slot_children_count.untyped()];
        self._slot_children_count
            .replace_with(Property::computed(move || slot_children_count.get(), &deps));
    }

    pub fn mouse_over(&mut self, _ctx: &NodeContext, _event: Event<MouseOver>) {
        self._showing.set(true);
    }

    pub fn mouse_out(&mut self, _ctx: &NodeContext, _event: Event<MouseOut>) {
        self._showing.set(false);
    }
}
