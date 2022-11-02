use std::rc::Rc;
use pax::*;
use pax::api::{Size2D, Size, ArgsRender, Property, Transform2D};
use crate::primitives::{Frame, Group};
use crate::types::{StackerDirection, StackerCellProperties};

/// Stacker lays out a series of nodes either
/// vertically or horizontally (i.e. a single row or column) with a specified gutter in between
/// each node.  `Stacker`s can be stacked inside of each other, horizontally
/// and vertically, along with `Transform.align` and `Transform.anchor` to compose any rectilinear 2D layout.
#[pax(
    for i in 0..self.cells {
        <Frame transform={self.get_frame_transform(i, $container)} size={(get_frame_size(i, $container))}>
            slot(i)
        </Frame>
    }
)]
pub struct Stacker {
    pub direction: Property<StackerDirection>,
    pub cells: Property<usize>,
    pub gutter_width: Property<Size>,

    pub overrides_cell_size: Property<Vec<(usize, Size)>>,
    pub overrides_gutter_size: Property<Vec<(usize, Size)>>,
}

impl Stacker {

    pub fn get_frame_transform(&self, index: usize, container: (Size2D, Size2D)) -> Transform2D {
        todo!()
    }

    pub fn get_frame_size(&self, index: usize, container: (Size2D, Size2D), direction: StackerDirection) -> Size2D {
        todo!()
    }

    #[pax_on(WillRender)]
    pub fn compute_dimensions(&mut self, args: ArgsRender) {

        //TODO: dirty check
        let bounds = args.bounds;

        let active_bound = match *self.direction.get() {
            StackerDirection::Horizontal => bounds.0,
            StackerDirection::Vertical => bounds.1
        };

        let gutter_calc = match *self.gutter_width.get() {
            Size::Pixels(px) => px,
            Size::Percent(pct) => active_bound * (pct / 100.0),
        };

        let cells = *self.cells.get() as f64;

        let usable_interior_space = active_bound - (cells - 1.0) * gutter_calc;
        let per_cell_space = usable_interior_space / cells;

        //TODO: account for overrides
        //The two data structures act as "sparse maps," where
        //the first element in the tuple is the index of the cell/gutter to
        //override and the second is the override value.  In the absence
        //of overrides (`vec![]`), cells and gutters will divide space evenly.


        //Manual dirty-check: intended to be supplanted by reactive dirty-check mechanism
        //was needed to stop instance churn that was happening with
        //
        // let old = self.computed_layout_spec.get();
        // let new : Vec<Rc<StackerCellProperties>> = (0..(cells as usize)).into_iter().map(|i| {
        //     match self.direction.get() {
        //         StackerDirection::Horizontal =>
        //             Rc::new(StackerCellProperties {
        //                 height_px: bounds.1,
        //                 width_px: per_cell_space,
        //                 x_px: ((i) as f64) * (gutter_calc) + (i as f64) * per_cell_space,
        //                 y_px: 0.0,
        //             }),
        //         StackerDirection::Vertical =>
        //             Rc::new(StackerCellProperties {
        //                 height_px: per_cell_space,
        //                 width_px: bounds.0,
        //                 x_px: 0.0,
        //                 y_px: ((i) as f64) * (gutter_calc) + (i as f64) * per_cell_space,
        //             }),
        //     }
        // }).collect();
        // let is_dirty = old.len() != new.len() || old.iter().enumerate().any(|(i,p_old)|{
        //     let p_new = new.get(i).unwrap();
        //
        //     p_old.height_px != p_new.height_px ||
        //         p_old.width_px != p_new.width_px ||
        //         p_old.x_px != p_new.x_px ||
        //         p_old.y_px != p_new.y_px
        // });
        //
        // if is_dirty {
        //     self.computed_layout_spec.set(new);
        // }

    }

}
