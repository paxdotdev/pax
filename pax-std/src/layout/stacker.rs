#[allow(unused)]
use crate::*;
use pax_engine::api::Numeric;
use pax_engine::api::{Axis, Property, Size};
use pax_engine::math::{Transform2, Vector2};
use pax_engine::*;
use pax_runtime::api::{borrow, NodeContext};
use pax_runtime::{ExpandedNode, LayoutHull};

/// Stacker lays out a series of nodes either
/// vertically or horizontally (i.e. a single row or column) with a specified gutter in between
/// each node.  `Stacker`s can be stacked inside of each other, horizontally
/// and vertically, along with percentage-based positioning and `Transform2D.anchor` to compose any rectilinear 2D layout.
#[pax]
#[engine_import_path("pax_engine")]
#[custom(Default)]
#[inlined(
    for i in 0..self._content_children_count {
        slot(i)
    }
    <Rectangle fill=TRANSPARENT _raycastable=false/>

    @settings {
        @mount: on_mount
        @pre_render: update
    }

)]
pub struct Stacker {
    /// The direction the stacker should flow its cells
    pub direction: Property<StackerDirection>,

    // Computed cell rectangles mirrored into content-child container frames.
    pub _cell_specs: Property<Vec<StackerCell>>,

    // Mirrored content child count for direct slot expansion in the inline template.
    pub _content_children_count: Property<usize>,

    /// Spacing between cells
    pub gutter: Property<Size>,

    /// When true, the stacker uses content-child bounds to autosize its cells
    /// and, when possible, its own bounds as well.
    ///
    /// `Stacker` interprets plain `autosize=true` as "autosize the extending
    /// axis only" (`y` for vertical stacks, `x` for horizontal stacks). Use
    /// `autosize_x` / `autosize_y` to override those per-axis defaults.
    ///
    /// The underlying shrink-sizing pass only applies when the measured axis
    /// can be resolved without parent-size cycles, so percent-sized children
    /// continue to use the existing top-down layout behavior unless the
    /// corresponding stacker axis is already explicit.
    pub autosize: Property<bool>,
    /// Optional override for whether autosize manages the `x` axis.
    pub autosize_x: Property<Option<bool>>,
    /// Optional override for whether autosize manages the `y` axis.
    pub autosize_y: Property<Option<bool>>,

    /// Size of each cell, by index.  None-values (or array-index out-of-bounds values)
    /// will fall back to computed, equal-sizing
    pub sizes: Property<Vec<Option<Size>>>,
}

impl Default for Stacker {
    fn default() -> Self {
        Self {
            direction: Property::new(StackerDirection::Vertical),
            _cell_specs: Property::new(vec![]),
            _content_children_count: Property::new(0),
            gutter: Property::new(Size::Pixels(Numeric::I32(0))),
            autosize: Property::new(false),
            autosize_x: Property::new(None),
            autosize_y: Property::new(None),
            sizes: Property::new(vec![]),
        }
    }
}

impl Stacker {
    // Seeds the content-child mirror and initial container frames.
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        self.bind_container(ctx);
        self.sync_layout(ctx);
    }

    // Recomputes content-child frames and optional autosize bounds each frame.
    pub fn update(&mut self, ctx: &NodeContext) {
        self.sync_layout(ctx);
    }

    fn sync_layout(&self, ctx: &NodeContext) {
        let Some(node) = ctx.expanded_node.upgrade() else {
            return;
        };

        let common_props = node.get_common_properties();
        let common_props = borrow!(common_props);
        let width_explicit = common_props.width.get().is_some();
        let height_explicit = common_props.height.get().is_some();
        drop(common_props);

        let content_children = ctx.content_children.get();
        let flow_children = content_children
            .iter()
            .filter(|child| !child.is_layout_breakout())
            .cloned()
            .collect::<Vec<_>>();
        let child_measurements = flow_children
            .iter()
            .map(|child| measure_child_bounds(&node, child))
            .collect::<Vec<_>>();

        let layout = compute_stacker_layout(
            self.direction.get(),
            ctx.bounds_self.get(),
            self.gutter.get(),
            self.sizes.get(),
            self.autosize.get(),
            self.autosize_x.get(),
            self.autosize_y.get(),
            &child_measurements,
            flow_children.len(),
            width_explicit,
            height_explicit,
        );

        let current_specs = self._cell_specs.get();
        if !cell_specs_equal(&current_specs, &layout.cell_specs) {
            self._cell_specs.set(layout.cell_specs.clone());
        }

        apply_container_frames(&flow_children, &layout.cell_specs);
        clear_breakout_container_frames(&content_children);

        match layout.measured_size {
            Some(measured_size) => {
                if node.measured_size.get() != Some(measured_size) {
                    node.set_measured_size(measured_size.0, measured_size.1);
                }
            }
            None => {
                if node.measured_size.get().is_some() {
                    node.measured_size.set(None);
                }
            }
        }
    }
}

impl Container for Stacker {
    // Mirrors normalized content child count so the inline template expands direct slots.
    fn bind_container(&self, ctx: &NodeContext) {
        let content_children_count = ctx.content_children_count.clone();
        let deps = [content_children_count.untyped()];
        self._content_children_count
            .replace_with(Property::computed_with_name(
                move || content_children_count.get(),
                &deps,
                "stacker content child count",
            ));
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct ChildMeasurement {
    width: Option<f64>,
    height: Option<f64>,
}

impl ChildMeasurement {
    fn axis(&self, axis: Axis) -> Option<f64> {
        match axis {
            Axis::X => self.width,
            Axis::Y => self.height,
        }
    }
}

struct StackerLayoutResult {
    cell_specs: Vec<StackerCell>,
    measured_size: Option<(f64, f64)>,
}

fn compute_stacker_layout(
    direction: StackerDirection,
    bounds: (f64, f64),
    gutter: Size,
    sizes: Vec<Option<Size>>,
    autosize: bool,
    autosize_x: Option<bool>,
    autosize_y: Option<bool>,
    child_measurements: &[ChildMeasurement],
    slot_children_count: usize,
    width_explicit: bool,
    height_explicit: bool,
) -> StackerLayoutResult {
    let standard_cell_specs =
        compute_standard_cell_specs(&direction, bounds, gutter, &sizes, slot_children_count);

    if !autosize {
        return StackerLayoutResult {
            cell_specs: standard_cell_specs,
            measured_size: None,
        };
    }

    let (manage_x, manage_y) =
        resolve_stacker_autosize_axes(&direction, autosize, autosize_x, autosize_y);
    let (main_axis, cross_axis, autosize_main_axis, autosize_cross_axis) = match &direction {
        StackerDirection::Horizontal => (
            Axis::X,
            Axis::Y,
            manage_x && !width_explicit,
            manage_y && !height_explicit,
        ),
        StackerDirection::Vertical => (
            Axis::Y,
            Axis::X,
            manage_y && !height_explicit,
            manage_x && !width_explicit,
        ),
    };

    let main_bound = axis_value(bounds, main_axis);
    let cross_bound = axis_value(bounds, cross_axis);

    if autosize_main_axis && size_depends_on_parent(gutter) {
        return StackerLayoutResult {
            cell_specs: standard_cell_specs,
            measured_size: None,
        };
    }

    let gutter_px = gutter.get_pixels(main_bound);

    let mut main_sizes = Vec::with_capacity(slot_children_count);
    let mut cross_sizes = Vec::with_capacity(slot_children_count);

    for index in 0..slot_children_count {
        let explicit_main = sizes
            .get(index)
            .copied()
            .flatten()
            .map(|size| {
                if autosize_main_axis && size_depends_on_parent(size) {
                    None
                } else {
                    Some(size.get_pixels(main_bound))
                }
            })
            .flatten();

        let main_size = if let Some(explicit_main) = explicit_main {
            explicit_main
        } else if autosize_main_axis {
            let Some(measured_main) = child_measurements
                .get(index)
                .and_then(|measurement| measurement.axis(main_axis))
            else {
                return StackerLayoutResult {
                    cell_specs: standard_cell_specs,
                    measured_size: None,
                };
            };
            measured_main
        } else {
            match direction {
                StackerDirection::Horizontal => standard_cell_specs[index].width_px,
                StackerDirection::Vertical => standard_cell_specs[index].height_px,
            }
        };

        let cross_size = if autosize_cross_axis {
            let Some(measured_cross) = child_measurements
                .get(index)
                .and_then(|measurement| measurement.axis(cross_axis))
            else {
                return StackerLayoutResult {
                    cell_specs: standard_cell_specs,
                    measured_size: None,
                };
            };
            measured_cross
        } else {
            cross_bound
        };

        if !autosize_cross_axis && !cross_bound.is_finite() {
            return StackerLayoutResult {
                cell_specs: standard_cell_specs,
                measured_size: None,
            };
        }

        main_sizes.push(main_size);
        cross_sizes.push(cross_size);
    }

    let content_main = if slot_children_count == 0 {
        0.0
    } else {
        main_sizes.iter().sum::<f64>() + gutter_px * (slot_children_count.saturating_sub(1) as f64)
    };
    let content_cross = cross_sizes.iter().copied().fold(0.0, f64::max);

    let mut cursor = 0.0;
    let cell_specs = (0..slot_children_count)
        .map(|index| {
            let main_size = main_sizes[index];
            let cross_size = cross_sizes[index];
            let cell = match &direction {
                StackerDirection::Horizontal => StackerCell {
                    width_px: main_size,
                    height_px: cross_size,
                    x_px: cursor,
                    y_px: 0.0,
                },
                StackerDirection::Vertical => StackerCell {
                    width_px: cross_size,
                    height_px: main_size,
                    x_px: 0.0,
                    y_px: cursor,
                },
            };
            cursor += main_size + gutter_px;
            cell
        })
        .collect();

    let measured_size = if autosize_main_axis || autosize_cross_axis {
        Some(match &direction {
            StackerDirection::Horizontal => (
                if autosize_main_axis {
                    content_main
                } else {
                    bounds.0
                },
                if autosize_cross_axis {
                    content_cross
                } else {
                    bounds.1
                },
            ),
            StackerDirection::Vertical => (
                if autosize_cross_axis {
                    content_cross
                } else {
                    bounds.0
                },
                if autosize_main_axis {
                    content_main
                } else {
                    bounds.1
                },
            ),
        })
    } else {
        None
    };

    StackerLayoutResult {
        cell_specs,
        measured_size,
    }
}

fn resolve_axis_autosize(
    autosize: bool,
    axis_override: Option<bool>,
    default_when_enabled: bool,
) -> bool {
    axis_override.unwrap_or(autosize && default_when_enabled)
}

fn resolve_stacker_autosize_axes(
    direction: &StackerDirection,
    autosize: bool,
    autosize_x: Option<bool>,
    autosize_y: Option<bool>,
) -> (bool, bool) {
    let (default_x, default_y) = match direction {
        StackerDirection::Horizontal => (true, false),
        StackerDirection::Vertical => (false, true),
    };
    (
        resolve_axis_autosize(autosize, autosize_x, default_x),
        resolve_axis_autosize(autosize, autosize_y, default_y),
    )
}

fn compute_standard_cell_specs(
    direction: &StackerDirection,
    bounds: (f64, f64),
    gutter: Size,
    sizes: &[Option<Size>],
    slot_children_count: usize,
) -> Vec<StackerCell> {
    if slot_children_count == 0 {
        return vec![];
    }

    let active_bound = axis_value(
        bounds,
        match direction {
            StackerDirection::Horizontal => Axis::X,
            StackerDirection::Vertical => Axis::Y,
        },
    );
    let gutter_px = gutter.get_pixels(active_bound);
    let usable_interior_space =
        active_bound - gutter_px * (slot_children_count.saturating_sub(1) as f64);
    let mut cell_space =
        vec![usable_interior_space / slot_children_count as f64; slot_children_count];

    let mut used_space = 0.0;
    let mut remaining_indices = Vec::new();
    for (index, size) in sizes.iter().take(slot_children_count).enumerate() {
        if let Some(size) = size {
            let space = size.get_pixels(active_bound);
            used_space += space;
            cell_space[index] = space;
        } else {
            remaining_indices.push(index);
        }
    }

    if !remaining_indices.is_empty() {
        let remaining_per_cell_space =
            ((usable_interior_space - used_space) / remaining_indices.len() as f64).max(5.0);
        for index in remaining_indices {
            cell_space[index] = remaining_per_cell_space;
        }
    }

    let mut cursor = 0.0;
    (0..slot_children_count)
        .map(|index| {
            let space = cell_space[index];
            let cell = match direction {
                StackerDirection::Horizontal => StackerCell {
                    width_px: space,
                    height_px: bounds.1,
                    x_px: cursor,
                    y_px: 0.0,
                },
                StackerDirection::Vertical => StackerCell {
                    width_px: bounds.0,
                    height_px: space,
                    x_px: 0.0,
                    y_px: cursor,
                },
            };
            cursor += space + gutter_px;
            cell
        })
        .collect()
}

fn measure_child_bounds(
    _parent: &std::rc::Rc<ExpandedNode>,
    child: &std::rc::Rc<ExpandedNode>,
) -> ChildMeasurement {
    child_measurement_from_hull(child.subtree_layout_hull.get())
}

fn child_measurement_from_hull(hull: LayoutHull) -> ChildMeasurement {
    ChildMeasurement {
        width: hull.forward_extent_x(),
        height: hull.forward_extent_y(),
    }
}

fn axis_value(bounds: (f64, f64), axis: Axis) -> f64 {
    match axis {
        Axis::X => bounds.0,
        Axis::Y => bounds.1,
    }
}

fn size_depends_on_parent(size: Size) -> bool {
    const EPSILON: f64 = f64::EPSILON;
    match size {
        Size::Pixels(_) => false,
        Size::Percent(percent) => percent.to_float().abs() > EPSILON,
        Size::Combined(_, percent) => percent.to_float().abs() > EPSILON,
    }
}

fn apply_container_frames(children: &[std::rc::Rc<ExpandedNode>], cell_specs: &[StackerCell]) {
    for (child, cell_spec) in children.iter().zip(cell_specs.iter()) {
        let frame = Some(ContainerFrame {
            transform: Transform2::translate(Vector2::new(cell_spec.x_px, cell_spec.y_px)),
            bounds: (cell_spec.width_px, cell_spec.height_px),
        });
        if child.container_frame.get() != frame {
            child.container_frame.set(frame);
        }
    }

    for child in children.iter().skip(cell_specs.len()) {
        if child.container_frame.get().is_some() {
            child.container_frame.set(None);
        }
    }
}

fn clear_breakout_container_frames(children: &[std::rc::Rc<ExpandedNode>]) {
    for child in children.iter().filter(|child| child.is_layout_breakout()) {
        if child.container_frame.get().is_some() {
            child.container_frame.set(None);
        }
    }
}

fn cell_specs_equal(current: &[StackerCell], next: &[StackerCell]) -> bool {
    current.len() == next.len()
        && current.iter().zip(next.iter()).all(|(current, next)| {
            current.x_px == next.x_px
                && current.y_px == next.y_px
                && current.width_px == next.width_px
                && current.height_px == next.height_px
        })
}

// Internal cell rectangle mirrored into container frames.
#[pax]
#[engine_import_path("pax_engine")]
pub struct StackerCell {
    // Cell x offset in pixels.
    pub x_px: f64,
    // Cell y offset in pixels.
    pub y_px: f64,
    // Cell width in pixels.
    pub width_px: f64,
    // Cell height in pixels.
    pub height_px: f64,
}

/// Flow direction for a `Stacker`.
#[pax]
#[engine_import_path("pax_engine")]
pub enum StackerDirection {
    /// Stack children top-to-bottom.
    #[default]
    Vertical,
    /// Stack children left-to-right.
    Horizontal,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_cell(cell: &StackerCell, x: f64, y: f64, width: f64, height: f64) {
        assert_eq!(cell.x_px, x);
        assert_eq!(cell.y_px, y);
        assert_eq!(cell.width_px, width);
        assert_eq!(cell.height_px, height);
    }

    #[test]
    fn autosize_vertical_uses_child_measurements() {
        let layout = compute_stacker_layout(
            StackerDirection::Vertical,
            (120.0, 200.0),
            Size::Pixels(8.into()),
            vec![],
            true,
            Some(true),
            None,
            &[
                ChildMeasurement {
                    width: Some(60.0),
                    height: Some(24.0),
                },
                ChildMeasurement {
                    width: Some(90.0),
                    height: Some(40.0),
                },
            ],
            2,
            false,
            false,
        );

        assert_eq!(layout.measured_size, Some((90.0, 72.0)));
        assert_eq!(layout.cell_specs.len(), 2);
        assert_cell(&layout.cell_specs[0], 0.0, 0.0, 60.0, 24.0);
        assert_cell(&layout.cell_specs[1], 0.0, 32.0, 90.0, 40.0);
    }

    #[test]
    fn autosize_falls_back_when_main_axis_cannot_be_measured() {
        let layout = compute_stacker_layout(
            StackerDirection::Vertical,
            (120.0, 200.0),
            Size::Pixels(8.into()),
            vec![],
            true,
            None,
            None,
            &[
                ChildMeasurement {
                    width: Some(60.0),
                    height: None,
                },
                ChildMeasurement {
                    width: Some(90.0),
                    height: Some(40.0),
                },
            ],
            2,
            false,
            false,
        );

        assert_eq!(layout.measured_size, None);
        assert_eq!(layout.cell_specs.len(), 2);
        assert_cell(&layout.cell_specs[0], 0.0, 0.0, 120.0, 96.0);
        assert_cell(&layout.cell_specs[1], 0.0, 104.0, 120.0, 96.0);
    }

    #[test]
    fn autosize_uses_explicit_cross_axis_when_child_widths_fill_parent() {
        let layout = compute_stacker_layout(
            StackerDirection::Vertical,
            (160.0, 240.0),
            Size::Pixels(10.into()),
            vec![],
            true,
            None,
            None,
            &[
                ChildMeasurement {
                    width: None,
                    height: Some(30.0),
                },
                ChildMeasurement {
                    width: None,
                    height: Some(50.0),
                },
            ],
            2,
            true,
            false,
        );

        assert_eq!(layout.measured_size, Some((160.0, 90.0)));
        assert_eq!(layout.cell_specs.len(), 2);
        assert_cell(&layout.cell_specs[0], 0.0, 0.0, 160.0, 30.0);
        assert_cell(&layout.cell_specs[1], 0.0, 40.0, 160.0, 50.0);
    }

    #[test]
    fn autosize_defaults_to_extending_axis_only() {
        let vertical = compute_stacker_layout(
            StackerDirection::Vertical,
            (160.0, 240.0),
            Size::Pixels(10.into()),
            vec![],
            true,
            None,
            None,
            &[
                ChildMeasurement {
                    width: None,
                    height: Some(30.0),
                },
                ChildMeasurement {
                    width: None,
                    height: Some(50.0),
                },
            ],
            2,
            false,
            false,
        );
        assert_eq!(vertical.measured_size, Some((160.0, 90.0)));

        let horizontal = compute_stacker_layout(
            StackerDirection::Horizontal,
            (160.0, 240.0),
            Size::Pixels(10.into()),
            vec![],
            true,
            None,
            None,
            &[
                ChildMeasurement {
                    width: Some(30.0),
                    height: None,
                },
                ChildMeasurement {
                    width: Some(50.0),
                    height: None,
                },
            ],
            2,
            false,
            false,
        );
        assert_eq!(horizontal.measured_size, Some((90.0, 240.0)));
    }

    #[test]
    fn axis_overrides_can_disable_default_or_enable_cross_axis() {
        let layout = compute_stacker_layout(
            StackerDirection::Vertical,
            (120.0, 200.0),
            Size::Pixels(8.into()),
            vec![],
            true,
            Some(true),
            Some(false),
            &[
                ChildMeasurement {
                    width: Some(60.0),
                    height: Some(24.0),
                },
                ChildMeasurement {
                    width: Some(90.0),
                    height: Some(40.0),
                },
            ],
            2,
            false,
            false,
        );

        assert_eq!(layout.measured_size, Some((90.0, 200.0)));
        assert_eq!(layout.cell_specs.len(), 2);
        assert_cell(&layout.cell_specs[0], 0.0, 0.0, 60.0, 96.0);
        assert_cell(&layout.cell_specs[1], 0.0, 104.0, 90.0, 96.0);
    }

    #[test]
    fn child_measurement_uses_local_hull_extent() {
        let measurement = child_measurement_from_hull(LayoutHull::from_axis_ranges(
            Some((0.0, 60.0)),
            Some((0.0, 72.0)),
        ));

        assert_eq!(measurement.width, Some(60.0));
        assert_eq!(measurement.height, Some(72.0));
    }
}
