use pax_lang::*;
use pax_lang::api::{Size2D, Size, Property, Transform2D};
use pax_lang::api::numeric::Numeric;
use pax_runtime_api::RuntimeContext;
use crate::primitives::{Frame};
use crate::types::{StackerDirection, StackerCell};

/// Stacker lays out a series of nodes either
/// vertically or horizontally (i.e. a single row or column) with a specified gutter in between
/// each node.  `Stacker`s can be stacked inside of each other, horizontally
/// and vertically, along with `Transform.align` and `Transform.anchor` to compose any rectilinear 2D layout.
#[derive(Pax)]
#[inlined(
    for (cell_spec, i) in self._cell_specs {
        <Frame
            transform={Transform2D::translate(cell_spec.x_px, cell_spec.y_px)}
            width={(cell_spec.width_px)px}
            height={(cell_spec.height_px)px}
        >
            slot(i)
        </Frame>
    }

    @events {
        will_render: handle_will_render
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

impl Stacker {
    pub fn handle_will_render(&mut self, ctx: RuntimeContext) {

        let cells = self.cells.get().get_as_float();
        let bounds = ctx.bounds_parent;
        let active_bound = match *self.direction.get() {
            StackerDirection::Horizontal => bounds.0,
            StackerDirection::Vertical => bounds.1
        };

        let gutter_calc = match *self.gutter.get() {
             Size::Pixels(px) => px,
             Size::Percent(pct) => Numeric::from(active_bound)* (pct / Numeric::from(100.0)),
        };

        let usable_interior_space = active_bound - (cells - 1.0) * gutter_calc.get_as_float();
        let per_cell_space = usable_interior_space / cells;

        let new_cell_specs = (0..cells as usize).into_iter().map(|i|{
            match self.direction.get() {
                StackerDirection::Horizontal =>
                    StackerCell {
                        height_px: bounds.1,
                        width_px: per_cell_space,
                        x_px: ((i) as f64) * (gutter_calc) + (i as f64) * per_cell_space,
                        y_px: 0.0,
                    },
                StackerDirection::Vertical =>
                    StackerCell {
                        height_px: per_cell_space,
                        width_px: bounds.0,
                        x_px: 0.0,
                        y_px: ((i) as f64) * (gutter_calc) + (i as f64) * per_cell_space,
                    },
            }
        }).collect();

        self._cell_specs.set(new_cell_specs);
    }
}

