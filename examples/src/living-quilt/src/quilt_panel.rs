use crate::quilt_tile::Panel;
use pax_kit::*;

#[pax]
#[file("quilt_panel.pax")]
pub struct QuiltPanel {
    pub panel: Property<Panel>,
    pub colorized: Property<bool>,
    pub x: Property<f64>,
    pub y: Property<f64>,
}

impl QuiltPanel {
    pub fn handle_pre_render(&mut self, ctx: &NodeContext) {
        self.advance(ctx.elapsed_time_millis().min(u64::MAX as u128) as u64);
    }

    pub fn advance(&self, now: u64) {
        let (x, y) = self.panel.read(|panel| panel.position(now));
        // Publish the final offset once, and leave settled panels quiet.
        self.x.set_if_neq(x);
        self.y.set_if_neq(y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};

    #[test]
    fn motion_updates_leave_shape_bindings_clean_and_settle_without_writes() {
        let mut panel = Panel::new(1, 12, 0, false);
        panel.direction = 1;
        let component = QuiltPanel {
            panel: Property::new(panel),
            ..Default::default()
        };
        // Variable is the same snapshot adapter used by PAXEL bindings.
        let shape = Variable::new_from_typed_property(component.panel.clone());
        let shape_reads = Rc::new(Cell::new(0));
        let reads = shape_reads.clone();
        let shape_dependency = shape.get_untyped_property().clone();
        let shape_binding = Property::computed(
            move || {
                reads.set(reads.get() + 1);
                shape.get_as_pax_value()
            },
            &[shape_dependency],
        );
        let motion_reads = Rc::new(Cell::new(0));
        let reads = motion_reads.clone();
        let x = component.x.clone();
        let x_dependency = x.untyped();
        let motion_binding = Property::computed(
            move || {
                reads.set(reads.get() + 1);
                x.get()
            },
            &[x_dependency],
        );
        for now in [0, 100, 200, 400, 580] {
            component.advance(now);
            shape_binding.get();
            motion_binding.get();
        }
        assert_eq!(shape_reads.get(), 1);
        assert_eq!(motion_reads.get(), 5);
        assert_eq!(component.x.get(), 0.0);
        for now in [600, 1000, 2000] {
            component.advance(now);
            motion_binding.get();
        }
        assert_eq!(motion_reads.get(), 5);
    }
}
