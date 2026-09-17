#![allow(unused_imports)]

use crate::SiteTheme;
use pax_kit::*;

const COMPACT_BREAKPOINT_PX: f64 = 760.0;
const DESKTOP_CARD_WIDTH_PX: f64 = 350.0;
const COMPACT_CARD_WIDTH_PX: f64 = 286.0;
const CARD_GAP_PX: f64 = 14.0;
const MARQUEE_SPEED_PX_PER_SECOND: f64 = 24.0;
const CULL_OVERSCAN_CARDS: i64 = 2;
const SCROLLER_CHROME_PX: f64 = 12.0;
const TOUCH_MOMENTUM_SETTLE_MILLIS: f64 = 600.0;

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
}

impl FeatureCard {
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

/// One structurally mounted card in the marquee's viewport plus overscan.
#[derive(PartialEq)]
#[pax]
#[custom(Defaults)]
pub struct MarqueeCell {
    pub virtual_index: i64,
    pub slot_index: usize,
    pub x_px: f64,
}

/// A continuously scrolling, virtualized horizontal card rail.
///
/// Direct manipulation belongs to a snap-free native scroller surface; the Pax
/// side advances that same scroll position for marquee motion and only changes
/// the mounted window when the viewport crosses a card boundary. The template
/// uses `ScrollerHost` directly so its dynamic `slot(...)` sites remain owned by
/// this component rather than crossing a second projection boundary.
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
    pub _visible_cells: Property<Vec<MarqueeCell>>,
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
            _visible_cells: Property::new(vec![]),
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
        self.sync_window_for_scroll(normalized_scroll_x);

        let previous = self._last_frame_millis.get();
        self._last_frame_millis.set(now);
        let paused = self.hover_paused.get()
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
            (next_scroll_x / stride).floor() as i64
        } else {
            0
        };
        self._window_base_virtual_index.set_if_neq(base);
        self.reconcile_window(count, viewport_width, stride, base);
    }

    fn reconcile_window(&mut self, count: usize, viewport_width: f64, stride: f64, base: i64) {
        let cells = if count == 0 {
            vec![]
        } else {
            let first = base - CULL_OVERSCAN_CARDS;
            let required_cell_count =
                (viewport_width / stride).ceil() as usize + (CULL_OVERSCAN_CARDS * 2 + 2) as usize;
            // Keep the mounted window at its high-water size. Removing a
            // repeat item during pre-render can invalidate that item's PaxEL
            // dependencies before the frame finishes.
            let mut cells = self._visible_cells.get();
            let mounted_cell_count = cells.len().max(required_cell_count);
            let desired_indices: Vec<_> = (0..mounted_cell_count)
                .map(|offset| first + offset as i64)
                .collect();
            let mut missing_indices: Vec<_> = desired_indices
                .iter()
                .copied()
                .filter(|desired| !cells.iter().any(|cell| cell.virtual_index == *desired))
                .collect();

            // Preserve cells whose virtual position remains in the overscan
            // window. Recycle only departed cells, preferring a replacement
            // with the same projected slot across a complete marquee period.
            for cell in &mut cells {
                if desired_indices.contains(&cell.virtual_index) {
                    continue;
                }
                let replacement = missing_indices
                    .iter()
                    .position(|candidate| {
                        candidate.rem_euclid(count as i64) as usize == cell.slot_index
                    })
                    .unwrap_or(0);
                cell.virtual_index = missing_indices.remove(replacement);
            }
            cells.extend(
                missing_indices
                    .into_iter()
                    .map(|virtual_index| MarqueeCell {
                        virtual_index,
                        slot_index: virtual_index.rem_euclid(count as i64) as usize,
                        x_px: 0.0,
                    }),
            );
            for cell in &mut cells {
                cell.slot_index = cell.virtual_index.rem_euclid(count as i64) as usize;
                cell.x_px = cell.virtual_index as f64 * stride;
            }
            cells
        };
        self._visible_cells.set_if_neq(cells);
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
    use super::normalize_loop_scroll;

    const PERIOD: f64 = 14_924.0;

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
