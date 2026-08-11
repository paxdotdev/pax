#![allow(unused_imports)]

use pax_kit::*;

const COMPACT_BREAKPOINT_PX: f64 = 760.0;
const DESKTOP_CARD_WIDTH_PX: f64 = 350.0;
const COMPACT_CARD_WIDTH_PX: f64 = 286.0;
const CARD_GAP_PX: f64 = 14.0;
const MARQUEE_SPEED_PX_PER_SECOND: f64 = 24.0;
const CULL_OVERSCAN_CARDS: isize = 2;

/// A single gallery card. Its media area is projected content so a card
/// can contain Pax-native artwork today and richer image/video/example content
/// later without changing a content schema.
#[pax]
#[file("feature_gallery/card.pax")]
pub struct FeatureCard {
    pub category: Property<String>,
    pub title: Property<String>,
    pub summary: Property<String>,
    pub proof: Property<String>,
    pub docs_label: Property<String>,
    pub docs_url: Property<String>,
    pub marquee_hovered: Property<bool>,
    pub marquee_touch_active: Property<bool>,
    pub marquee_scroll_x: Property<f64>,
    pub _active_touch_identifier: Property<i64>,
}

impl FeatureCard {
    pub fn pause_marquee_for_mouse(&mut self, _ctx: &NodeContext, _event: Event<MouseOver>) {
        self.marquee_hovered.set(true);
    }

    pub fn resume_marquee_for_mouse(&mut self, _ctx: &NodeContext, _event: Event<MouseOut>) {
        self.marquee_hovered.set(false);
    }

    pub fn pause_marquee_for_touch(&mut self, _ctx: &NodeContext, event: Event<TouchStart>) {
        if let Some(touch) = event.touches.first() {
            self._active_touch_identifier.set(touch.identifier);
        }
        self.marquee_touch_active.set(true);
    }

    pub fn drag_marquee(&mut self, _ctx: &NodeContext, event: Event<TouchMove>) {
        let identifier = self._active_touch_identifier.get();
        if let Some(touch) = event
            .touches
            .iter()
            .find(|touch| touch.identifier == identifier)
        {
            self.marquee_scroll_x
                .set(self.marquee_scroll_x.get() - touch.delta_x);
            event.prevent_default();
        }
    }

    pub fn resume_marquee_for_touch(&mut self, _ctx: &NodeContext, _event: Event<TouchEnd>) {
        self.marquee_touch_active.set(false);
        self._active_touch_identifier.set(-1);
    }

    pub fn cancel_marquee_touch(&mut self, _ctx: &NodeContext, _event: Event<TouchCancel>) {
        self.marquee_touch_active.set(false);
        self._active_touch_identifier.set(-1);
    }
}

/// One structurally mounted card in the marquee's viewport plus overscan.
#[derive(PartialEq)]
#[pax]
#[custom(Defaults)]
pub struct MarqueeCell {
    pub slot_index: usize,
    pub x_px: f64,
}

/// A continuously scrolling, virtualized horizontal card rail.
///
/// Unlike `Carousel`, this component intentionally does not install mandatory
/// page snap points: continuous marquee motion and mandatory native snapping
/// compete for scroll authority. Direct manipulation stays free-scrolling,
/// while hover and touch pause the automatic lane.
#[pax]
#[file("feature_gallery/marquee.pax")]
#[custom(Default)]
pub struct MarqueeCarousel {
    pub card_width_px: Property<f64>,
    pub gap_px: Property<f64>,
    pub speed_px_per_second: Property<f64>,
    pub scroll_x: Property<f64>,

    pub _content_width_px: Property<f64>,
    pub _visible_cells: Property<Vec<MarqueeCell>>,
    pub hover_paused: Property<bool>,
    pub touch_paused: Property<bool>,
    pub _active_touch_identifier: Property<i64>,
    pub _last_frame_millis: Property<f64>,
}

impl Default for MarqueeCarousel {
    fn default() -> Self {
        Self {
            card_width_px: Property::new(DESKTOP_CARD_WIDTH_PX),
            gap_px: Property::new(CARD_GAP_PX),
            speed_px_per_second: Property::new(MARQUEE_SPEED_PX_PER_SECOND),
            scroll_x: Property::new(0.0),
            _content_width_px: Property::new(0.0),
            _visible_cells: Property::new(vec![]),
            hover_paused: Property::new(false),
            touch_paused: Property::new(false),
            _active_touch_identifier: Property::new(-1),
            _last_frame_millis: Property::new(0.0),
        }
    }
}

impl MarqueeCarousel {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self._last_frame_millis
            .set(ctx.elapsed_time_millis() as f64);
        self.sync_geometry(ctx);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        let previous = self._last_frame_millis.get();
        self._last_frame_millis.set(now);

        let delta_seconds = ((now - previous).max(0.0).min(50.0)) / 1000.0;
        let paused = self.hover_paused.get() || self.touch_paused.get();
        if !paused && delta_seconds > 0.0 {
            let period = self._content_width_px.get();
            if period > 0.0 {
                let next =
                    self.scroll_x.get() + self.speed_px_per_second.get().max(0.0) * delta_seconds;
                self.scroll_x.set(next.rem_euclid(period));
            }
        }

        self.sync_geometry(ctx);
    }

    pub fn pause_for_mouse(&mut self, _ctx: &NodeContext, _event: Event<MouseOver>) {
        self.hover_paused.set(true);
    }

    pub fn resume_for_mouse(&mut self, _ctx: &NodeContext, _event: Event<MouseOut>) {
        self.hover_paused.set(false);
    }

    pub fn pause_for_touch(&mut self, _ctx: &NodeContext, event: Event<TouchStart>) {
        if let Some(touch) = event.touches.first() {
            self._active_touch_identifier.set(touch.identifier);
        }
        self.touch_paused.set(true);
    }

    pub fn drag_touch(&mut self, ctx: &NodeContext, event: Event<TouchMove>) {
        let identifier = self._active_touch_identifier.get();
        if let Some(touch) = event
            .touches
            .iter()
            .find(|touch| touch.identifier == identifier)
        {
            self.scroll_by(ctx, -touch.delta_x);
            event.prevent_default();
        }
    }

    pub fn resume_for_touch(&mut self, _ctx: &NodeContext, _event: Event<TouchEnd>) {
        self.touch_paused.set(false);
        self._active_touch_identifier.set(-1);
    }

    pub fn cancel_touch_pause(&mut self, _ctx: &NodeContext, _event: Event<TouchCancel>) {
        self.touch_paused.set(false);
        self._active_touch_identifier.set(-1);
    }

    pub fn handle_scroll(&mut self, ctx: &NodeContext, event: Event<Scroll>) {
        let delta = if event.delta_x.abs() > event.delta_y.abs() {
            event.delta_x
        } else {
            event.delta_y
        };
        self.scroll_by(ctx, delta);
        event.prevent_default();
    }

    fn sync_geometry(&mut self, ctx: &NodeContext) {
        let count = ctx.projected_children_count.get();
        let card_width = self.card_width_px.get().max(1.0);
        let gap = self.gap_px.get().max(0.0);
        let stride = card_width + gap;
        // Include the trailing gap in the loop period. This makes the frames on
        // either side of wrap-around geometrically identical.
        let content_width = count as f64 * stride;
        self._content_width_px.set_if_neq(content_width);

        let viewport_width = ctx.bounds_self.get().0.max(0.0);
        let scroll_x = if content_width > 0.0 {
            self.scroll_x.get().rem_euclid(content_width)
        } else {
            0.0
        };
        self.scroll_x.set_if_neq(scroll_x);

        let cells = if count == 0 {
            vec![]
        } else {
            let first = (scroll_x / stride).floor() as isize - CULL_OVERSCAN_CARDS;
            let last = ((scroll_x + viewport_width) / stride).ceil() as isize + CULL_OVERSCAN_CARDS;
            (first..last)
                .map(|virtual_index| MarqueeCell {
                    slot_index: virtual_index.rem_euclid(count as isize) as usize,
                    x_px: virtual_index as f64 * stride,
                })
                .collect()
        };
        self._visible_cells.set_if_neq(cells);
    }

    fn scroll_by(&mut self, ctx: &NodeContext, delta: f64) {
        self.scroll_x.set(self.scroll_x.get() + delta);
        self.sync_geometry(ctx);
    }
}

/// The content-authored feature heap used on the Pax home page.
#[pax]
#[file("feature_gallery/gallery.pax")]
pub struct FeatureGallery {
    pub compact: Property<bool>,
    pub card_width_px: Property<f64>,
    pub marquee_hovered: Property<bool>,
    pub marquee_touch_active: Property<bool>,
    pub marquee_scroll_x: Property<f64>,
}

impl FeatureGallery {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
    }

    fn sync_layout(&mut self, ctx: &NodeContext) {
        let compact = ctx.bounds_self.get().0 < COMPACT_BREAKPOINT_PX;
        self.compact.set_if_neq(compact);
        self.card_width_px.set_if_neq(if compact {
            COMPACT_CARD_WIDTH_PX
        } else {
            DESKTOP_CARD_WIDTH_PX
        });
    }
}
