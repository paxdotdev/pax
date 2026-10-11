#![allow(unused_imports)]

use crate::SiteTheme;
use pax_kit::math::Point2;
use pax_kit::*;

pub mod paint_demo;
use crate::feature_gallery::paint_demo::FeaturePaintDemo;
pub mod studies;
use crate::feature_gallery::studies::FeatureStudy;

const COMPACT_BREAKPOINT_PX: f64 = 760.0;
const DESKTOP_CARD_WIDTH_PX: f64 = 350.0;
const COMPACT_CARD_WIDTH_PX: f64 = 286.0;
const CARD_GAP_PX: f64 = 14.0;
const MARQUEE_SPEED_PX_PER_SECOND: f64 = 30.0;
const SCROLLER_CHROME_PX: f64 = 12.0;
const TOUCH_MOMENTUM_SETTLE_MILLIS: f64 = 600.0;

/// A single gallery card. Its media area is projected content so a card
/// can contain Pax-native artwork today and richer image/video/example content
/// later without changing a content schema.
#[pax]
#[file("feature_gallery/card.pax")]
pub struct FeatureCard {
    pub art_scale: Property<f64>,
    pub category: Property<String>,
    pub title: Property<String>,
    pub summary: Property<String>,
    pub proof: Property<String>,
    pub docs_label: Property<String>,
    pub docs_url: Property<String>,
    pub marquee_hovered: Property<bool>,
    pub marquee_touch_active: Property<bool>,
}

impl FeatureCard {
    pub fn measure_artboard(&mut self, ctx: &NodeContext) {
        self.art_scale
            .set_if_neq(artboard_scale(ctx.bounds_self.get().0));
    }

    pub fn pause_marquee_for_mouse(&mut self, _ctx: &NodeContext, _event: Event<MouseOver>) {
        self.marquee_hovered.set(true);
    }

    pub fn resume_marquee_for_mouse(&mut self, _ctx: &NodeContext, _event: Event<MouseOut>) {
        self.marquee_hovered.set(false);
    }

    pub fn pause_marquee_for_touch(&mut self, _ctx: &NodeContext, _event: Event<TouchStart>) {
        self.marquee_touch_active.set(true);
    }

    pub fn resume_marquee_for_touch(&mut self, _ctx: &NodeContext, _event: Event<TouchEnd>) {
        self.marquee_touch_active.set(false);
    }

    pub fn cancel_marquee_touch(&mut self, _ctx: &NodeContext, _event: Event<TouchCancel>) {
        self.marquee_touch_active.set(false);
    }
}

fn artboard_scale(card_width: f64) -> f64 {
    ((card_width - 48.0) / 288.0).clamp(0.0, 1.0)
}

/// A continuously scrolling horizontal rail with every card permanently projected.
///
/// Direct manipulation belongs to a snap-free native scroller surface; the Pax
/// side advances that same scroll position for marquee motion. Wrapping moves
/// distant cards by one complete period; it never recycles their slot ownership.
/// `ScrollerHost` keeps those slots owned by this component, without another
/// projection boundary. GPU surface culling remains the renderer's concern.
#[pax]
#[file("feature_gallery/marquee.pax")]
#[custom(Default)]
pub struct MarqueeCarousel {
    pub card_width_px: Property<f64>,
    pub gap_px: Property<f64>,
    pub speed_px_per_second: Property<f64>,

    pub _scroll_pos_x: Property<f64>,
    pub _scroll_width_px: Property<f64>,
    pub _content_width_px: Property<f64>,
    pub _card_height_px: Property<f64>,
    pub _card_positions: Property<Vec<f64>>,
    pub hover_paused: Property<bool>,
    pub touch_paused: Property<bool>,
    pub _last_frame_millis: Property<f64>,
    pub _placement_base: Property<i64>,
    pub _last_projected_child_count: Property<usize>,
    pub _last_viewport_width_px: Property<f64>,
    pub _last_viewport_height_px: Property<f64>,
    pub _last_stride_px: Property<f64>,
    pub _expected_scroll_x: Property<f64>,
    pub _resume_after_millis: Property<f64>,
}

impl Default for MarqueeCarousel {
    fn default() -> Self {
        Self {
            card_width_px: Property::new(DESKTOP_CARD_WIDTH_PX),
            gap_px: Property::new(CARD_GAP_PX),
            speed_px_per_second: Property::new(MARQUEE_SPEED_PX_PER_SECOND),
            _scroll_pos_x: Property::new(0.0),
            _scroll_width_px: Property::new(0.0),
            _content_width_px: Property::new(0.0),
            _card_height_px: Property::new(0.0),
            _card_positions: Property::new(vec![]),
            hover_paused: Property::new(false),
            touch_paused: Property::new(false),
            _last_frame_millis: Property::new(0.0),
            _placement_base: Property::new(0),
            _last_projected_child_count: Property::new(usize::MAX),
            _last_viewport_width_px: Property::new(-1.0),
            _last_viewport_height_px: Property::new(-1.0),
            _last_stride_px: Property::new(-1.0),
            _expected_scroll_x: Property::new(0.0),
            _resume_after_millis: Property::new(0.0),
        }
    }
}

impl MarqueeCarousel {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self._last_frame_millis
            .set(ctx.elapsed_time_millis() as f64);
        self.sync_geometry(ctx, true);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        let now = ctx.elapsed_time_millis() as f64;
        self.sync_geometry(ctx, false);

        let content_width = self._content_width_px.get();
        if content_width <= 0.0 {
            return;
        }

        let scroll_x = self._scroll_pos_x.get();
        let expected_scroll_x = self._expected_scroll_x.get();
        if (scroll_x - expected_scroll_x).abs() > 0.5 {
            self._resume_after_millis
                .set(now + TOUCH_MOMENTUM_SETTLE_MILLIS);
            self._expected_scroll_x.set(scroll_x);
        }

        let normalized_scroll_x = normalize_loop_scroll(scroll_x, content_width);
        if (normalized_scroll_x - self._scroll_pos_x.get()).abs() > 0.01 {
            self._scroll_pos_x.set(normalized_scroll_x);
            self._expected_scroll_x.set(normalized_scroll_x);
        }
        self.sync_positions(normalized_scroll_x);

        let previous = self._last_frame_millis.get();
        self._last_frame_millis.set(now);
        // Keep the lead card at the start until the rail actually enters view.
        let origin = ctx.local_point(Point2::new(0.0, 0.0));
        let height = ctx.bounds_self.get().1;
        let offscreen =
            -origin.y * height >= ctx.viewport.get().height || (1.0 - origin.y) * height <= 0.0;
        let paused = offscreen
            || self.hover_paused.get()
            || self.touch_paused.get()
            || now < self._resume_after_millis.get();
        if paused {
            return;
        }

        let delta_seconds = ((now - previous).max(0.0).min(50.0)) / 1000.0;
        let next_scroll_x = normalize_loop_scroll(
            normalized_scroll_x + self.speed_px_per_second.get().max(0.0) * delta_seconds,
            content_width,
        );
        self._scroll_pos_x.set_if_neq(next_scroll_x);
        self._expected_scroll_x.set_if_neq(next_scroll_x);
        self.sync_positions(next_scroll_x);
    }

    fn sync_geometry(&mut self, ctx: &NodeContext, force: bool) {
        let count = ctx.projected_children_count.get();
        let card_width = self.card_width_px.get().max(1.0);
        let gap = self.gap_px.get().max(0.0);
        let stride = card_width + gap;
        let (viewport_width, viewport_height) = ctx.bounds_self.get();
        let viewport_width = viewport_width.max(0.0);
        let viewport_height = viewport_height.max(0.0);
        let previous_stride = self._last_stride_px.get();
        let previous_content_width = self._content_width_px.get();
        let geometry_changed = count != self._last_projected_child_count.get()
            || (viewport_width - self._last_viewport_width_px.get()).abs() > 0.01
            || (viewport_height - self._last_viewport_height_px.get()).abs() > 0.01
            || (stride - previous_stride).abs() > 0.01;

        if !force && !geometry_changed {
            return;
        }

        let content_width = count as f64 * stride;
        let next_scroll_x = if content_width <= 0.0 {
            0.0
        } else if previous_content_width > 0.0 {
            let phase = self._scroll_pos_x.get().rem_euclid(previous_content_width)
                / previous_content_width;
            content_width * (1.0 + phase)
        } else {
            content_width
        };
        self._last_projected_child_count.set_if_neq(count);
        self._last_viewport_width_px.set_if_neq(viewport_width);
        self._last_viewport_height_px.set_if_neq(viewport_height);
        self._last_stride_px.set_if_neq(stride);
        self._content_width_px.set_if_neq(content_width);
        self._scroll_width_px
            .set_if_neq((content_width * 3.0 + viewport_width).max(viewport_width));
        self._card_height_px
            .set_if_neq((viewport_height - SCROLLER_CHROME_PX).max(0.0));
        self._scroll_pos_x.set_if_neq(next_scroll_x);
        self._expected_scroll_x.set_if_neq(next_scroll_x);
        let base = if stride > 0.0 {
            ((next_scroll_x + viewport_width * 0.5) / stride).floor() as i64
        } else {
            0
        };
        self._placement_base.set_if_neq(base);
        self._card_positions
            .set_if_neq(card_positions(count, stride, base));
    }

    fn sync_positions(&mut self, scroll_x: f64) {
        let stride = self._last_stride_px.get();
        if stride <= 0.0 {
            return;
        }

        let base = ((scroll_x + self._last_viewport_width_px.get() * 0.5) / stride).floor() as i64;
        if base == self._placement_base.get() {
            return;
        }

        self._placement_base.set(base);
        self._card_positions.set_if_neq(card_positions(
            self._last_projected_child_count.get(),
            stride,
            base,
        ));
    }
}

fn card_positions(count: usize, stride: f64, center_card: i64) -> Vec<f64> {
    if count == 0 {
        return vec![];
    }
    // One complete cycle centered around the viewport. Each vector index is
    // always the same authored slot; only its offscreen wrap position changes.
    let first = center_card - count as i64 / 2;
    (0..count)
        .map(|slot| (first + (slot as i64 - first).rem_euclid(count as i64)) as f64 * stride)
        .collect()
}

fn normalize_loop_scroll(scroll_x: f64, content_width: f64) -> f64 {
    if content_width <= 0.0 || !scroll_x.is_finite() {
        return 0.0;
    }
    if scroll_x < content_width * 0.5 || scroll_x > content_width * 2.5 {
        content_width + scroll_x.rem_euclid(content_width)
    } else {
        scroll_x
    }
}

/// The content-authored feature heap used on the Pax home page.
#[pax]
#[file("feature_gallery/gallery.pax")]
pub struct FeatureGallery {
    pub content_height: Property<f64>,
    pub compact: Property<bool>,
    pub card_width_px: Property<f64>,
    pub marquee_hovered: Property<bool>,
    pub marquee_touch_active: Property<bool>,
}

impl FeatureGallery {
    pub fn handle_mount(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
    }

    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
    }

    fn sync_layout(&mut self, ctx: &NodeContext) {
        crate::section_layout::publish_height(ctx, "gallery_content", &self.content_height);
        let compact = ctx.bounds_self.get().0 < COMPACT_BREAKPOINT_PX;
        self.compact.set_if_neq(compact);
        self.card_width_px.set_if_neq(if compact {
            COMPACT_CARD_WIDTH_PX
        } else {
            DESKTOP_CARD_WIDTH_PX
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{card_positions, normalize_loop_scroll};

    const PERIOD: f64 = 14_924.0;

    #[test]
    fn every_authored_slot_stays_present_and_keeps_its_identity() {
        let count = include_str!("gallery.pax").matches("<FeatureCard ").count();
        let stride = 374.0;
        for center in -(count as i64)..4 * count as i64 {
            let positions = card_positions(count, stride, center);
            assert_eq!(positions.len(), count);
            let mut ordered = vec![];
            for (slot, x) in positions.into_iter().enumerate() {
                let index = (x / stride).round() as i64;
                assert_eq!(index.rem_euclid(count as i64) as usize, slot);
                ordered.push(index);
            }
            ordered.sort_unstable();
            assert!(ordered.windows(2).all(|pair| pair[1] == pair[0] + 1));
        }
        let template = include_str!("marquee.pax");
        assert!(template.contains("for (position, index) in self._card_positions"));
        assert!(template.contains("slot(index)"));
        assert!(!template.contains("_visible_cells"));
    }

    #[test]
    fn wrapping_only_moves_the_farthest_card_by_one_full_period() {
        let count = 43;
        let stride = 374.0;
        for center in 0..3 * count as i64 {
            let before = card_positions(count, stride, center);
            let after = card_positions(count, stride, center + 1);
            let changed: Vec<_> = before.iter().zip(&after).filter(|(a, b)| a != b).collect();
            assert_eq!(changed.len(), 1);
            assert_eq!(changed[0].1 - changed[0].0, count as f64 * stride);
            assert!((changed[0].0 / stride - center as f64).abs() >= 20.0);
            assert_eq!(card_positions(count, stride, center), before);
        }
        assert!(card_positions(0, stride, 0).is_empty());
    }

    #[test]
    fn recentering_and_resizing_preserve_the_complete_cycle() {
        for (stride, width) in [(310.0, 390.0), (374.0, 1280.0), (374.0, 1920.0)] {
            let count = 43;
            let period = stride * count as f64;
            for phase in [0.25, 0.8, 1.5, 2.25, 2.75] {
                let scroll = phase * period;
                let normalized = normalize_loop_scroll(scroll, period);
                let positions = |x: f64| {
                    card_positions(count, stride, ((x + width * 0.5) / stride).floor() as i64)
                };
                for (a, b) in positions(scroll).iter().zip(positions(normalized)) {
                    assert!(((a - scroll) - (b - normalized)).abs() < 1e-8);
                }
                let placed = positions(normalized);
                assert!(placed.iter().copied().fold(f64::INFINITY, f64::min) < normalized);
                assert!(
                    placed.iter().copied().fold(f64::NEG_INFINITY, f64::max) > normalized + width
                );
            }
        }
    }

    #[test]
    fn artboards_fit_both_card_sizes_with_stroke_bleed() {
        for width in [286.0, 350.0] {
            let scale = super::artboard_scale(width);
            assert!(288.0 * scale <= width - 48.0);
            assert!(144.0 * scale <= 144.0);
            assert!(scale > 0.8);
        }
    }

    #[test]
    fn loop_scroll_stays_native_inside_the_middle_cycles() {
        assert_eq!(normalize_loop_scroll(PERIOD, PERIOD), PERIOD);
        assert_eq!(normalize_loop_scroll(PERIOD * 2.25, PERIOD), PERIOD * 2.25);
    }

    #[test]
    fn loop_scroll_recenters_without_changing_its_visual_phase() {
        assert_eq!(normalize_loop_scroll(PERIOD * 2.75, PERIOD), PERIOD * 1.75);
        assert_eq!(normalize_loop_scroll(PERIOD * 0.25, PERIOD), PERIOD * 1.25);
    }
}
