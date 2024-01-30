use crate::primitives::Frame;
use crate::types::{StackerCell, StackerDirection};
use pax_lang::api::numeric::Numeric;
use pax_lang::api::{Property, Size, Transform2D};
use pax_lang::*;
use pax_runtime_api::{NodeContext, PropertyLiteral};

/// Stacker lays out a series of nodes either
/// vertically or horizontally (i.e. a single row or column) with a specified gutter in between
/// each node.  `Stacker`s can be stacked inside of each other, horizontally
/// and vertically, along with `Transform.align` and `Transform.anchor` to compose any rectilinear 2D layout.
#[derive(Pax)]
#[custom(Default)]
#[inlined(
    for (cell_spec, i) in self._cell_specs {
        <Frame
            transform={Transform2D::translate((cell_spec.x_px)px, (cell_spec.y_px)px)}
            width={(cell_spec.width_px)px}
            height={(cell_spec.height_px)px}
        >
            slot(i)
        </Frame>
    }

    @handlers {
        tick: handle_tick
    }

)]
pub struct Stacker {
    pub cells: Property<Numeric>,
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
            cells: Box::new(PropertyLiteral::new(1.into())),
            direction: Box::new(PropertyLiteral::new(StackerDirection::Horizontal)),
            _cell_specs: Box::new(PropertyLiteral::new(vec![])),
            gutter: Box::new(PropertyLiteral::new(Size::Pixels(Numeric::Integer(0)))),
            sizes: Box::new(PropertyLiteral::new(vec![])),
        }
    }
}

impl Stacker {
    pub fn handle_tick(&mut self, ctx: &NodeContext) {
        let cells = self.cells.get().get_as_float();
        let bounds = ctx.bounds_self;

        let active_bound = match *self.direction.get() {
            StackerDirection::Horizontal => bounds.0,
            StackerDirection::Vertical => bounds.1,
        };

        let gutter_calc = match *self.gutter.get() {
            Size::Pixels(pix) => pix,
            Size::Percent(per) => Numeric::from(active_bound) * (per / Numeric::from(100.0)),
            Size::Combined(pix, per) => {
                pix + (Numeric::from(active_bound) * (per / Numeric::from(100.0)))
            }
        };

        let usable_interior_space = active_bound - (cells - 1.0) * gutter_calc.get_as_float();

        let per_cell_space = usable_interior_space / cells;

        let mut cell_space = vec![per_cell_space; self.cells.get().get_as_float() as usize];
        let sizes = self.sizes.get();

        if sizes.len() > 0 {
            if sizes.len() != (cells as usize) {
                unreachable!(
                    "Sizes is not a valid length. Please specify {} sizes",
                    (cells as usize)
                );
            }
            let mut used_space = 0.0;
            let mut remaining_cells = 0.0;
            for (i, size) in self.sizes.get().iter().enumerate() {
                if let Some(s) = size {
                    let space = match s {
                        Size::Pixels(pix) => *pix,
                        Size::Percent(per) => {
                            Numeric::from(active_bound) * (*per / Numeric::from(100.0))
                        }
                        Size::Combined(pix, per) => {
                            *pix + (Numeric::from(active_bound) * (*per / Numeric::from(100.0)))
                        }
                    }
                    .get_as_float();
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

            let remaining_per_cell_space = (usable_interior_space - used_space) / remaining_cells;
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
                let ret = match self.direction.get() {
                    StackerDirection::Horizontal => StackerCell {
                        height_px: bounds.1,
                        width_px: cell_space[i],
                        x_px: ((i) as f64) * (gutter_calc) + used_space,
                        y_px: 0.0,
                    },
                    StackerDirection::Vertical => StackerCell {
                        height_px: cell_space[i],
                        width_px: bounds.0,
                        x_px: 0.0,
                        y_px: ((i) as f64) * (gutter_calc) + used_space,
                    },
                };
                used_space += cell_space[i];
                ret
            })
            .collect();

        self._cell_specs.set(new_cell_specs);
    }
}
