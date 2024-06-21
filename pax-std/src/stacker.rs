use crate::primitives::*;
use crate::types::{StackerCell, StackerDirection};
use pax_engine::api::Numeric;
use pax_engine::api::{Property, Size, Transform2D};
use pax_engine::*;
use pax_runtime::api::NodeContext;

/// Stacker lays out a series of nodes either
/// vertically or horizontally (i.e. a single row or column) with a specified gutter in between
/// each node.  `Stacker`s can be stacked inside of each other, horizontally
/// and vertically, along with percentage-based positioning and `Transform2D.anchor` to compose any rectilinear 2D layout.
#[pax]
#[custom(Default)]
#[inlined(
    for (cell_spec, i) in self._cell_specs {

    }

    @settings {
        @mount: on_mount
    }

)]
pub struct Stacker {
    pub direction: Property<crate::types::StackerDirection>,
    pub _cell_specs: Property<Vec<StackerCell>>,
    pub gutter: Property<Size>,

    /// For for specifying sizes of each cell.  None-values (or array-index out-of-bounds values)
    /// will fall back to computed, equal-sizing
    pub sizes: Property<Vec<Option<Size>>>,
}

impl Default for Stacker {
    fn default() -> Self {
        Self {
            direction: Property::new(StackerDirection::Vertical, "Stacker::direction"),
            _cell_specs: Property::new(vec![], "Stacker::_cell_specs"),
            gutter: Property::new(Size::Pixels(Numeric::I32(0)), "Stacker::gutter"),
            sizes: Property::new(vec![], "Stacker::sizes"),
        }
    }
}

impl Stacker {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let sizes = self.sizes.clone();
        let transform_and_bounds_self = ctx.transform_and_bounds_self.clone();
        let slot_children_count = ctx.slot_children_count.clone();
        let gutter = self.gutter.clone();
        let direction = self.direction.clone();

        let deps = [
            transform_and_bounds_self.get_id(),
            direction.get_id(),
            sizes.get_id(),
            gutter.get_id(),
            slot_children_count.get_id(),
        ];

        //NOTE: replace with is needed since the for loop already has a connection to the prop
        self._cell_specs = Property::expression(
            move || {
                let cells: f64 = slot_children_count.get() as f64;
                let bounds = transform_and_bounds_self.get().bounds;

                let active_bound = match direction.get() {
                    StackerDirection::Horizontal => bounds.0,
                    StackerDirection::Vertical => bounds.1,
                };

                let gutter_calc = match gutter.get().clone() {
                    Size::Pixels(pix) => pix,
                    Size::Percent(per) => Numeric::F64(active_bound) * (per / Numeric::F64(100.0)),
                    Size::Combined(pix, per) => {
                        pix + (Numeric::F64(active_bound) * (per / Numeric::F64(100.0)))
                    }
                };

                let usable_interior_space = active_bound - (cells - 1.0) * gutter_calc.to_float();

                let per_cell_space = usable_interior_space / cells;

                let mut cell_space = vec![per_cell_space; cells as usize];
                let sizes = sizes.get();

                if sizes.len() > 0 {
                    if sizes.len() != (cells as usize) {
                        unreachable!(
                            "Sizes is not a valid length. Please specify {} sizes",
                            (cells as usize)
                        );
                    }
                    let mut used_space = 0.0;
                    let mut remaining_cells = 0.0;
                    for (i, size) in sizes.iter().enumerate() {
                        if let Some(s) = size {
                            let space = match s {
                                Size::Pixels(pix) => *pix,
                                Size::Percent(per) => {
                                    Numeric::F64(active_bound) * (*per / Numeric::F64(100.0))
                                }
                                Size::Combined(pix, per) => {
                                    *pix + (Numeric::F64(active_bound)
                                        * (*per / Numeric::F64(100.0)))
                                }
                            }
                            .to_float();
                            used_space += space;
                            cell_space[i] = space;
                        } else {
                            cell_space[i] = -1.0;
                            remaining_cells += 1.0;
                        }
                    }
                    if used_space > usable_interior_space {
                        unreachable!(
                            "Invalid sizes. Usable interior space is {}px",
                            usable_interior_space
                        );
                    }

                    let remaining_per_cell_space =
                        (usable_interior_space - used_space) / remaining_cells;
                    for i in &mut cell_space {
                        if *i == -1.0 {
                            *i = remaining_per_cell_space;
                        }
                    }
                }

                let mut used_space = 0.0;
                let new_cell_specs = (0..cells as usize)
                    .into_iter()
                    .map(|i| {
                        let ret = match direction.get() {
                            StackerDirection::Horizontal => StackerCell {
                                height_px: bounds.1,
                                width_px: cell_space[i],
                                x_px: ((i) as f64) * gutter_calc.to_float() + used_space,
                                y_px: 0.0,
                            },
                            StackerDirection::Vertical => StackerCell {
                                height_px: cell_space[i],
                                width_px: bounds.0,
                                x_px: 0.0,
                                y_px: ((i) as f64) * gutter_calc.to_float() + used_space,
                            },
                        };
                        used_space += cell_space[i];
                        ret
                    })
                    .collect();
                new_cell_specs
            },
            &deps, "Stacker::_cell_specs",
        );
    }
}
