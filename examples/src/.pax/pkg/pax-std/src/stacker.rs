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
        <Group
            transform={Transform2D::translate((cell_spec.x_px)px, (cell_spec.y_px)px)}
            width={(cell_spec.width_px)px}
            height={(cell_spec.height_px)px}
        >
            slot(i)
        </Group>
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
            direction: Property::new(StackerDirection::Vertical),
            _cell_specs: Property::new(vec![]),
            gutter: Property::new(Size::Pixels(Numeric::I32(0))),
            sizes: Property::new(vec![]),
        }
    }
}

impl Stacker {
    pub fn on_mount(&mut self, ctx: &NodeContext) {
        let sizes = self.sizes.clone();
        let bound = ctx.bounds_self.clone();
        let slot_children_count = ctx.slot_children_count.clone();
        let gutter = self.gutter.clone();
        let direction = self.direction.clone();

        let deps = [
            bound.untyped(),
            direction.untyped(),
            sizes.untyped(),
            gutter.untyped(),
            slot_children_count.untyped(),
        ];

        //NOTE: replace with is needed since the for loop already has a connection to the prop
        self._cell_specs.replace_with(Property::computed_with_name(
            move || {
                let cells: f64 = slot_children_count.get() as f64;
                let bounds = bound.get();

                let active_bound = match direction.get() {
                    StackerDirection::Horizontal => bounds.0,
                    StackerDirection::Vertical => bounds.1,
                };

                let gutter_calc = match gutter.get() {
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
            &deps,
            "stacker _cell_specs",
        ));
    }
}
