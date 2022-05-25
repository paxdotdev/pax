use std::rc::Rc;
use pax::*;
use pax::api::{Size2D, Size, ArgsRender, Property};
use crate::primitives::Frame;
use crate::types::{StackerDirection, StackerCellProperties};

/// Stacker lays out a series of nodes either
/// vertically or horizontally (i.e. a single row or column) with a specified gutter in between
/// each node.  Stackers can be stacked inside of each other, horizontally
/// and vertically, alongside `Transform.align` and `Transform.anchor` to achieve any 2D layout.
#[pax(
    for (elem, i) in self.computed_layout_spec {
        <Frame transform={translate(elem.x_px, elem.y_px)} size={[elem.width_px, elem.height_px]}>
            slot(i)
        </Frame>
    }
)]
pub struct Stacker {
    pub computed_layout_spec: Property<Vec<Rc<StackerCellProperties>>>,
    pub direction: Property<StackerDirection>,
    pub cell_count: Property<usize>,
    pub gutter_width: Property<Size>,

    pub overrides_cell_size: Property<Vec<(usize, Size)>>,
    pub overrides_gutter_size: Property<Vec<(usize, Size)>>,
}

impl Stacker {

    #[pax_on(pre_render)]
    pub fn handle_pre_render(&mut self, args: ArgsRender) {

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

        let cell_count = *self.cell_count.get() as f64;

        let usable_interior_space = active_bound - (cell_count - 1.0) * gutter_calc;
        let per_cell_space = usable_interior_space / cell_count;

        //TODO: account for overrides
        //The two data structures act as "sparse maps," where
        //the first element in the tuple is the index of the cell/gutter to
        //override and the second is the override value.  In the absence
        //of overrides (`vec![]`), cells and gutters will divide space evenly.


        //Manual dirty-check: intended to be supplanted by reactive dirty-check mechanism
        //was needed to stop instance churn that was happening with

        let old = self.computed_layout_spec.get();
        let new : Vec<Rc<StackerCellProperties>> = (0..(cell_count as usize)).into_iter().map(|i| {
            match self.direction.get() {
                StackerDirection::Horizontal =>
                    Rc::new(StackerCellProperties {
                        height_px: bounds.1,
                        width_px: per_cell_space,
                        x_px: ((i) as f64) * (gutter_calc) + (i as f64) * per_cell_space,
                        y_px: 0.0,
                    }),
                StackerDirection::Vertical =>
                    Rc::new(StackerCellProperties {
                        height_px: per_cell_space,
                        width_px: bounds.0,
                        x_px: 0.0,
                        y_px: ((i) as f64) * (gutter_calc) + (i as f64) * per_cell_space,
                    }),
            }
        }).collect();
        let is_dirty = old.len() != new.len() || old.iter().enumerate().any(|(i,p_old)|{
            let p_new = new.get(i).unwrap();

            p_old.height_px != p_new.height_px ||
                p_old.width_px != p_new.width_px ||
                p_old.x_px != p_new.x_px ||
                p_old.y_px != p_new.y_px
        });

        if is_dirty {
            self.computed_layout_spec.set(new);
        }


    }


}
