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
const OVERSCAN_CARDS: i64 = 2;
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

/// One visited card, retained at a stable slot identity as it moves around the loop.
#[derive(PartialEq)]
#[pax]
#[custom(Defaults)]
pub struct MarqueeCell {
    pub virtual_index: i64,
    pub slot_index: usize,
    pub x_px: f64,
}

/// A continuously scrolling horizontal card rail with retained visited cards.
///
/// The viewport plus overscan selects cards to mount for the first time. A card
/// keeps its own keyed slot after that, including across reversal and resize.
/// Only its offscreen position wraps; it is never recycled into another card.
/// `ScrollerHost` keeps these slot sites in this component's projection scope.
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
    pub _mounted_cells: Property<Vec<MarqueeCell>>,
    pub hover_paused: Property<bool>,
    pub touch_paused: Property<bool>,
    pub _last_frame_millis: Property<f64>,
    pub _window_base_virtual_index: Property<i64>,
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
            _mounted_cells: Property::new(vec![]),
            hover_paused: Property::new(false),
            touch_paused: Property::new(false),
            _last_frame_millis: Property::new(0.0),
            _window_base_virtual_index: Property::new(0),
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
        if !self.sync_geometry(ctx, false) {
            self._last_frame_millis.set(now);
            return;
        }

        let content_width = self._content_width_px.get();
        if content_width <= 0.0 || content_width <= self._last_viewport_width_px.get() {
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
        self.sync_window_for_scroll(normalized_scroll_x);

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
        self.sync_window_for_scroll(next_scroll_x);
    }

    fn sync_geometry(&mut self, ctx: &NodeContext, force: bool) -> bool {
        let count = ctx.projected_children_count.get();
        let (viewport_width, viewport_height) = ctx.bounds_self.get();
        let Some(stride) = ready_marquee_stride(
            self.card_width_px.local().get(),
            self.gap_px.local().get(),
            (viewport_width, viewport_height),
        ) else {
            // Bound inputs can still be settling at mount. A fabricated 1px
            // card would permanently inflate the retained projection window.
            return false;
        };
        let previous_stride = self._last_stride_px.get();
        let previous_content_width = self._content_width_px.get();
        let geometry_changed = count != self._last_projected_child_count.get()
            || (viewport_width - self._last_viewport_width_px.get()).abs() > 0.01
            || (viewport_height - self._last_viewport_height_px.get()).abs() > 0.01
            || (stride - previous_stride).abs() > 0.01;

        if !force && !geometry_changed {
            return true;
        }

        let content_width = count as f64 * stride;
        let next_scroll_x = if content_width <= viewport_width {
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
            .set_if_neq(if content_width > viewport_width {
                content_width * 3.0 + viewport_width
            } else {
                viewport_width
            });
        self._card_height_px
            .set_if_neq((viewport_height - SCROLLER_CHROME_PX).max(0.0));
        self._scroll_pos_x.set_if_neq(next_scroll_x);
        self._expected_scroll_x.set_if_neq(next_scroll_x);
        let base = if stride > 0.0 {
            (next_scroll_x / stride).floor() as i64
        } else {
            0
        };
        self._window_base_virtual_index.set_if_neq(base);
        self.reconcile_window(count, viewport_width, stride, base);
        true
    }

    fn reconcile_window(&mut self, count: usize, viewport_width: f64, stride: f64, base: i64) {
        let cells = reconcile_cells(
            self._mounted_cells.get(),
            count,
            viewport_width,
            stride,
            base,
        );
        self._mounted_cells.set_if_neq(cells);
    }

    fn sync_window_for_scroll(&mut self, scroll_x: f64) {
        let stride = self._last_stride_px.get();
        if stride <= 0.0 {
            return;
        }

        let base = (scroll_x / stride).floor() as i64;
        if base == self._window_base_virtual_index.get() {
            return;
        }

        self._window_base_virtual_index.set(base);
        self.reconcile_window(
            self._last_projected_child_count.get(),
            self._last_viewport_width_px.get(),
            stride,
            base,
        );
    }
}

fn ready_marquee_stride(card_width: f64, gap: f64, viewport: (f64, f64)) -> Option<f64> {
    if !card_width.is_finite()
        || card_width <= 0.0
        || !gap.is_finite()
        || !viewport.0.is_finite()
        || viewport.0 <= 0.0
        || !viewport.1.is_finite()
        || viewport.1 <= 0.0
    {
        return None;
    }
    let stride = card_width + gap.max(0.0);
    stride.is_finite().then_some(stride)
}

fn reconcile_cells(
    mut cells: Vec<MarqueeCell>,
    count: usize,
    viewport_width: f64,
    stride: f64,
    base: i64,
) -> Vec<MarqueeCell> {
    // Only an actual change to the authored children can remove a retained card.
    cells.retain(|cell| cell.slot_index < count);
    if count == 0 {
        return cells;
    }
    let looping = count as f64 * stride > viewport_width;
    let first = if looping { base - OVERSCAN_CARDS } else { 0 };
    let required = ((viewport_width / stride).ceil() as usize)
        .saturating_add((OVERSCAN_CARDS * 2 + 2) as usize)
        .min(count);
    for offset in 0..required {
        let slot_index = (first + offset as i64).rem_euclid(count as i64) as usize;
        if !cells.iter().any(|cell| cell.slot_index == slot_index) {
            cells.push(MarqueeCell {
                slot_index,
                virtual_index: 0,
                x_px: 0.0,
            });
        }
    }
    for cell in &mut cells {
        // All copies of this slot would alias the same projected node. Keep one
        // attachment and wrap it beyond the viewport when its old position exits.
        cell.virtual_index = first + (cell.slot_index as i64 - first).rem_euclid(count as i64);
        cell.x_px = cell.virtual_index as f64 * stride;
    }
    cells
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
#[custom(Default)]
pub struct FeatureGallery {
    pub content_height: Property<f64>,
    pub compact: Property<bool>,
    pub card_width_px: Property<f64>,
    pub marquee_hovered: Property<bool>,
    pub marquee_touch_active: Property<bool>,
}

impl Default for FeatureGallery {
    fn default() -> Self {
        Self {
            content_height: Property::new(0.0),
            compact: Property::new(false),
            card_width_px: Property::new(DESKTOP_CARD_WIDTH_PX),
            marquee_hovered: Property::new(false),
            marquee_touch_active: Property::new(false),
        }
    }
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
        // Descendant mount handlers must see this layout in the same graph
        // turn, not wait for a shared-state publication to be imported.
        self.compact.local().set_if_neq(compact);
        self.card_width_px.local().set_if_neq(if compact {
            COMPACT_CARD_WIDTH_PX
        } else {
            DESKTOP_CARD_WIDTH_PX
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const PERIOD: f64 = 14_924.0;

    fn assert_unique_window(rail: &MarqueeCarousel, count: usize, expected: usize) {
        let cells = rail._mounted_cells.get();
        assert_eq!(cells.len(), expected);
        assert!(cells.iter().all(|cell| cell.slot_index < count));
        let slots: HashSet<_> = cells.iter().map(|cell| cell.slot_index).collect();
        assert_eq!(slots.len(), cells.len());
    }

    #[test]
    fn gallery_starts_with_valid_card_geometry() {
        let gallery = FeatureGallery::default();
        assert_eq!(gallery.card_width_px.get(), 350.0);
        assert_eq!(
            ready_marquee_stride(350.0, 24.0, (1280.0, 522.0)),
            Some(374.0)
        );
        assert_eq!(
            ready_marquee_stride(286.0, 24.0, (390.0, 546.0)),
            Some(310.0)
        );
    }

    #[test]
    fn unready_or_nonfinite_geometry_does_not_allocate_a_window() {
        for width in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(ready_marquee_stride(width, 24.0, (1280.0, 522.0)), None);
            assert_eq!(ready_marquee_stride(350.0, 24.0, (width, 522.0)), None);
            assert_eq!(ready_marquee_stride(350.0, 24.0, (1280.0, width)), None);
        }
        assert_eq!(ready_marquee_stride(350.0, f64::NAN, (1280.0, 522.0)), None);
        assert_eq!(
            ready_marquee_stride(f64::MAX, f64::MAX, (1280.0, 522.0)),
            None
        );
    }

    #[test]
    fn startup_window_is_viewport_sized() {
        let mut rail = MarqueeCarousel::default();
        rail.reconcile_window(43, 1280.0, 374.0, 43);
        assert_unique_window(&rail, 43, 10);

        let mut mobile = MarqueeCarousel::default();
        mobile.reconcile_window(43, 390.0, 310.0, 43);
        assert_unique_window(&mobile, 43, 8);
    }

    #[test]
    fn resizing_retains_only_a_valid_bounded_high_water_window() {
        let mut rail = MarqueeCarousel::default();
        for (width, stride, expected) in
            [(390.0, 310.0, 8), (1280.0, 374.0, 10), (390.0, 310.0, 10)]
        {
            rail.reconcile_window(43, width, stride, 43);
            assert_unique_window(&rail, 43, expected);
        }
        rail.reconcile_window(43, 100_000.0, 374.0, 43);
        assert_unique_window(&rail, 43, 43);
    }

    #[test]
    fn short_or_changing_content_never_reuses_a_projected_child() {
        let mut rail = MarqueeCarousel::default();
        for count in [43, 3, 1, 0, 43] {
            rail.reconcile_window(count, 1280.0, 374.0, count as i64);
            assert_unique_window(&rail, count, count.min(10));
        }
    }

    #[test]
    fn unset_or_nonfinite_geometry_cannot_allocate_cards() {
        for card_width in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(
                ready_marquee_stride(card_width, 24.0, (1440.0, 522.0)),
                None
            );
        }
        assert_eq!(ready_marquee_stride(350.0, 24.0, (0.0, 522.0)), None);
        assert_eq!(ready_marquee_stride(350.0, 24.0, (1440.0, 0.0)), None);
        assert_eq!(ready_marquee_stride(350.0, f64::NAN, (1440.0, 522.0)), None);
        for (width, card, expected) in [(720.0, 286.0, 9), (1440.0, 350.0, 10), (1920.0, 350.0, 12)]
        {
            let stride = ready_marquee_stride(card, 24.0, (width, 522.0)).unwrap();
            let cells = reconcile_cells(vec![], 43, width, stride, 43);
            assert_eq!(cells.len(), expected);
        }
    }

    #[test]
    fn looping_and_reversal_preserve_visited_cards_without_duplicate_slots() {
        let mut cells = vec![];
        let mut visited = HashSet::new();
        for base in (21..150).chain((0..150).rev()) {
            cells = reconcile_cells(cells, 43, 1440.0, 374.0, base);
            let slots: HashSet<_> = cells.iter().map(|c| c.slot_index).collect();
            assert_eq!(slots.len(), cells.len());
            assert!(
                visited.is_subset(&slots),
                "scrolling cannot evict a visited card"
            );
            for visible in base..base + 5 {
                assert!(cells.iter().any(|c| c.virtual_index == visible));
            }
            for cell in &cells {
                assert_eq!(cell.virtual_index.rem_euclid(43) as usize, cell.slot_index);
                assert_eq!(cell.x_px, cell.virtual_index as f64 * 374.0);
            }
            visited = slots;
        }
        assert_eq!(cells.len(), 43);
    }

    #[test]
    fn resize_and_loop_recentering_preserve_card_identity() {
        let cells = reconcile_cells(vec![], 43, 1920.0, 374.0, 43);
        let slots: Vec<_> = cells.iter().map(|c| c.slot_index).collect();
        let narrow = reconcile_cells(cells, 43, 720.0, 310.0, 43);
        assert_eq!(
            narrow.iter().map(|c| c.slot_index).collect::<Vec<_>>(),
            slots
        );
        let recentered = reconcile_cells(narrow.clone(), 43, 720.0, 310.0, 86);
        assert_eq!(recentered.len(), narrow.len());
        for (a, b) in narrow.iter().zip(&recentered) {
            assert_eq!(a.slot_index, b.slot_index);
            assert_eq!(b.x_px - a.x_px, 43.0 * 310.0);
        }
    }

    #[test]
    fn short_rails_and_authored_removal_never_duplicate_slots() {
        let cells = reconcile_cells(vec![], 43, 1440.0, 374.0, 43);
        let short = reconcile_cells(cells, 2, 1440.0, 374.0, 43);
        assert_eq!(short.len(), 2);
        for cell in &short {
            assert_eq!(cell.virtual_index, cell.slot_index as i64);
            assert_eq!(cell.x_px, cell.slot_index as f64 * 374.0);
        }
        assert!(reconcile_cells(short, 0, 1440.0, 374.0, 0).is_empty());
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
