use pax::*;
use pax::api::{Size2D, Size, Property, Transform2D};
use pax::api::numeric::Numeric;
use pax_runtime_api::RuntimeContext;
use crate::primitives::{Frame};
use crate::types::{StackerDirection, StackerCell};

/// Stacker lays out a series of nodes either
/// vertically or horizontally (i.e. a single row or column) with a specified gutter in between
/// each node.  `Stacker`s can be stacked inside of each other, horizontally
/// and vertically, along with `Transform.align` and `Transform.anchor` to compose any rectilinear 2D layout.
#[derive(Pax)]
#[inlined(
    for (cell_spec, i) in 0..self._cell_specs {
        <Frame
            transform={Transform2D::translate(cell_spec.x, cell_spec.y)}
            width={cell_spec.width}
            height={cell_spec.height}
        >
            slot(i)
        </Frame>
    }

    @events {
        will_render: handle_will_render
    }

)]
pub struct Stacker {
    pub cells: Property<usize>,
    pub direction: Property<crate::types::StackerDirection>,
    pub _cell_specs: Property<Vec<StackerCell>>,
    pub gutter_width: Property<Size>,

    /// For for specifying sizes of each cell.  None-values (or array-index out-of-bounds values)
    /// will fall back to computed, equal-sizing
    pub sizes: Property<Vec<Option<Size>>>,
}

impl Stacker {
    pub fn handle_will_render(&mut self, ctx: RuntimeContext) {

        let cells = *self.cells.get() as f64;
        let bounds = ctx.bounds_parent;
        let active_bound = match *self.direction.get() {
            StackerDirection::Horizontal => bounds.0,
            StackerDirection::Vertical => bounds.1
        };

        let gutter_calc = match *self.gutter_width.get() {
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
//
// impl Stacker {
//
//     pub fn get_frame_transform(&self, index: usize, container: Size2D) -> Transform2D {
//         todo!()
//     }
//
//     pub fn get_frame_size(&self, index: usize, container: Size2D, direction: StackerDirection) -> Size2D {
//         todo!()
//     }
//
//     pub fn compute_dimensions(&mut self, ctx: RuntimeContext) {
//
//         //TODO: dirty check
//         let bounds = ctx.bounds_parent;
//
//         let active_bound = match *self.direction.get() {
//             StackerDirection::Horizontal => bounds.0,
//             StackerDirection::Vertical => bounds.1
//         };
//
//         let gutter_calc = match *self.gutter_width.get() {
//             Size::Pixels(px) => px,
//             Size::Percent(pct) => Numeric::from(active_bound)* (pct / Numeric::from(100.0)),
//         };
//
//         let cells = self.cells.get().len() as f64;
//
//         let usable_interior_space = active_bound - (cells - 1.0) * gutter_calc.get_as_float();
//         // let per_cell_space = usable_interior_space / cells;
//
//         //TODO: account for overrides
//         //The two data structures act as "sparse maps," where
//         //the first element in the tuple is the index of the cell/gutter to
//         //override and the second is the override value.  In the absence
//         //of overrides (`vec![]`), cells and gutters will divide space evenly.
//
//
//         //Manual dirty-check: intended to be supplanted by reactive dirty-check mechanism
//         //was needed to stop instance churn that was happening with
//         //
//         // let old = self.computed_layout_spec.get();
//         // let new : Vec<Rc<StackerCell>> = (0..(cells as usize)).into_iter().map(|i| {
//         //     match self.direction.get() {
//         //         StackerDirection::Horizontal =>
//         //             Rc::new(StackerCell {
//         //                 height_px: bounds.1,
//         //                 width_px: per_cell_space,
//         //                 x_px: ((i) as f64) * (gutter_calc) + (i as f64) * per_cell_space,
//         //                 y_px: 0.0,
//         //             }),
//         //         StackerDirection::Vertical =>
//         //             Rc::new(StackerCell {
//         //                 height_px: per_cell_space,
//         //                 width_px: bounds.0,
//         //                 x_px: 0.0,
//         //                 y_px: ((i) as f64) * (gutter_calc) + (i as f64) * per_cell_space,
//         //             }),
//         //     }
//         // }).collect();
//         // let is_dirty = old.len() != new.len() || old.iter().enumerate().any(|(i,p_old)|{
//         //     let p_new = new.get(i).unwrap();
//         //
//         //     p_old.height_px != p_new.height_px ||
//         //         p_old.width_px != p_new.width_px ||
//         //         p_old.x_px != p_new.x_px ||
//         //         p_old.y_px != p_new.y_px
//         // });
//         //
//         // if is_dirty {
//         //     self.computed_layout_spec.set(new);
//         // }
//
//     }
//
// }
