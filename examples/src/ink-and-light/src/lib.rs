#![allow(unused_imports)]

use pax_kit::math::Point2;
use pax_kit::*;

pub mod palette_status;
pub use palette_status::PaletteStatus;

const COMPACT_BREAKPOINT: f64 = 720.0;
const PRESSED_SCALE: f64 = 0.987;
const OVERSHOOT_SCALE: f64 = 1.033;

#[pax]
#[main]
#[file("lib.pax")]
pub struct Example {
    pub is_compact: Property<bool>,
    pub greeting_progress: Property<f64>,
    pub seal_progress: Property<f64>,
    pub postmark_reveal: Property<f64>,
    pub is_sealed: Property<bool>,
    pub seal_x: Property<f64>,
    pub seal_y: Property<f64>,
    pub card_scale: Property<f64>,
    pub active_touch_identifier: Property<i64>,
}

impl Example {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
        self.seal_x
            .set(if self.is_compact.get() { 79.0 } else { 88.0 });
        self.seal_y
            .set(if self.is_compact.get() { 16.0 } else { 36.0 });
        self.card_scale.set(1.0);
        self.active_touch_identifier.set(-1);
        self.greeting_progress.ease_to(
            1.0,
            Duration::Milliseconds(1600.into()),
            EasingCurve::OutQuad,
        );
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
    }

    pub fn card_mouse_down(&mut self, ctx: &NodeContext, event: Event<MouseDown>) {
        self.begin_stamp(ctx, event.mouse.x, event.mouse.y);
    }

    pub fn card_mouse_up(&mut self, _ctx: &NodeContext, _event: Event<MouseUp>) {
        self.release_card();
    }

    pub fn card_touch_start(&mut self, ctx: &NodeContext, event: Event<TouchStart>) {
        let Some(touch) = event.touches.first() else {
            return;
        };
        self.active_touch_identifier.set(touch.identifier);
        self.begin_stamp(ctx, touch.x, touch.y);
    }

    pub fn card_touch_end(&mut self, _ctx: &NodeContext, event: Event<TouchEnd>) {
        let identifier = self.active_touch_identifier.get();
        if event
            .touches
            .iter()
            .any(|touch| touch.identifier == identifier)
        {
            self.active_touch_identifier.set(-1);
            self.release_card();
        }
    }

    pub fn card_touch_cancel(&mut self, _ctx: &NodeContext, event: Event<TouchCancel>) {
        let identifier = self.active_touch_identifier.get();
        if event
            .touches
            .iter()
            .any(|touch| touch.identifier == identifier)
        {
            self.active_touch_identifier.set(-1);
            self.release_card();
        }
    }

    fn begin_stamp(&mut self, ctx: &NodeContext, window_x: f64, window_y: f64) {
        let local = ctx.local_point(Point2::new(window_x, window_y));
        self.seal_x.ease_to(
            local.x.clamp(0.11, 0.89) * 100.0,
            Duration::Milliseconds(360.into()),
            EasingCurve::OutBack,
        );
        self.seal_y.ease_to(
            local.y.clamp(0.12, 0.78) * 100.0,
            Duration::Milliseconds(360.into()),
            EasingCurve::OutBack,
        );

        self.card_scale.cancel_transitions();
        self.card_scale.ease_to(
            PRESSED_SCALE,
            Duration::Milliseconds(95.into()),
            EasingCurve::OutQuad,
        );

        self.seal_progress.cancel_transitions();
        self.postmark_reveal.cancel_transitions();
        self.is_sealed.set(true);
        self.seal_progress.set(0.0);
        self.postmark_reveal.set(0.0);
        self.seal_progress.ease_to(
            1.0,
            Duration::Milliseconds(520.into()),
            EasingCurve::OutBack,
        );
        self.postmark_reveal.ease_to(
            1.0,
            Duration::Milliseconds(460.into()),
            EasingCurve::OutQuad,
        );
    }

    fn release_card(&mut self) {
        self.card_scale.cancel_transitions();
        self.card_scale.ease_to(
            OVERSHOOT_SCALE,
            Duration::Milliseconds(170.into()),
            EasingCurve::OutQuad,
        );
        self.card_scale.ease_to_later(
            1.0,
            Duration::Milliseconds(260.into()),
            EasingCurve::OutBack,
        );
    }

    fn sync_layout(&mut self, ctx: &NodeContext) {
        let (width, _) = ctx.bounds_self.get();
        self.is_compact.set_if_neq(width < COMPACT_BREAKPOINT);
    }
}
