#[allow(unused)]
use crate::*;
use pax_engine::api::Numeric;
use pax_engine::api::{Event, Wheel};
use pax_engine::api::{Property, Size};
use pax_engine::*;
use pax_runtime::api::{NodeContext, TouchEnd, TouchMove, TouchStart, OS};
use std::cell::RefCell;
use std::collections::HashMap;

/// A scrolling container for arbitrary content.
#[pax]
#[inlined(
    <Frame _clip_content={self._clip_content}>
        <Scrollbar
            scroll_y={self.scroll_pos_y}
            width=20px
            anchor_x=100%
            x=100%
            size_inner_pane_y={self.scroll_height}
        />
        <Scrollbar
            scroll_x={self.scroll_pos_x}
            height=20px
            anchor_y=100%
            y=100%
            size_inner_pane_x={self.scroll_width}
        />
        <Group x={(-self.scroll_pos_x)px} y={(-self.scroll_pos_y)px} width={self.scroll_width} height={self.scroll_height}>
            for i in 0..self._slot_children_count {
                slot(i)
            }
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
#[custom(Default)]
pub struct Scroller {
    pub scroll_pos_x: Property<Numeric>,
    pub scroll_pos_y: Property<Numeric>,
    pub scroll_width: Property<Size>,
    pub scroll_height: Property<Size>,

    // used by pax create (might want to just make public at some point)
    pub _clip_content: Property<bool>,

    // private fields
    pub _platform_params: Property<PlatformSpecificScrollParams>,
    pub _momentum_x: Property<f64>,
    pub _momentum_y: Property<f64>,
    pub _damping: Property<f64>,
    pub _slot_children_count: Property<usize>,
}

impl Default for Scroller {
    fn default() -> Self {
        Self {
            scroll_pos_x: Default::default(),
            scroll_pos_y: Default::default(),
            scroll_width: Default::default(),
            scroll_height: Default::default(),
            _clip_content: Property::new(true),
            _platform_params: Default::default(),
            _momentum_x: Default::default(),
            _momentum_y: Default::default(),
            _damping: Default::default(),
            _slot_children_count: Default::default(),
        }
    }
}

#[pax]
pub struct PlatformSpecificScrollParams {
    // constant decrease in momentum over time
    pub deacceleration: f64,
    // friction (decreases momentum proportionally to current value)
    pub damping: f64,
    // if user "flings", set to high momentum to quickly scroll
    pub fling: bool,
}

pub struct TouchInfo {
    x: f64,
    y: f64,
}

thread_local! {
    static TOUCH_TRACKER: RefCell<HashMap<i64, TouchInfo>> = RefCell::new(HashMap::new());
}
pub fn no_touches() -> bool {
    TOUCH_TRACKER.with_borrow(|touches| touches.len() == 0)
}

impl Scroller {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let slot_children_count = ctx.slot_children_count.clone();
        let deps = [slot_children_count.untyped()];
        self._slot_children_count
            .replace_with(Property::computed(move || slot_children_count.get(), &deps));
        let scroll_params = match ctx.os {
            OS::Android => PlatformSpecificScrollParams {
                deacceleration: 0.02,
                damping: 0.03,
                fling: true,
            },
            OS::IPhone => PlatformSpecificScrollParams {
                deacceleration: 0.00,
                damping: 0.04,
                fling: false,
            },
            // just choose some hopefully sane default
            OS::Windows | OS::Mac | OS::Linux | OS::Unknown => PlatformSpecificScrollParams {
                deacceleration: 0.01,
                damping: 0.04,
                fling: false,
            },
        };
        self._platform_params.set(scroll_params);
    }

    pub fn update(&mut self, ctx: &NodeContext) {
        let mom_x = self._momentum_x.get();
        let mom_y = self._momentum_y.get();
        if no_touches() {
            self.add_position(ctx, mom_x, mom_y);
        }
        let platform_data = self._platform_params.get();
        let damping = self._damping.get();

        let mut new_mom_x;
        let mut new_mom_y;

        // damping
        let falloff_factor = 1.0 - damping;
        new_mom_x = mom_x * falloff_factor;
        new_mom_y = mom_y * falloff_factor;

        // decelleration
        if new_mom_x > 0.0 {
            new_mom_x = (new_mom_x - platform_data.deacceleration).max(0.0);
        }
        if new_mom_x < 0.0 {
            new_mom_x = (new_mom_x + platform_data.deacceleration).min(0.0);
        }
        if new_mom_y > 0.0 {
            new_mom_y = (new_mom_y - platform_data.deacceleration).max(0.0);
        }
        if new_mom_y < 0.0 {
            new_mom_y = (new_mom_y + platform_data.deacceleration).min(0.0);
        }

        // stop if close to 0
        if new_mom_x.abs() < 0.1 {
            new_mom_x = 0.0;
        }
        if new_mom_y.abs() < 0.1 {
            new_mom_y = 0.0;
        }

        self._momentum_x.set(new_mom_x);
        self._momentum_y.set(new_mom_y);
    }

    pub fn add_position(&self, ctx: &NodeContext, dx: f64, dy: f64) {
        let (bounds_x, bounds_y) = ctx.bounds_self.get();
        let (max_bounds_x, max_bounds_y) = (
            self.scroll_width.get().get_pixels(bounds_x),
            self.scroll_height.get().get_pixels(bounds_y),
        );
        let old_x = self.scroll_pos_x.get().to_float();
        let old_y = self.scroll_pos_y.get().to_float();
        let target_x = old_x + dx;
        let target_y = old_y + dy;
        let clamped_target_x = target_x.clamp(0.0, (max_bounds_x - bounds_x).max(0.0));
        let clamped_target_y = target_y.clamp(0.0, (max_bounds_y - bounds_y).max(0.0));
        self.scroll_pos_x.set(Numeric::F64(clamped_target_x));
        self.scroll_pos_y.set(Numeric::F64(clamped_target_y));
    }

    pub fn add_momentum(&self, ddx: f64, ddy: f64) {
        let mom_x = self._momentum_x.get();
        let mom_y = self._momentum_y.get();
        self._momentum_x.set(mom_x + ddx);
        self._momentum_y.set(mom_y + ddy);
    }

    pub fn process_new_touch_pos(&self, ctx: &NodeContext, x: f64, y: f64, ident: i64) {
        TOUCH_TRACKER.with_borrow_mut(|touches| {
            let last = touches
                .get_mut(&ident)
                .expect("should have recieved touch down before touch move");
            let delta_x = last.x - x;
            let delta_y = last.y - y;
            last.x = x;
            last.y = y;
            self.add_position(ctx, delta_x, delta_y);
            self.add_momentum(delta_x, delta_y);
        });
    }

    pub fn handle_wheel(&mut self, ctx: &NodeContext, args: Event<Wheel>) {
        self.add_position(ctx, args.delta_x, args.delta_y);
    }

    pub fn touch_move(&mut self, ctx: &NodeContext, args: Event<TouchMove>) {
        for touch in &args.touches {
            self.process_new_touch_pos(ctx, touch.x, touch.y, touch.identifier);
        }
    }

    pub fn touch_start(&mut self, _ctx: &NodeContext, args: Event<TouchStart>) {
        if no_touches() {
            // this is first touch
            let cached_damping = self._platform_params.get().damping;
            let temp_damping = cached_damping.max(0.5);
            self._damping.set(temp_damping);
        }
        self._momentum_x.set(0.0);
        self._momentum_y.set(0.0);
        TOUCH_TRACKER.with_borrow_mut(|touches| {
            touches.extend(
                args.touches
                    .iter()
                    .map(|e| (e.identifier, TouchInfo { x: e.x, y: e.y })),
            );
        });
    }

    pub fn touch_end(&mut self, ctx: &NodeContext, args: Event<TouchEnd>) {
        for touch in &args.touches {
            self.process_new_touch_pos(ctx, touch.x, touch.y, touch.identifier);
        }
        TOUCH_TRACKER.with_borrow_mut(|touches| {
            let idents: Vec<_> = args.touches.iter().map(|e| e.identifier).collect();
            touches.retain(|k, _| !idents.contains(k));
        });
        if no_touches() {
            let params = self._platform_params.get();
            self._damping.set(params.damping);

            let mut mom_x = self._momentum_x.get();
            let mut mom_y = self._momentum_y.get();
            if params.fling && mom_x.abs() > 50.0 {
                mom_x *= 1.7;
            }
            if params.fling && mom_y.abs() > 50.0 {
                mom_y *= 1.7;
            }
            self._momentum_x.set(mom_x);
            self._momentum_y.set(mom_y);
        }
    }
}
