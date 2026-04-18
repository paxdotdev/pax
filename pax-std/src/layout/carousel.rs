#![allow(unused_imports)]
use crate::*;
use pax_engine::api::{Axis, Event, KeyDown, Numeric, Property, Size};
use pax_engine::*;
use pax_runtime::api::NodeContext;

const DOT_DIAMETER: f64 = 10.0;
const DOT_GAP: f64 = 10.0;
const DOT_PILL_PADDING_X: f64 = 14.0;
const DOT_PILL_THICKNESS: f64 = 26.0;

/// A paged scrolling container with native scroll snapping and optional page dots.
///
/// Each slotted child becomes one page. The carousel lays pages out along `axis`,
/// sizes each page with `page_size`, and binds its scroll position through an
/// internal `Scroller`.
#[pax]
#[engine_import_path("pax_engine")]
#[file("layout/carousel.pax")]
#[custom(Default)]
pub struct Carousel {
    /// Axis along which pages are laid out and snapped.
    pub axis: Property<CarouselAxis>,
    /// Size of each page along the scroll axis (defaults to 100%).
    pub page_size: Property<Size>,
    /// Whether to show page-position dots when there is more than one page.
    pub show_dots: Property<bool>,
    /// Horizontal scroll position, in pixels.
    pub scroll_pos_x: Property<f64>,
    /// Vertical scroll position, in pixels.
    pub scroll_pos_y: Property<f64>,

    // Computed page layouts for the inline template.
    pub _pages: Property<Vec<CarouselCell>>,
    // Computed page width in pixels.
    pub _page_width_px: Property<f64>,
    // Computed page height in pixels.
    pub _page_height_px: Property<f64>,
    // Computed scrollable content width.
    pub _scroll_width: Property<Size>,
    // Computed scrollable content height.
    pub _scroll_height: Property<Size>,
    // Computed horizontal scroll snap anchors.
    pub _snap_positions_x: Property<Vec<Size>>,
    // Computed vertical scroll snap anchors.
    pub _snap_positions_y: Property<Vec<Size>>,
    // Computed number of pages.
    pub _page_count: Property<usize>,
    // Computed active page index.
    pub _active_page: Property<usize>,
    // Computed visibility for dots after accounting for page count.
    pub _show_dots: Property<bool>,
    // Computed dot positions and active states.
    pub _dots: Property<Vec<CarouselDot>>,
    // Computed total dot row/column length.
    pub _dots_width: Property<f64>,
    // Computed dot-pill width.
    pub _dots_pill_width: Property<f64>,
    // Computed dot-pill height.
    pub _dots_pill_height: Property<f64>,
    // Computed dot-pill x position.
    pub _dots_pos_x: Property<Size>,
    // Computed dot-pill y position.
    pub _dots_pos_y: Property<Size>,
}

impl Default for Carousel {
    fn default() -> Self {
        Self {
            axis: Property::new(CarouselAxis::Horizontal),
            page_size: Property::new(Size::Percent(Numeric::F64(100.0))),
            show_dots: Property::new(false),
            scroll_pos_x: Property::new(0.0),
            scroll_pos_y: Property::new(0.0),
            _pages: Property::new(vec![]),
            _page_width_px: Property::new(0.0),
            _page_height_px: Property::new(0.0),
            _scroll_width: Property::new(Size::Pixels(Numeric::F64(0.0))),
            _scroll_height: Property::new(Size::Pixels(Numeric::F64(0.0))),
            _snap_positions_x: Property::new(vec![]),
            _snap_positions_y: Property::new(vec![]),
            _page_count: Property::new(0),
            _active_page: Property::new(0),
            _show_dots: Property::new(false),
            _dots: Property::new(vec![]),
            _dots_width: Property::new(0.0),
            _dots_pill_width: Property::new(0.0),
            _dots_pill_height: Property::new(0.0),
            _dots_pos_x: Property::new(Size::Percent(Numeric::F64(50.0))),
            _dots_pos_y: Property::new(Size::Percent(Numeric::F64(92.0))),
        }
    }
}

/// Direction for carousel paging and scroll snapping.
#[pax]
#[engine_import_path("pax_engine")]
pub enum CarouselAxis {
    /// Pages flow left-to-right.
    #[default]
    Horizontal,
    /// Pages flow top-to-bottom.
    Vertical,
}

// Internal page layout emitted into the inline template.
#[pax]
#[engine_import_path("pax_engine")]
pub struct CarouselCell {
    // Page x offset in pixels.
    pub x_px: f64,
    // Page y offset in pixels.
    pub y_px: f64,
}

// Internal page-dot layout emitted into the inline template.
#[pax]
#[engine_import_path("pax_engine")]
pub struct CarouselDot {
    // Dot x position in pixels.
    pub x_px: f64,
    // Dot y position in pixels.
    pub y_px: f64,
    // Whether this dot represents the active page.
    pub is_active: bool,
}

struct CarouselLayout {
    pages: Vec<CarouselCell>,
    page_width_px: f64,
    page_height_px: f64,
    scroll_width: Size,
    scroll_height: Size,
    snap_positions_x: Vec<Size>,
    snap_positions_y: Vec<Size>,
}

impl Carousel {
    // Wires derived layout properties used by the inline template.
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let axis = self.axis.clone();
        let page_size = self.page_size.clone();
        let bounds = ctx.bounds_self.clone();
        let slot_children_count = ctx.slot_children_count.clone();
        let show_dots = self.show_dots.clone();
        let scroll_pos_x = self.scroll_pos_x.clone();
        let scroll_pos_y = self.scroll_pos_y.clone();

        let deps = [
            axis.untyped(),
            page_size.untyped(),
            bounds.untyped(),
            slot_children_count.untyped(),
        ];

        let axis_pages = axis.clone();
        let page_size_pages = page_size.clone();
        let bounds_pages = bounds.clone();
        let slot_children_count_pages = slot_children_count.clone();
        self._pages.replace_with(Property::computed(
            move || {
                compute_layout(
                    axis_pages.get(),
                    page_size_pages.get(),
                    bounds_pages.get(),
                    slot_children_count_pages.get(),
                )
                .pages
            },
            &deps,
        ));

        let axis_width = axis.clone();
        let page_size_width = page_size.clone();
        let bounds_width = bounds.clone();
        let slot_children_count_width = slot_children_count.clone();
        self._page_width_px.replace_with(Property::computed(
            move || {
                compute_layout(
                    axis_width.get(),
                    page_size_width.get(),
                    bounds_width.get(),
                    slot_children_count_width.get(),
                )
                .page_width_px
            },
            &deps,
        ));

        let axis_height = axis.clone();
        let page_size_height = page_size.clone();
        let bounds_height = bounds.clone();
        let slot_children_count_height = slot_children_count.clone();
        self._page_height_px.replace_with(Property::computed(
            move || {
                compute_layout(
                    axis_height.get(),
                    page_size_height.get(),
                    bounds_height.get(),
                    slot_children_count_height.get(),
                )
                .page_height_px
            },
            &deps,
        ));

        let axis_scroll_width = axis.clone();
        let page_size_scroll_width = page_size.clone();
        let bounds_scroll_width = bounds.clone();
        let slot_children_count_scroll_width = slot_children_count.clone();
        self._scroll_width.replace_with(Property::computed(
            move || {
                compute_layout(
                    axis_scroll_width.get(),
                    page_size_scroll_width.get(),
                    bounds_scroll_width.get(),
                    slot_children_count_scroll_width.get(),
                )
                .scroll_width
            },
            &deps,
        ));

        let axis_scroll_height = axis.clone();
        let page_size_scroll_height = page_size.clone();
        let bounds_scroll_height = bounds.clone();
        let slot_children_count_scroll_height = slot_children_count.clone();
        self._scroll_height.replace_with(Property::computed(
            move || {
                compute_layout(
                    axis_scroll_height.get(),
                    page_size_scroll_height.get(),
                    bounds_scroll_height.get(),
                    slot_children_count_scroll_height.get(),
                )
                .scroll_height
            },
            &deps,
        ));

        let axis_snap_x = axis.clone();
        let page_size_snap_x = page_size.clone();
        let bounds_snap_x = bounds.clone();
        let slot_children_count_snap_x = slot_children_count.clone();
        self._snap_positions_x.replace_with(Property::computed(
            move || {
                compute_layout(
                    axis_snap_x.get(),
                    page_size_snap_x.get(),
                    bounds_snap_x.get(),
                    slot_children_count_snap_x.get(),
                )
                .snap_positions_x
            },
            &deps,
        ));

        let axis_snap_y = axis.clone();
        let page_size_snap_y = page_size.clone();
        let bounds_snap_y = bounds.clone();
        let slot_children_count_snap_y = slot_children_count.clone();
        self._snap_positions_y.replace_with(Property::computed(
            move || {
                compute_layout(
                    axis_snap_y.get(),
                    page_size_snap_y.get(),
                    bounds_snap_y.get(),
                    slot_children_count_snap_y.get(),
                )
                .snap_positions_y
            },
            &deps,
        ));

        let slot_children_count_count = slot_children_count.clone();
        self._page_count.replace_with(Property::computed(
            move || slot_children_count_count.get(),
            &[slot_children_count.untyped()],
        ));

        let axis_active = axis.clone();
        let page_size_active = page_size.clone();
        let bounds_active = bounds.clone();
        let slot_children_count_active = slot_children_count.clone();
        let scroll_pos_x_active = scroll_pos_x.clone();
        let scroll_pos_y_active = scroll_pos_y.clone();
        self._active_page.replace_with(Property::computed(
            move || {
                active_page_index(
                    axis_active.get(),
                    page_size_active.get(),
                    bounds_active.get(),
                    slot_children_count_active.get(),
                    scroll_pos_x_active.get(),
                    scroll_pos_y_active.get(),
                )
            },
            &[
                axis.untyped(),
                page_size.untyped(),
                bounds.untyped(),
                slot_children_count.untyped(),
                scroll_pos_x.untyped(),
                scroll_pos_y.untyped(),
            ],
        ));

        let show_dots_prop = show_dots.clone();
        let slot_children_count_dots = slot_children_count.clone();
        self._show_dots.replace_with(Property::computed(
            move || show_dots_prop.get() && slot_children_count_dots.get() > 1,
            &[show_dots.untyped(), slot_children_count.untyped()],
        ));

        let axis_dots = axis.clone();
        let show_dots_specs = self._show_dots.clone();
        let active_page_specs = self._active_page.clone();
        let slot_children_count_specs = slot_children_count.clone();
        self._dots.replace_with(Property::computed(
            move || {
                let count = slot_children_count_specs.get();
                if !show_dots_specs.get() || count <= 1 {
                    return vec![];
                }
                let active_index = active_page_specs.get();
                let spacing = DOT_DIAMETER + DOT_GAP;
                let total_width = (count as f64 - 1.0) * spacing + DOT_DIAMETER;
                let start_x = -total_width * 0.5 + DOT_DIAMETER * 0.5;
                let is_vertical = matches!(axis_dots.get(), CarouselAxis::Vertical);
                let mut dots = Vec::with_capacity(count);
                for i in 0..count {
                    let offset = start_x + i as f64 * spacing;
                    dots.push(CarouselDot {
                        x_px: if is_vertical { 0.0 } else { offset },
                        y_px: if is_vertical { offset } else { 0.0 },
                        is_active: i == active_index,
                    });
                }
                dots
            },
            &[
                self._show_dots.untyped(),
                self._active_page.untyped(),
                axis.untyped(),
                slot_children_count.untyped(),
            ],
        ));

        let show_dots_width = self._show_dots.clone();
        let slot_children_count_width = slot_children_count.clone();
        self._dots_width.replace_with(Property::computed(
            move || {
                let count = slot_children_count_width.get();
                if !show_dots_width.get() || count <= 1 {
                    return 0.0;
                }
                (count as f64 - 1.0) * (DOT_DIAMETER + DOT_GAP) + DOT_DIAMETER
            },
            &[self._show_dots.untyped(), slot_children_count.untyped()],
        ));

        let axis_pill_width = axis.clone();
        let dots_width_prop = self._dots_width.clone();
        self._dots_pill_width.replace_with(Property::computed(
            move || {
                if matches!(axis_pill_width.get(), CarouselAxis::Vertical) {
                    DOT_PILL_THICKNESS
                } else {
                    dots_width_prop.get() + DOT_PILL_PADDING_X * 2.0
                }
            },
            &[axis.untyped(), self._dots_width.untyped()],
        ));

        let axis_pill_height = axis.clone();
        let dots_height_prop = self._dots_width.clone();
        self._dots_pill_height.replace_with(Property::computed(
            move || {
                if matches!(axis_pill_height.get(), CarouselAxis::Vertical) {
                    dots_height_prop.get() + DOT_PILL_PADDING_X * 2.0
                } else {
                    DOT_PILL_THICKNESS
                }
            },
            &[axis.untyped(), self._dots_width.untyped()],
        ));

        let axis_pos = axis.clone();
        let axis_pos_dep = axis.clone();
        self._dots_pos_x.replace_with(Property::computed(
            move || {
                if matches!(axis_pos.get(), CarouselAxis::Vertical) {
                    Size::Percent(Numeric::F64(6.0))
                } else {
                    Size::Percent(Numeric::F64(50.0))
                }
            },
            &[axis_pos_dep.untyped()],
        ));

        let axis_pos_y = axis.clone();
        let axis_pos_y_dep = axis.clone();
        self._dots_pos_y.replace_with(Property::computed(
            move || {
                if matches!(axis_pos_y.get(), CarouselAxis::Vertical) {
                    Size::Percent(Numeric::F64(50.0))
                } else {
                    Size::Percent(Numeric::F64(92.0))
                }
            },
            &[axis_pos_y_dep.untyped()],
        ));
    }

    // Keyboard paging for arrow-key navigation.
    pub fn key_down(&mut self, ctx: &NodeContext, args: Event<KeyDown>) {
        let key = args.keyboard.key.as_str();
        let axis = self.axis.get();
        let delta = match axis {
            CarouselAxis::Horizontal => match key {
                "ArrowLeft" => -1,
                "ArrowRight" => 1,
                _ => 0,
            },
            CarouselAxis::Vertical => match key {
                "ArrowUp" => -1,
                "ArrowDown" => 1,
                _ => 0,
            },
        };
        if delta == 0 {
            return;
        }
        let page_count = ctx.slot_children_count.get();
        if page_count <= 1 {
            return;
        }
        let bounds = ctx.bounds_self.get();
        let page_extent_px = page_extent_px(axis.clone(), self.page_size.get(), bounds, page_count);
        if page_extent_px <= 0.0 {
            return;
        }
        let current_scroll = match axis {
            CarouselAxis::Horizontal => self.scroll_pos_x.get(),
            CarouselAxis::Vertical => self.scroll_pos_y.get(),
        };
        let current_index = (current_scroll / page_extent_px).round() as isize;
        let mut next_index = current_index + delta as isize;
        if next_index < 0 {
            next_index = 0;
        }
        let last_index = page_count.saturating_sub(1) as isize;
        if next_index > last_index {
            next_index = last_index;
        }
        let target_scroll = page_extent_px * next_index as f64;
        match axis {
            CarouselAxis::Horizontal => self.scroll_pos_x.set(target_scroll),
            CarouselAxis::Vertical => self.scroll_pos_y.set(target_scroll),
        }
    }
}

fn page_extent_px(
    axis: CarouselAxis,
    page_size: Size,
    bounds: (f64, f64),
    page_count: usize,
) -> f64 {
    let axis_extent = match axis {
        CarouselAxis::Horizontal => bounds.0,
        CarouselAxis::Vertical => bounds.1,
    };
    if page_count <= 1 {
        return axis_extent;
    }
    match axis {
        CarouselAxis::Horizontal => page_size.evaluate(bounds, Axis::X),
        CarouselAxis::Vertical => page_size.evaluate(bounds, Axis::Y),
    }
}

fn active_page_index(
    axis: CarouselAxis,
    page_size: Size,
    bounds: (f64, f64),
    page_count: usize,
    scroll_pos_x: f64,
    scroll_pos_y: f64,
) -> usize {
    if page_count <= 1 {
        return 0;
    }
    let page_extent = page_extent_px(axis.clone(), page_size, bounds, page_count);
    if page_extent <= 0.0 {
        return 0;
    }
    let scroll_pos = match axis {
        CarouselAxis::Horizontal => scroll_pos_x,
        CarouselAxis::Vertical => scroll_pos_y,
    };
    let mut idx = (scroll_pos / page_extent).round() as isize;
    if idx < 0 {
        idx = 0;
    }
    let max_idx = page_count.saturating_sub(1) as isize;
    if idx > max_idx {
        idx = max_idx;
    }
    idx as usize
}

fn compute_layout(
    axis: CarouselAxis,
    page_size: Size,
    bounds: (f64, f64),
    page_count: usize,
) -> CarouselLayout {
    let axis_extent_px = match axis {
        CarouselAxis::Horizontal => bounds.0,
        CarouselAxis::Vertical => bounds.1,
    };
    let cross_extent_px = match axis {
        CarouselAxis::Horizontal => bounds.1,
        CarouselAxis::Vertical => bounds.0,
    };
    let page_extent_px = if page_count <= 1 {
        axis_extent_px
    } else {
        match axis {
            CarouselAxis::Horizontal => page_size.evaluate(bounds, Axis::X),
            CarouselAxis::Vertical => page_size.evaluate(bounds, Axis::Y),
        }
    };
    let scroll_pages = page_count.max(1) as f64;
    let total_extent_px = page_extent_px * scroll_pages;

    let (scroll_width_px, scroll_height_px) = match axis {
        CarouselAxis::Horizontal => (total_extent_px, cross_extent_px),
        CarouselAxis::Vertical => (cross_extent_px, total_extent_px),
    };
    let (page_width_px, page_height_px) = match axis {
        CarouselAxis::Horizontal => (page_extent_px, cross_extent_px),
        CarouselAxis::Vertical => (cross_extent_px, page_extent_px),
    };

    let mut pages = Vec::with_capacity(page_count);
    let mut snap_positions_x = vec![];
    let mut snap_positions_y = vec![];

    if page_count > 1 {
        for i in 0..page_count {
            let offset_px = page_extent_px * (i as f64);
            match axis {
                CarouselAxis::Horizontal => {
                    pages.push(CarouselCell {
                        x_px: offset_px,
                        y_px: 0.0,
                    });
                    snap_positions_x.push(Size::Pixels(Numeric::F64(offset_px)));
                }
                CarouselAxis::Vertical => {
                    pages.push(CarouselCell {
                        x_px: 0.0,
                        y_px: offset_px,
                    });
                    snap_positions_y.push(Size::Pixels(Numeric::F64(offset_px)));
                }
            }
        }
    } else if page_count == 1 {
        pages.push(CarouselCell {
            x_px: 0.0,
            y_px: 0.0,
        });
    }

    CarouselLayout {
        pages,
        page_width_px,
        page_height_px,
        scroll_width: Size::Pixels(Numeric::F64(scroll_width_px)),
        scroll_height: Size::Pixels(Numeric::F64(scroll_height_px)),
        snap_positions_x,
        snap_positions_y,
    }
}
