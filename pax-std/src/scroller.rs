use crate::primitives::*;
use crate::types::{StackerCell, StackerDirection};
use pax_engine::api::Numeric;
use pax_engine::api::{Event, Wheel};
use pax_engine::api::{Property, Size, Transform2D};
use pax_engine::*;
use pax_runtime::api::{NodeContext, StringBox};
/// Stacker lays out a series of nodes either
/// vertically or horizontally (i.e. a single row or column) with a specified gutter in between
/// each node.  `Stacker`s can be stacked inside of each other, horizontally
/// and vertically, along with percentage-based positioning and `Transform2D.anchor` to compose any rectilinear 2D layout.
#[pax]
#[inlined(
    <Frame>
        <Group x={(-self.scroll_x)px} y={(-self.scroll_y)px}>
            slot(0)
        </Group>
        <Rectangle fill=TRANSPARENT/>
    </Frame>

    @settings {
        @mount: on_mount
        @wheel: handle_wheel
        @pre_render: update,
    }

)]
pub struct Scroller {
    pub scroll_x: Property<f64>,
    pub scroll_y: Property<f64>,
    pub target_x: Property<f64>,
    pub target_y: Property<f64>,
}

// - how to not go to next page in Y direction? (args.prevent_default()?)

impl Scroller {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {}

    pub fn update(&mut self, ctx: &NodeContext) {
        // const SOFT_COEF: f64 = 0.30;
        // let soft_return_x =
        //     self.scroll_x.get() * SOFT_COEF + self.target_x.get() * (1.0 - SOFT_COEF);
        // let soft_return_y =
        //     self.scroll_y.get() * SOFT_COEF + self.target_y.get() * (1.0 - SOFT_COEF);
        // self.scroll_x.set(soft_return_x);
        // self.scroll_y.set(soft_return_y);
    }

    pub fn handle_wheel(&mut self, ctx: &NodeContext, args: Event<Wheel>) {
        let (bounds_x, bounds_y) = ctx.bounds_self.get();
        // - how to get the size of the content in the slot? (might need to be a primitive?)
        //   or, pass in manually
        let (max_bounds_x, max_bounds_y) = (bounds_x, bounds_y * 2.0); //temp
        let old_x = self.scroll_x.get();
        let old_y = self.scroll_y.get();
        let target_x = old_x + args.delta_x;
        let target_y = old_y + args.delta_y;
        let clamped_target_x = target_x.clamp(0.0, max_bounds_x - bounds_x);
        let clamped_target_y = target_y.clamp(0.0, max_bounds_y - bounds_y);
        self.scroll_x.set(clamped_target_x);
        self.scroll_y.set(clamped_target_y);
        // self.scroll_x.set(target_x);
        // self.scroll_y.set(target_y);
        // self.target_x.set(clamped_target_x);
        // self.target_y.set(clamped_target_y);
    }
}
