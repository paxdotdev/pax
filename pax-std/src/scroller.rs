use std::cell::RefCell;
use std::collections::HashMap;

use crate::primitives::*;
use crate::types::{StackerCell, StackerDirection};
use pax_engine::api::Numeric;
use pax_engine::api::{Event, Wheel};
use pax_engine::api::{Property, Size, Transform2D};
use pax_engine::*;
use pax_runtime::api::{NodeContext, StringBox, TouchEnd, TouchMove, TouchStart};
/// Stacker lays out a series of nodes either
/// vertically or horizontally (i.e. a single row or column) with a specified gutter in between
/// each node.  `Stacker`s can be stacked inside of each other, horizontally
/// and vertically, along with percentage-based positioning and `Transform2D.anchor` to compose any rectilinear 2D layout.
#[pax]
#[inlined(
    <Frame>
        <Group x={(-self.scroll_pos_x)px} y={(-self.scroll_pos_y)px}>
            slot(0)
        </Group>
        <Rectangle fill=TRANSPARENT/>
    </Frame>

    @settings {
        @mount: on_mount
        @wheel: handle_wheel
        @pre_render: update,
        @touch_move: touch_move,
        @touch_start: touch_start,
        @touch_end: touch_end,
    }

)]
pub struct Scroller {
    pub scroll_pos_x: Property<f64>,
    pub scroll_pos_y: Property<f64>,
    pub momentum_x: Property<f64>,
    pub momentum_y: Property<f64>,
    pub damping: Property<f64>,
    pub cached_damping: Property<f64>,
}

pub struct TouchInfo {
    x: f64,
    y: f64,
}

thread_local! {
    static TOUCH_TRACKER: RefCell<HashMap<i64, TouchInfo>> = RefCell::new(HashMap::new());
}

// - how to not go to next page in Y direction? (args.prevent_default()?)

impl Scroller {
    pub fn on_mount(&mut self, _ctx: &NodeContext) {
        self.damping.set(0.90);
    }

    pub fn update(&mut self, ctx: &NodeContext) {
        let mom_x = self.momentum_x.get();
        let mom_y = self.momentum_y.get();
        if TOUCH_TRACKER.with_borrow(|touches| touches.len() == 0) {
            let (bounds_x, bounds_y) = ctx.bounds_self.get();
            let (max_bounds_x, max_bounds_y) = (bounds_x, bounds_y * 2.0); //temp
            let old_x = self.scroll_pos_x.get();
            let old_y = self.scroll_pos_y.get();
            let target_x = old_x + mom_x;
            let target_y = old_y + mom_y;
            let clamped_target_x = target_x.clamp(0.0, max_bounds_x - bounds_x);
            let clamped_target_y = target_y.clamp(0.0, max_bounds_y - bounds_y);
            self.scroll_pos_x.set(clamped_target_x);
            self.scroll_pos_y.set(clamped_target_y);
        }
        self.momentum_x.set(mom_x * self.damping.get());
        self.momentum_y.set(mom_y * self.damping.get());
    }

    pub fn handle_wheel(&mut self, ctx: &NodeContext, args: Event<Wheel>) {
        let (bounds_x, bounds_y) = ctx.bounds_self.get();
        // - how to get the size of the content in the slot? (might need to be a primitive?)
        //   or, pass in manually
        let (max_bounds_x, max_bounds_y) = (bounds_x, bounds_y * 2.0); //temp
        let old_x = self.scroll_pos_x.get();
        let old_y = self.scroll_pos_y.get();
        let target_x = old_x + args.delta_x;
        let target_y = old_y + args.delta_y;
        let clamped_target_x = target_x.clamp(0.0, max_bounds_x - bounds_x);
        let clamped_target_y = target_y.clamp(0.0, max_bounds_y - bounds_y);
        self.scroll_pos_x.set(clamped_target_x);
        self.scroll_pos_y.set(clamped_target_y);
    }

    pub fn touch_move(&mut self, ctx: &NodeContext, args: Event<TouchMove>) {
        let args = args.touches.first().unwrap();
        TOUCH_TRACKER.with_borrow_mut(|touches| {
            let last = touches
                .get_mut(&args.identifier)
                .expect("should have recieved touch down before touch move");
            let delta_x = last.x - args.x;
            let delta_y = last.y - args.y;
            last.x = args.x;
            last.y = args.y;
            let (bounds_x, bounds_y) = ctx.bounds_self.get();
            let (max_bounds_x, max_bounds_y) = (bounds_x, bounds_y * 2.0); //temp
            let old_x = self.scroll_pos_x.get();
            let old_y = self.scroll_pos_y.get();
            let target_x = old_x + delta_x;
            let target_y = old_y + delta_y;
            let clamped_target_x = target_x.clamp(0.0, max_bounds_x - bounds_x);
            let clamped_target_y = target_y.clamp(0.0, max_bounds_y - bounds_y);
            self.scroll_pos_x.set(clamped_target_x);
            self.scroll_pos_y.set(clamped_target_y);
            let mom_x = self.momentum_x.get();
            let mom_y = self.momentum_y.get();
            self.momentum_x.set(mom_x + delta_x);
            self.momentum_y.set(mom_y + delta_y);
        });
    }

    pub fn touch_start(&mut self, _ctx: &NodeContext, args: Event<TouchStart>) {
        if TOUCH_TRACKER.with_borrow(|touches| touches.len() == 0) {
            let cached_damping = self.damping.get();
            self.cached_damping.set(cached_damping);
            self.damping.set(0.5);
        }
        self.momentum_x.set(0.0);
        self.momentum_y.set(0.0);
        TOUCH_TRACKER.with_borrow_mut(|touches| {
            touches.extend(
                args.touches
                    .iter()
                    .map(|e| (e.identifier, TouchInfo { x: e.x, y: e.y })),
            );
        });
    }
    pub fn touch_end(&mut self, _ctx: &NodeContext, args: Event<TouchEnd>) {
        TOUCH_TRACKER.with_borrow_mut(|touches| {
            let idents: Vec<_> = args.touches.iter().map(|e| e.identifier).collect();
            touches.retain(|k, _| !idents.contains(k));
        });
        if TOUCH_TRACKER.with_borrow(|touches| touches.len() == 0) {
            let cached_damping = self.cached_damping.get();
            self.damping.set(cached_damping);
        }
    }
}
